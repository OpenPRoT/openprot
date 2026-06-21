// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use std::time::Duration;
use zerocopy::FromBytes;

use opentitanlib::app::TransportWrapper;
use opentitanlib::io::usb::UsbDevice;
use opentitanlib::rescue::dfu::{DfuRequest, DfuRequestType, DfuState, DfuStatus};
use opentitanlib::io::console::ConsoleExt;
use opentitanlib::io::uart::Uart;
use opentitanlib::test_utils::init::InitializeTest;
use opentitanlib::uart::console::UartConsole;

#[derive(Parser, Debug)]
struct CmdArgs {
    #[command(flatten)]
    // Standard arguments for initializing the transport (Hyperdebug, etc.)
    init: InitializeTest,
}

const DFU_TIMEOUT: Duration = Duration::from_secs(10);

struct DfuClient<'a> {
    device: &'a dyn UsbDevice,
    interface: u8,
}

impl<'a> DfuClient<'a> {
    fn new(device: &'a dyn UsbDevice, interface: u8) -> Self {
        Self { device, interface }
    }

    fn write_control(&self, request: DfuRequest, value: u16, data: &[u8]) -> Result<usize> {
        self.device.write_control_timeout(
            DfuRequestType::Out.into(),
            request.into(),
            value,
            self.interface as u16,
            data,
            DFU_TIMEOUT,
        )
    }

    fn read_control(&self, request: DfuRequest, value: u16, data: &mut [u8]) -> Result<usize> {
        self.device.read_control_timeout(
            DfuRequestType::In.into(),
            request.into(),
            value,
            self.interface as u16,
            data,
            DFU_TIMEOUT,
        )
    }

    fn download(&self, block_num: u16, data: &[u8]) -> Result<usize> {
        self.write_control(DfuRequest::DnLoad, block_num, data)
    }

    fn upload(&self, block_num: u16, data: &mut [u8]) -> Result<usize> {
        self.read_control(DfuRequest::UpLoad, block_num, data)
    }

    fn get_status(&self) -> Result<DfuStatus> {
        let mut buf = [0u8; 6];
        self.read_control(DfuRequest::GetStatus, 0, &mut buf)?;
        DfuStatus::read_from_bytes(&buf).map_err(|e| anyhow!("Failed to parse DfuStatus: {:?}", e))
    }

    fn clear_status(&self) -> Result<()> {
        self.write_control(DfuRequest::ClrStatus, 0, &[])?;
        Ok(())
    }

    #[allow(dead_code)]
    fn abort(&self) -> Result<()> {
        self.write_control(DfuRequest::Abort, 0, &[])?;
        Ok(())
    }

    fn wait_state(&self, expected: DfuState, uart: &dyn Uart) -> Result<DfuStatus> {
        loop {
            print_uart(uart);
            let status = self.get_status()?;
            log::debug!("DFU State: {:?}, Status: {:?}", status.state(), status.status());
            if status.state() == expected {
                return Ok(status);
            }
            if status.state() == DfuState::Error {
                return Err(anyhow!("DFU entered Error state: {:?}", status.status()));
            }
            let delay = Duration::from_millis(status.poll_timeout() as u64);
            std::thread::sleep(delay);
        }
    }
}

fn print_uart(uart: &dyn Uart) {
    if !log::log_enabled!(log::Level::Info) {
        return;
    }
    let mut buf = [0u8; 256];
    while let Ok(n) = uart.read_timeout(&mut buf, Duration::ZERO) {
        if n == 0 {
            break;
        }
        use std::io::Write;
        let _ = std::io::stdout().write_all(&buf[..n]);
        let _ = std::io::stdout().flush();
    }
}

fn get_dfu_transfer_size(device: &dyn UsbDevice, interface_num: u8) -> Result<u16> {
    let config = device.active_configuration()?;
    for intf in config.interface_alt_settings() {
        let desc = intf.descriptor()?;
        if desc.intf_num == interface_num {
            for subdesc in intf.subdescriptors() {
                if subdesc.len() >= 7 && subdesc[1] == 0x21 {
                    let transfer_size = u16::from_le_bytes([subdesc[5], subdesc[6]]);
                    return Ok(transfer_size);
                }
            }
        }
    }
    bail!("DFU functional descriptor not found");
}

fn run_dfu_test(transport: &TransportWrapper, usb_vid: u16, usb_pid: u16) -> Result<()> {
    let uart = transport.uart("console")?;
    
    log::info!("Resetting target...");
    transport.reset(opentitanlib::app::UartRx::Clear)?;

    log::info!("waiting for RUNNING on console...");
    let _ = UartConsole::wait_for(&*uart, r"🔄 RUNNING", Duration::from_secs(10))?;

    log::info!("waiting for Serial Number on console...");
    let res = UartConsole::wait_for(&*uart, r"Serial Number: ([0-9a-fA-F]+)", Duration::from_secs(5))?;
    let serial_number = res[1].as_str();
    log::info!("Captured Serial Number: {}", serial_number);

    log::info!("waiting for DFU device (VID={:04x}, PID={:04x}, Serial={})...", usb_vid, usb_pid, serial_number);
    let device = transport.usb()?.device_by_id_with_timeout(
        usb_vid,
        usb_pid,
        Some(serial_number),
        Duration::from_secs(10),
    ).context("DFU device not found")?;

    log::info!("Claiming DFU interface...");
    let interface_num = 0;
    device.claim_interface(interface_num)?;

    let transfer_size = get_dfu_transfer_size(&*device, interface_num)?;
    log::info!("DFU Transfer Size (Block Size): {} bytes", transfer_size);

    let dfu = DfuClient::new(&*device, interface_num);

    // Ensure we start from a clean state
    let status = dfu.get_status()?;
    if status.state() == DfuState::Error {
        log::info!("Clearing DFU error status...");
        dfu.clear_status()?;
    }

    // 1. Download Test
    log::info!("Preparing 64KB test data...");
    let mut test_data = vec![0u8; 65536];
    for (i, byte) in test_data.iter_mut().enumerate() {
        *byte = (i & 0xFF) as u8;
    }

    log::info!("Starting DFU download...");
    let mut block_num = 0;
    for chunk in test_data.chunks(transfer_size as usize) {
        log::debug!("Sending block {}, size {}...", block_num, chunk.len());
        dfu.download(block_num, chunk)?;
        dfu.wait_state(DfuState::DnLoadIdle, &*uart)?;
        block_num += 1;
    }

    log::info!("Signaling end of download (ZLP)...");
    dfu.download(block_num, &[])?;
    
    log::info!("Waiting for manifestation to complete...");
    // ManifestationTolerant = 1 in firmware, so it should transition back to Idle
    dfu.wait_state(DfuState::Idle, &*uart)?;
    log::info!("Download complete!");
    print_uart(&*uart);

    // 2. Upload Test (Verify Data)
    log::info!("Starting DFU upload for verification...");
    let mut uploaded_data = Vec::new();
    let mut block_num = 0;
    let mut buf = vec![0u8; transfer_size as usize];
    loop {
        log::debug!("Reading block {}...", block_num);
        let n = dfu.upload(block_num, &mut buf)?;
        print_uart(&*uart);
        if n == 0 {
            break;
        }
        uploaded_data.extend_from_slice(&buf[..n]);
        if n < transfer_size as usize {
            break; // Short packet signals EOF
        }
        block_num += 1;
    }
    log::info!("Upload complete! Read {} bytes", uploaded_data.len());
    print_uart(&*uart);

    if uploaded_data != test_data {
        bail!("Uploaded data does not match downloaded data!");
    }
    log::info!("Data verification PASSED!");

    // 3. Certificate Upload Test (Graceful handling of empty certs)
    for alt in 1..=3 {
        log::info!("Testing Certificate Alt Setting {}...", alt);
        if let Err(e) = device.set_alternate_setting(interface_num, alt) {
            log::warn!("Failed to set alternate setting {}: {:?}", alt, e);
            continue;
        }
        
        let mut cert_buf = vec![0u8; transfer_size as usize];
        // The firmware get_certificate might return ErrUnknown (Stall) if blank.
        match dfu.upload(0, &mut cert_buf) {
            Ok(n) => {
                log::info!("Successfully read cert Alt {} ({} bytes)", alt, n);
                // We can't verify content easily, but we log it.
                if n > 0 {
                    log::info!("Cert Alt {} data (truncated): {:?}", alt, &cert_buf[..std::cmp::min(n, 16)]);
                } else {
                    log::warn!("Cert Alt {} returned 0 bytes", alt);
                }
            }
            Err(e) => {
                // Expected on FPGA where certs are blank
                log::info!("Cert Alt {} read failed (expected if blank): {:?}", alt, e);
                // Try to clear status in case we stalled
                let _ = dfu.clear_status();
            }
        }
    }

    // Switch back to Alt 0
    let _ = device.set_alternate_setting(interface_num, 0);

    device.release_interface(interface_num)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = CmdArgs::parse();
    args.init.init_logging();

    let usb_vid = args.init.backend_opts.usb_vid.unwrap_or(0x18d1);
    let usb_pid = args.init.backend_opts.usb_pid.unwrap_or(0x503a);

    let transport = args.init.init_target()?;
    run_dfu_test(&transport, usb_vid, usb_pid)?;
    Ok(())
}

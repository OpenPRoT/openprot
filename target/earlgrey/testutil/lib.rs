// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, Result};
use std::time::Duration;
use zerocopy::FromBytes;

use opentitanlib::io::console::ConsoleExt;
use opentitanlib::io::uart::Uart;
use opentitanlib::io::usb::UsbDevice;
use opentitanlib::rescue::dfu::{DfuRequest, DfuRequestType, DfuState, DfuStatus};

const DFU_TIMEOUT: Duration = Duration::from_secs(10);

pub struct DfuClient<'a> {
    pub device: &'a dyn UsbDevice,
    pub interface: u8,
}

impl<'a> DfuClient<'a> {
    pub fn new(device: &'a dyn UsbDevice, interface: u8) -> Self {
        Self { device, interface }
    }

    pub fn write_control(&self, request: DfuRequest, value: u16, data: &[u8]) -> Result<usize> {
        self.device.write_control_timeout(
            DfuRequestType::Out.into(),
            request.into(),
            value,
            self.interface as u16,
            data,
            DFU_TIMEOUT,
        )
    }

    pub fn read_control(&self, request: DfuRequest, value: u16, data: &mut [u8]) -> Result<usize> {
        self.device.read_control_timeout(
            DfuRequestType::In.into(),
            request.into(),
            value,
            self.interface as u16,
            data,
            DFU_TIMEOUT,
        )
    }

    pub fn download(&self, block_num: u16, data: &[u8]) -> Result<usize> {
        self.write_control(DfuRequest::DnLoad, block_num, data)
    }

    pub fn upload(&self, block_num: u16, data: &mut [u8]) -> Result<usize> {
        self.read_control(DfuRequest::UpLoad, block_num, data)
    }

    pub fn get_status(&self) -> Result<DfuStatus> {
        let mut buf = [0u8; 6];
        self.read_control(DfuRequest::GetStatus, 0, &mut buf)?;
        DfuStatus::read_from_bytes(&buf).map_err(|e| anyhow!("Failed to parse DfuStatus: {:?}", e))
    }

    pub fn clear_status(&self) -> Result<()> {
        self.write_control(DfuRequest::ClrStatus, 0, &[])?;
        Ok(())
    }

    pub fn abort(&self) -> Result<()> {
        self.write_control(DfuRequest::Abort, 0, &[])?;
        Ok(())
    }

    pub fn wait_state(&self, expected: DfuState, uart: &dyn Uart) -> Result<DfuStatus> {
        loop {
            print_uart(uart);
            let status = self.get_status()?;
            log::debug!(
                "DFU State: {:?}, Status: {:?}",
                status.state(),
                status.status()
            );
            if status.state() == expected {
                return Ok(status);
            }
            if status.state() == DfuState::Error {
                bail!("DFU entered Error state: {:?}", status.status());
            }
            let delay = Duration::from_millis(status.poll_timeout() as u64);
            std::thread::sleep(delay);
        }
    }
}

pub fn print_uart(uart: &dyn Uart) {
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

pub fn get_dfu_transfer_size(device: &dyn UsbDevice, interface_num: u8) -> Result<u16> {
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

pub fn sequence_dfu_download(
    dfu: &DfuClient,
    uart: &dyn Uart,
    data: &[u8],
    transfer_size: u16,
    expect_reboot: bool,
) -> Result<()> {
    // Ensure we start from a clean state
    let status = dfu.get_status()?;
    if status.state() == DfuState::Error {
        log::info!("Clearing DFU error status...");
        dfu.clear_status()?;
    }

    log::info!("Starting DFU download...");
    let mut block_num = 0;
    for chunk in data.chunks(transfer_size as usize) {
        log::debug!("Sending block {}, size {}...", block_num, chunk.len());
        dfu.download(block_num, chunk)?;
        dfu.wait_state(DfuState::DnLoadIdle, uart)?;
        block_num += 1;
    }

    log::info!("Signaling end of download (ZLP)...");
    dfu.download(block_num, &[])?;

    log::info!("Triggering Manifestation (GET_STATUS)...");
    let manifest_res = dfu.get_status();

    if expect_reboot {
        log::info!(
            "Waiting for manifestation-induced reboot (USB status OK: {})...",
            manifest_res.is_ok()
        );
        std::thread::sleep(Duration::from_secs(4));
        log::info!("Manifestation complete (Device Rebooted)!");
    } else {
        manifest_res?;
        log::info!("Waiting for manifestation to complete...");
        dfu.wait_state(DfuState::Idle, uart)?;
        log::info!("Download complete!");
    }
    // Removed print_uart(uart) to preserve telemetry for the test harness.
    Ok(())
}

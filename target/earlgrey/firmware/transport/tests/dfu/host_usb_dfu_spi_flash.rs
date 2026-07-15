// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::time::Duration;

use earlgrey_testutil::{
    get_dfu_transfer_size, print_uart, sequence_dfu_download, sequence_dfu_upload, DfuClient,
};
use opentitanlib::app::TransportWrapper;
use opentitanlib::io::uart::Uart;
use opentitanlib::test_utils::init::InitializeTest;
use opentitanlib::uart::console::UartConsole;
use usb::UsbOpts;

#[derive(Parser, Debug)]
struct CmdArgs {
    #[command(flatten)]
    init: InitializeTest,

    #[command(flatten)]
    usb: UsbOpts,

    #[arg(
        long,
        default_value = "target/earlgrey/firmware/transport/tests/dfu/bootinfo_simple.app_prod_0.signed.bin"
    )]
    firmware: String,
}

fn run_dfu_spi_flash_test_inner(
    transport: &TransportWrapper,
    usb: &UsbOpts,
    firmware_path: &str,
    uart: &dyn Uart,
) -> Result<()> {
    log::info!("Resetting target...");
    transport.reset(opentitanlib::app::UartRx::Clear)?;

    log::info!("waiting for Maize Welcome on console...");
    let _ = UartConsole::wait_for(
        uart,
        r"Welcome to Maize on Earlgrey Transport Firmware!",
        Duration::from_secs(10),
    )?;

    usb.apply_strappings(transport, true)?;
    if usb.vbus_control_available() {
        usb.enable_vbus(transport, true)?;
    }
    if usb.vbus_sense_available() {
        if !usb.vbus_present(transport)? {
            bail!("OT USB does not appear to be connected to a host (VBUS not detected)");
        }
    }

    let usb_vid = usb.vid;
    let usb_pid = usb.pid;

    log::info!(
        "waiting for DFU device (VID={:04x}, PID={:04x})...",
        usb_vid,
        usb_pid
    );
    let device = transport
        .usb()?
        .device_by_id_with_timeout(usb_vid, usb_pid, None, Duration::from_secs(10))
        .context("DFU device not found")?;

    log::info!("Claiming DFU interface...");
    let interface_num = 2;
    device.claim_interface(interface_num)?;

    let transfer_size = get_dfu_transfer_size(&*device, interface_num)?;
    log::info!("DFU Transfer Size (Block Size): {} bytes", transfer_size);

    // Set Alt setting 5 (SPI EEPROM 0)
    log::info!("Setting USB DFU Alt setting to 5 (SPI EEPROM 0)...");
    device.set_alternate_setting(interface_num, 5)?;

    let dfu = DfuClient::new(&*device, interface_num);

    log::info!("Reading payload from '{}'...", firmware_path);
    let test_data = std::fs::read(firmware_path)?;

    log::info!("Sequencing DFU Download (expect_reboot = false)...");
    sequence_dfu_download(&dfu, uart, &test_data, transfer_size, false)?;

    log::info!("Sequencing DFU Upload to read back payload...");
    let uploaded_data = sequence_dfu_upload(&dfu, test_data.len(), transfer_size)?;

    log::info!("Verifying integrity of uploaded data...");
    if uploaded_data != test_data {
        log::error!("Data mismatch!");
        log::error!(
            "Original len: {}, Uploaded len: {}",
            test_data.len(),
            uploaded_data.len()
        );
        log::error!(
            "Original (first 16 bytes): {:02x?}",
            &test_data[..std::cmp::min(16, test_data.len())]
        );
        log::error!(
            "Uploaded (first 16 bytes): {:02x?}",
            &uploaded_data[..std::cmp::min(16, uploaded_data.len())]
        );
        if let Some(mismatch_idx) = test_data
            .iter()
            .zip(uploaded_data.iter())
            .position(|(a, b)| a != b)
        {
            log::error!(
                "First mismatch at index {}: expected {:02x}, got {:02x}",
                mismatch_idx,
                test_data[mismatch_idx],
                uploaded_data[mismatch_idx]
            );
        } else {
            log::error!("No mismatch found within zipped range (vectors have different lengths).");
        }
        let _ = device.release_interface(interface_num);
        bail!("Data mismatch! Uploaded data does not match the downloaded payload.");
    }
    log::info!("✅ Integrity verification passed (hashes/bytes match)!");

    let _ = device.release_interface(interface_num);
    log::info!("Test Execution Finished Successfully!");
    Ok(())
}

fn run_dfu_spi_flash_test(
    transport: &TransportWrapper,
    usb: &UsbOpts,
    firmware_path: &str,
) -> Result<()> {
    let uart = transport.uart("console")?;
    let res = run_dfu_spi_flash_test_inner(transport, usb, firmware_path, &*uart);
    print_uart(&*uart);
    res
}

fn main() -> Result<()> {
    let args = CmdArgs::parse();
    args.init.init_logging();

    let transport = args.init.init_target()?;
    run_dfu_spi_flash_test(&transport, &args.usb, &args.firmware)?;
    Ok(())
}

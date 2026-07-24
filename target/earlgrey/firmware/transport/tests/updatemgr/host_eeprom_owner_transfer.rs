// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::time::Duration;

use earlgrey_testutil::{get_dfu_transfer_size, print_uart, sequence_dfu_download, DfuClient};
use opentitanlib::app::TransportWrapper;
use opentitanlib::image::image::{Image, ImageAssembler};
use opentitanlib::io::uart::Uart;
use opentitanlib::io::usb::UsbDevice;
use opentitanlib::test_utils::init::InitializeTest;
use opentitanlib::uart::console::UartConsole;
use opentitanlib::util::file::FromReader;
use usb::UsbOpts;

#[derive(Parser, Debug)]
struct CmdArgs {
    #[command(flatten)]
    init: InitializeTest,

    #[command(flatten)]
    usb: UsbOpts,

    #[arg(long)]
    rom_ext: String,

    #[arg(long)]
    firmware: String,

    #[arg(long)]
    transport_firmware: String,

    #[arg(long, default_value = "false")]
    expect_owner_transfer: bool,
}

fn setup_usb(transport: &TransportWrapper, usb: &UsbOpts) -> Result<()> {
    usb.apply_strappings(transport, true)?;
    if usb.vbus_control_available() {
        usb.enable_vbus(transport, true)?;
    }
    if usb.vbus_sense_available() {
        if !usb.vbus_present(transport)? {
            bail!("OT USB does not appear to be connected to a host (VBUS not detected)");
        }
    }
    Ok(())
}

fn connect_dfu_device(transport: &TransportWrapper, usb: &UsbOpts) -> Result<Box<dyn UsbDevice>> {
    log::info!(
        "Waiting for DFU device (VID={:04x}, PID={:04x})...",
        usb.vid,
        usb.pid
    );
    transport
        .usb()?
        .device_by_id_with_timeout(usb.vid, usb.pid, None, Duration::from_secs(10))
        .context("DFU device not found")
}

fn flash_eeprom_update_payload(
    device: &dyn UsbDevice,
    dfu: &DfuClient,
    uart: &dyn Uart,
    rom_ext_path: &str,
    firmware_path: &str,
    transfer_size: u16,
    interface_num: u8,
) -> Result<()> {
    log::info!("Setting USB DFU Alt setting to 5 (SPI EEPROM 0)...");
    device.set_alternate_setting(interface_num, 5)?;

    log::info!(
        "Assembling image: ROM Ext ('{}') @ 0, Firmware ('{}') @ 0x10000...",
        rom_ext_path,
        firmware_path
    );
    let mut image_assembler = ImageAssembler::with_params(0x100000, false);
    image_assembler.parse(&[
        format!("{}@0", rom_ext_path),
        format!("{}@0x10000", firmware_path),
    ])?;
    let mut test_data = image_assembler.assemble()?;

    if let Ok(image) = Image::from_reader(&test_data[..]) {
        if let Ok(subimages) = image.subimages() {
            if let Some(last_subimage) = subimages.last() {
                let actual_end = last_subimage.offset + last_subimage.manifest.length as usize;
                // Round up to next 2KiB (2048 bytes) alignment.
                let aligned_len = (actual_end + 2047) & !2047;
                if aligned_len < test_data.len() {
                    log::info!(
                        "Optimizing DFU payload size: found {} subimages. Truncating payload from {} bytes to {} bytes (actual payload end: 0x{:x}, 2KiB aligned)",
                        subimages.len(),
                        test_data.len(),
                        aligned_len,
                        actual_end
                    );
                    test_data.truncate(aligned_len);
                }
            }
        }
    }

    log::info!(
        "Sequencing DFU Download of assembled image ({} bytes) to EEPROM0...",
        test_data.len()
    );
    sequence_dfu_download(dfu, uart, &test_data, transfer_size, false)
}

fn flash_transport_firmware(
    device: &dyn UsbDevice,
    dfu: &DfuClient,
    uart: &dyn Uart,
    transport_firmware_path: &str,
    transfer_size: u16,
    interface_num: u8,
) -> Result<()> {
    log::info!("Setting USB DFU Alt setting to 0 (Internal Flash / DFU_ALT_FIRMWARE)...");
    device.set_alternate_setting(interface_num, 0)?;

    log::info!(
        "Reading transport_firmware payload from '{}'...",
        transport_firmware_path
    );
    let transport_fw_data = std::fs::read(transport_firmware_path)?;

    log::info!(
        "Sequencing DFU Download of transport_firmware ({} bytes) on Alt 0...",
        transport_fw_data.len()
    );
    sequence_dfu_download(dfu, uart, &transport_fw_data, transfer_size, true)
}

fn verify_telemetry(uart: &dyn Uart, expect_owner_transfer: bool) -> Result<()> {
    log::info!("Waiting for Application Execution telemetry on UART...");
    if expect_owner_transfer {
        let _ = UartConsole::wait_for(uart, r"ownership_transfers: 1", Duration::from_secs(20))
            .context("Failed to detect ownership_transfers: 1 in UART telemetry!")?;
        log::info!("✅ Detected ownership_transfers: 1");

        let _ = UartConsole::wait_for(uart, r"config_version: 1", Duration::from_secs(5))
            .context("Failed to detect config_version: 1 in UART telemetry!")?;
        log::info!("✅ Detected config_version: 1");

        let _ = UartConsole::wait_for(
            uart,
            r"update_mode: SELV \(0x564c4553\)",
            Duration::from_secs(5),
        )
        .context("Failed to detect update_mode: SELV in UART telemetry!")?;
        log::info!("✅ Detected update_mode: SELV (0x564c4553)");
    } else {
        let _ = UartConsole::wait_for(uart, r"ownership_transfers: 0", Duration::from_secs(20))
            .context("Failed to detect ownership_transfers: 0 in UART telemetry!")?;
        log::info!("✅ Detected ownership_transfers: 0");

        let _ = UartConsole::wait_for(uart, r"config_version: 1", Duration::from_secs(5))
            .context("Failed to detect config_version: 1 in UART telemetry!")?;
        log::info!("✅ Detected config_version: 1");

        let _ = UartConsole::wait_for(
            uart,
            r"update_mode: ANYV \(0x56594e41\)",
            Duration::from_secs(5),
        )
        .context("Failed to detect update_mode: ANYV in UART telemetry!")?;
        log::info!("✅ Detected update_mode: ANYV (0x56594e41)");
    }

    let _ = UartConsole::wait_for(uart, r"✅ PASSED bootinfo test", Duration::from_secs(5))
        .context("Failed to detect ✅ PASSED bootinfo test in UART telemetry!")?;
    log::info!("✅ Detected 'PASSED bootinfo test'!");

    print_uart(uart);
    Ok(())
}

fn run_dfu_eeprom_owner_transfer_test(
    transport: &TransportWrapper,
    usb: &UsbOpts,
    rom_ext_path: &str,
    firmware_path: &str,
    transport_firmware_path: &str,
    expect_owner_transfer: bool,
) -> Result<()> {
    let uart = transport.uart("console")?;

    log::info!("Resetting target running eeprom_programmer_firmware...");
    transport.reset(opentitanlib::app::UartRx::Clear)?;

    log::info!("Waiting for Maize Welcome on console...");
    let _ = UartConsole::wait_for(
        &*uart,
        r"Welcome to Maize on Earlgrey EEPROM Programmer Firmware!",
        Duration::from_secs(10),
    )?;

    setup_usb(transport, usb)?;

    let device = connect_dfu_device(transport, usb)?;
    let interface_num = 2;
    device.claim_interface(interface_num)?;

    let transfer_size = get_dfu_transfer_size(&*device, interface_num)?;
    let dfu = DfuClient::new(&*device, interface_num);

    flash_eeprom_update_payload(
        &*device,
        &dfu,
        &*uart,
        rom_ext_path,
        firmware_path,
        transfer_size,
        interface_num,
    )?;

    flash_transport_firmware(
        &*device,
        &dfu,
        &*uart,
        transport_firmware_path,
        transfer_size,
        interface_num,
    )?;

    let _ = device.release_interface(interface_num);
    log::info!("DFU download complete. Released DFU interface.");

    verify_telemetry(&*uart, expect_owner_transfer)?;

    log::info!("Test Execution Finished Successfully!");
    Ok(())
}

fn main() -> Result<()> {
    let args = CmdArgs::parse();
    args.init.init_logging();

    let transport = args.init.init_target()?;

    run_dfu_eeprom_owner_transfer_test(
        &transport,
        &args.usb,
        &args.rom_ext,
        &args.firmware,
        &args.transport_firmware,
        args.expect_owner_transfer,
    )?;
    Ok(())
}

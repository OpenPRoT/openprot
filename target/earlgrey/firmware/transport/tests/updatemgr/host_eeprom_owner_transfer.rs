// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, ensure, Context, Result};
use clap::Parser;
use std::path::Path;
use std::time::Duration;

use earlgrey_testutil::{get_dfu_transfer_size, print_uart, sequence_dfu_download, DfuClient};
use opentitanlib::app::TransportWrapper;
use opentitanlib::image::image::{Image, ImageAssembler, ImageChunk};
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
    transport_rom_ext: String,

    #[arg(long)]
    new_rom_ext: Option<String>,

    #[arg(long)]
    new_firmware: String,

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

fn get_firmware_assembly_offset(firmware_path: &str) -> Result<usize> {
    let owner_fw_image = Image::read_from_file(Path::new(firmware_path))
        .with_context(|| format!("Failed to read owner firmware image at '{firmware_path}'"))?;
    let image_bytes = owner_fw_image.bytes();
    ensure!(
        image_bytes.len() >= 816,
        "Owner firmware image at '{firmware_path}' is too small to contain a manifest header"
    );

    // Extract manifest_base_address directly from offset 812 (0x32c) of the owner firmware's manifest header
    // because:
    // 1) `opentitanlib::image::manifest::Manifest` (from external @lowrisc_opentitan) does not expose
    //    `manifest_base_address` as a public field on `&Manifest` in the pinned revision in MODULE.bazel.
    // 2) `earlgrey_util::manifest::Manifest` (from //target/earlgrey/util) exposes `pub manifest_base_address`,
    //    but parsing it via zerocopy traits fails due to a crate version mismatch between zerocopy v0.8.50
    //    (used by `earlgrey_util`) and v0.8.26 (imported via `@ot_crate_index`).
    // TODO: Parse via Manifest struct once (1) opentitanlib is updated or (2) zerocopy crate versions are unified.
    // Offset 812 is guaranteed by the Earlgrey/OpenTitan manifest binary specification.
    let manifest_base_addr = u32::from_le_bytes(image_bytes[812..816].try_into().unwrap());
    if manifest_base_addr == 0xa5a5a5a5 {
        Ok(0x10000)
    } else if manifest_base_addr >= 0x100000 {
        Ok((manifest_base_addr & 0xfffff) as usize)
    } else {
        Ok(manifest_base_addr as usize)
    }
}

fn flash_eeprom_update_payload(
    device: &dyn UsbDevice,
    dfu: &DfuClient,
    uart: &dyn Uart,
    new_rom_ext_path: Option<&str>,
    new_firmware_path: &str,
    transfer_size: u16,
    interface_num: u8,
) -> Result<()> {
    log::info!("Setting USB DFU Alt setting to 4 (SPI EEPROM 0)...");
    device.set_alternate_setting(interface_num, 4)?;

    let test_data = if let Some(rom_ext_path) = new_rom_ext_path {
        let firmware_offset = get_firmware_assembly_offset(new_firmware_path)?;
        log::info!(
            "Assembling image: ROM Ext ('{}') @ 0, Firmware ('{}') @ {:#x}...",
            rom_ext_path,
            new_firmware_path,
            firmware_offset
        );
        let mut image_assembler = ImageAssembler::with_params(0x100000, false);
        image_assembler.chunks.extend([
            ImageChunk::Offset(rom_ext_path.into(), 0),
            ImageChunk::Offset(new_firmware_path.into(), firmware_offset),
        ]);
        let mut data = image_assembler.assemble()?;

        if let Ok(image) = Image::from_reader(&data[..]) {
            if let Ok(subimages) = image.subimages() {
                if let Some(last_subimage) = subimages.last() {
                    let actual_end = last_subimage.offset + last_subimage.manifest.length as usize;
                    // Round up to next 2KiB (2048 bytes) alignment.
                    let aligned_len = (actual_end + 2047) & !2047;
                    if aligned_len < data.len() {
                        log::info!(
                            "Optimizing DFU payload size: found {} subimages. Truncating payload from {} bytes to {} bytes (actual payload end: 0x{:x}, 2KiB aligned)",
                            subimages.len(),
                            data.len(),
                            aligned_len,
                            actual_end
                        );
                        data.truncate(aligned_len);
                    }
                }
            }
        }
        data
    } else {
        log::info!(
            "Flashing standalone Application ('{}') directly to EEPROM0...",
            new_firmware_path
        );
        let fw_image = Image::read_from_file(Path::new(new_firmware_path)).with_context(|| {
            format!("Failed to read standalone firmware image at '{new_firmware_path}'")
        })?;
        fw_image.bytes().to_vec()
    };

    log::info!(
        "Sequencing DFU Download of payload ({} bytes) to EEPROM0...",
        test_data.len()
    );
    sequence_dfu_download(dfu, uart, &test_data, transfer_size, false)
}

fn flash_transport_firmware(
    transport: &TransportWrapper,
    bootstrap: &opentitanlib::bootstrap::BootstrapOptions,
    transport_rom_ext_path: &str,
    transport_firmware_path: &str,
) -> Result<()> {
    let firmware_offset = get_firmware_assembly_offset(transport_firmware_path)?;
    log::info!(
        "Assembling transport_firmware image: ROM Ext ('{}') @ 0, Firmware ('{}') @ {:#x}...",
        transport_rom_ext_path,
        transport_firmware_path,
        firmware_offset
    );
    let mut image_assembler = ImageAssembler::with_params(0x100000, true);
    image_assembler.chunks.extend([
        ImageChunk::Offset(transport_rom_ext_path.into(), 0),
        ImageChunk::Offset(transport_firmware_path.into(), firmware_offset),
    ]);
    let payload = image_assembler.assemble()?;

    log::info!("Bootstrapping transport_firmware back to device...");
    let progress = opentitanlib::app::StagedProgressBar::new();
    opentitanlib::bootstrap::Bootstrap::update_with_progress(
        transport, bootstrap, &payload, &progress,
    )?;
    Ok(())
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
    bootstrap: &opentitanlib::bootstrap::BootstrapOptions,
    transport_rom_ext_path: &str,
    new_rom_ext_path: Option<&str>,
    new_firmware_path: &str,
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
        new_rom_ext_path,
        new_firmware_path,
        transfer_size,
        interface_num,
    )?;

    let _ = device.release_interface(interface_num);
    log::info!("EEPROM DFU download complete. Released DFU interface.");

    flash_transport_firmware(
        transport,
        bootstrap,
        transport_rom_ext_path,
        transport_firmware_path,
    )?;

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
        &args.init.bootstrap.options,
        &args.transport_rom_ext,
        args.new_rom_ext.as_deref(),
        &args.new_firmware,
        &args.transport_firmware,
        args.expect_owner_transfer,
    )?;
    Ok(())
}

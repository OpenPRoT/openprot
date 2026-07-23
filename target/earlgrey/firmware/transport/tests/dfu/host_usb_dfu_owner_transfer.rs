// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, ensure, Context, Result};
use clap::Parser;
use std::path::Path;
use std::time::Duration;

use earlgrey_testutil::{get_dfu_transfer_size, print_uart, sequence_dfu_download, DfuClient};
use opentitanlib::app::TransportWrapper;
use opentitanlib::image::image::{Image, ImageAssembler, ImageChunk};
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

    #[arg(
        long,
        default_value = "target/earlgrey/firmware/transport/tests/dfu/bootinfo_transfer.app_prod_0.signed.bin"
    )]
    firmware: String,

    #[arg(long)]
    transport_firmware: String,

    #[arg(long, default_value = "false")]
    expect_reboot: bool,

    #[arg(long, default_value = "false")]
    expect_app: bool,

    #[arg(long, default_value = "false")]
    expect_owner_transfer: bool,
}

fn run_dfu_owner_transfer_test(
    transport: &TransportWrapper,
    usb: &UsbOpts,
    bootstrap: &opentitanlib::bootstrap::BootstrapOptions,
    rom_ext_path: &str,
    firmware_path: &str,
    transport_firmware_path: &str,
    expect_reboot: bool,
    expect_app: bool,
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
        "Waiting for DFU device (VID={:04x}, PID={:04x})...",
        usb_vid,
        usb_pid
    );
    let device = transport
        .usb()?
        .device_by_id_with_timeout(usb_vid, usb_pid, None, Duration::from_secs(10))
        .context("DFU device not found")?;

    log::info!("Claiming DFU interface to invalidate EEPROM0...");
    let interface_num = 2;
    device.claim_interface(interface_num)?;

    let transfer_size = get_dfu_transfer_size(&*device, interface_num)?;
    log::info!("DFU Transfer Size (Block Size): {} bytes", transfer_size);

    let dfu = DfuClient::new(&*device, interface_num);

    log::info!("Invalidating EEPROM0 with 132 KiB invalid payload on Alt 4...");
    device.set_alternate_setting(interface_num, 4)?;
    let invalid_data = vec![0x00u8; 132 * 1024];
    sequence_dfu_download(&dfu, &*uart, &invalid_data, transfer_size, false)?;

    let _ = device.release_interface(interface_num);

    let owner_fw_image =
        Image::read_from_file(Path::new(transport_firmware_path)).with_context(|| {
            format!("Failed to read owner firmware image at '{transport_firmware_path}'")
        })?;
    let image_bytes = owner_fw_image.bytes();
    ensure!(
        image_bytes.len() >= std::mem::size_of::<opentitanlib::image::manifest::Manifest>(),
        "Owner firmware image at '{transport_firmware_path}' is too small to contain a manifest header"
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
    let firmware_offset = if manifest_base_addr == 0xa5a5a5a5 {
        0x10000
    } else if manifest_base_addr >= 0x100000 {
        (manifest_base_addr & 0xfffff) as usize
    } else {
        manifest_base_addr as usize
    };

    log::info!(
        "Assembling transport_firmware image: ROM Ext ('{}') @ 0, Firmware ('{}') @ {:#x}...",
        rom_ext_path,
        transport_firmware_path,
        firmware_offset
    );
    let mut image_assembler = ImageAssembler::with_params(0x100000, true);
    image_assembler.chunks.extend([
        ImageChunk::Offset(rom_ext_path.into(), 0),
        ImageChunk::Offset(transport_firmware_path.into(), firmware_offset as usize),
    ]);
    let payload = image_assembler.assemble()?;

    log::info!("Bootstrapping transport_firmware back to device...");
    let progress = opentitanlib::app::StagedProgressBar::new();
    opentitanlib::bootstrap::Bootstrap::update_with_progress(
        transport, bootstrap, &payload, &progress,
    )?;

    log::info!("Waiting for Transport Firmware reboot on console...");
    let _ = UartConsole::wait_for(
        &*uart,
        r"Welcome to Maize on Earlgrey Transport Firmware!",
        Duration::from_secs(10),
    )?;

    log::info!("Connecting to DFU device running Transport Firmware...");
    let device = transport
        .usb()?
        .device_by_id_with_timeout(usb_vid, usb_pid, None, Duration::from_secs(10))
        .context("DFU device not found")?;
    device.claim_interface(interface_num)?;

    let transfer_size = get_dfu_transfer_size(&*device, interface_num)?;
    let dfu = DfuClient::new(&*device, interface_num);

    log::info!(
        "Reading Application firmware payload from '{}'...",
        firmware_path
    );
    let test_data = std::fs::read(firmware_path)?;

    log::info!(
        "Sequencing DFU Download (expect_reboot = {})...",
        expect_reboot
    );
    // Pass the actual expect_reboot variable to the test utility.
    sequence_dfu_download(&dfu, &*uart, &test_data, transfer_size, expect_reboot)?;

    if expect_reboot {
        if expect_app {
            log::info!("Waiting for Application Execution telemetry on UART...");
            if expect_owner_transfer {
                let _ = UartConsole::wait_for(
                    &*uart,
                    r"ownership_transfers: 1",
                    Duration::from_secs(20),
                )
                .context("Failed to detect ownership_transfers: 1 in UART telemetry!")?;
                log::info!("✅ Detected ownership_transfers: 1");

                let _ = UartConsole::wait_for(&*uart, r"config_version: 1", Duration::from_secs(5))
                    .context("Failed to detect config_version: 1 in UART telemetry!")?;
                log::info!("✅ Detected config_version: 1");

                let _ = UartConsole::wait_for(
                    &*uart,
                    r"update_mode: SELV \(0x564c4553\)",
                    Duration::from_secs(5),
                )
                .context("Failed to detect update_mode: SELV in UART telemetry!")?;
                log::info!("✅ Detected update_mode: SELV (0x564c4553)");
            } else {
                let _ = UartConsole::wait_for(
                    &*uart,
                    r"ownership_transfers: 0",
                    Duration::from_secs(20),
                )
                .context("Failed to detect ownership_transfers: 0 in UART telemetry!")?;
                log::info!("✅ Detected ownership_transfers: 0");

                let _ = UartConsole::wait_for(&*uart, r"config_version: 1", Duration::from_secs(5))
                    .context("Failed to detect config_version: 1 in UART telemetry!")?;
                log::info!("✅ Detected config_version: 1");

                let _ = UartConsole::wait_for(
                    &*uart,
                    r"update_mode: ANYV \(0x56594e41\)",
                    Duration::from_secs(5),
                )
                .context("Failed to detect update_mode: ANYV in UART telemetry!")?;
                log::info!("✅ Detected update_mode: ANYV (0x56594e41)");
            }

            let _ =
                UartConsole::wait_for(&*uart, r"✅ PASSED bootinfo test", Duration::from_secs(5))
                    .context("Failed to detect ✅ PASSED bootinfo test in UART telemetry!")?;
            log::info!("✅ Detected 'PASSED bootinfo test'!");
        } else {
            log::info!("Waiting for Transport firmware reboot (no ownership transfer)...");
            // Because no ownership transfer occurs, manifestation just reboots back into our Transport Firmware DFU server!
            let _ = UartConsole::wait_for(
                &*uart,
                r"Welcome to Maize on Earlgrey Transport Firmware!",
                Duration::from_secs(10),
            )?;
            log::info!("✅ Transport DFU Server rebooted successfully!");
        }
    }

    print_uart(&*uart);
    let _ = device.release_interface(interface_num);
    log::info!("Test Execution Finished Successfully!");
    Ok(())
}

fn main() -> Result<()> {
    let args = CmdArgs::parse();
    args.init.init_logging();

    let transport = args.init.init_target()?;
    run_dfu_owner_transfer_test(
        &transport,
        &args.usb,
        &args.init.bootstrap.options,
        &args.rom_ext,
        &args.firmware,
        &args.transport_firmware,
        args.expect_reboot,
        args.expect_app,
        args.expect_owner_transfer,
    )?;
    Ok(())
}

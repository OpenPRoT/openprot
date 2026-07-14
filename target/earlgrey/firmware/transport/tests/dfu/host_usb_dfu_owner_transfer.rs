// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::time::Duration;

use earlgrey_testutil::{get_dfu_transfer_size, print_uart, sequence_dfu_download, DfuClient};
use opentitanlib::app::TransportWrapper;
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
        default_value = "target/earlgrey/firmware/transport/tests/dfu/bootinfo_transfer.app_prod_0.signed.bin"
    )]
    firmware: String,

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
    firmware_path: &str,
    expect_reboot: bool,
    expect_app: bool,
    expect_owner_transfer: bool,
) -> Result<()> {
    let uart = transport.uart("console")?;

    log::info!("Resetting target...");
    transport.reset(opentitanlib::app::UartRx::Clear)?;

    log::info!("waiting for Maize Welcome on console...");
    let _ = UartConsole::wait_for(
        &*uart,
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
        &args.firmware,
        args.expect_reboot,
        args.expect_app,
        args.expect_owner_transfer,
    )?;
    Ok(())
}

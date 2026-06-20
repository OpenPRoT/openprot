// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, ensure, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use opentitanlib::io::console::ConsoleExt;
use opentitanlib::io::uart::Uart;
use opentitanlib::test_utils::init::InitializeTest;
use opentitanlib::transport::Capability;
use opentitanlib::uart::console::UartConsole;
use std::time::Instant;

use usb::{port_path_string, UsbDeviceHandle, UsbOpts};

#[derive(Debug, Parser)]
struct Opts {
    #[command(flatten)]
    init: InitializeTest,

    /// Console/USB timeout.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "60s")]
    timeout: Duration,

    /// USB options.
    #[command(flatten)]
    usb: UsbOpts,

    /// Wait for the USB device to appear before continuing with the test.
    #[arg(long, action = clap::ArgAction::Set, default_value = "true")]
    wait_for_usb_device: bool,

    /// Wait for the firmware to emit a PASS/FAIL result.
    #[arg(long, action = clap::ArgAction::Set, default_value = "true")]
    wait_for_firmware: bool,

    /// Executable to run after USB device connection.
    /// This harness will spawn a process to execute and continue monitoring the UART
    /// until the test passes (or fails). After that, the process will be killed.
    /// If `wait_for_usb_device` is true, the harness will pass two extra arguments
    /// to the executable to specify the bus and address of the USB device, as follows:
    /// `--device <bus>:<addr>`.
    #[arg(long)]
    exec: Option<PathBuf>,

    /// Arguments to pass to the executable.
    #[arg(long)]
    exec_arg: Vec<std::ffi::OsString>,
}

fn wait_for_device(opts: &Opts, uart: &dyn Uart) -> Result<UsbDeviceHandle> {
    let stop = Instant::now() + opts.timeout;
    log::info!("waiting for device (with UART log)...");
    while Instant::now() < stop {
        let mut devices = opts.usb.wait_for_device(Duration::ZERO)?;
        if !devices.is_empty() {
            if devices.len() > 1 {
                log::error!("several USB devices found:");
                for dev in &devices {
                    log::error!(
                        "- bus={} address={}",
                        dev.device().bus_number(),
                        dev.device().address()
                    );
                }
                bail!("several USB devices found");
            }
            let device = devices.remove(0);
            log::info!(
                "device found at bus={}, address={}, path={}",
                device.device().bus_number(),
                device.device().address(),
                port_path_string(&device.device())?
            );
            return Ok(device);
        }

        let mut buf = [0u8; 256];
        match uart.read_timeout(&mut buf, Duration::from_millis(10)) {
            Ok(n) if n > 0 => {
                use std::io::Write;
                std::io::stdout().write_all(&buf[..n])?;
                std::io::stdout().flush()?;
            }
            _ => {}
        }
    }
    bail!("no USB device found");
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    opts.init.init_logging();
    let transport = opts.init.init_target()?;

    transport
        .capabilities()?
        .request(Capability::USB)
        .ok()
        .context("This transport does not support USB")?;

    // Certain backends such as QEMU will not enumerate USB device until
    // we request the USB context.
    let _usb_context = transport.usb().context("Cannot get USB context")?;

    // Wait until test is running.
    let uart = transport.uart("console")?;
    log::info!("Resetting target...");
    transport.reset(opentitanlib::app::UartRx::Clear)?;
    log::info!("waiting for RUNNING on console...");
    if let Err(e) = UartConsole::wait_for(&*uart, r"RUNNING", Duration::from_secs(5)) {
        log::warn!("Failed waiting for RUNNING (non-fatal): {e}");
    }

    opts.usb.apply_strappings(&transport, true)?;
    // Enable VBUS sense on the board if necessary.
    if opts.usb.vbus_control_available() {
        opts.usb.enable_vbus(&transport, true)?;
    }
    // Sense VBUS if available.
    if opts.usb.vbus_sense_available() {
        ensure!(
            opts.usb.vbus_present(&transport)?,
            "OT USB does not appear to be connected to a host (VBUS not detected)"
        );
    }

    let mut captured_serial = None;
    if opts.wait_for_usb_device {
        log::info!("waiting for Serial Number on console...");
        let res = UartConsole::wait_for(&*uart, r"Serial Number: ([0-9a-fA-F]{64})", opts.timeout)?;
        captured_serial = Some(res[1].clone());
        log::info!(
            "Captured Serial Number: {}",
            captured_serial.as_ref().unwrap()
        );
    }

    // Wait for USB device to appear.
    let device = if opts.wait_for_usb_device {
        Some(wait_for_device(&opts, &*uart)?)
    } else {
        None
    };

    if let (Some(handle), Some(expected_serial)) = (&device, &captured_serial) {
        let device_desc = handle.device().device_descriptor()?;
        let usb_serial = handle.read_serial_number_string_ascii(&device_desc)?;
        log::info!("USB Serial Number: {usb_serial}");
        if usb_serial.as_str() != expected_serial.as_str() {
            bail!(
                "Serial number mismatch: expected (UART) {}, got (USB) {}",
                expected_serial,
                usb_serial
            );
        }
        log::info!("Serial number match: {usb_serial}");
    }

    // Run executable if requested.
    let child = match opts.exec {
        Some(exec) => {
            let mut cmd = Command::new(exec);
            if let Some(device) = device {
                cmd.arg("--device").arg(format!(
                    "{}:{}",
                    device.device().bus_number(),
                    device.device().address()
                ));
            }
            cmd.args(opts.exec_arg);
            log::info!(
                "calling {:?} on {:?}",
                cmd.get_program(),
                cmd.get_args().collect::<Vec<_>>()
            );
            Some(cmd.spawn().context("could not start executable")?)
        }
        None => None,
    };

    // Wait for test to pass.
    if opts.wait_for_firmware {
        log::info!("wait for pass...");
        let res = UartConsole::wait_for(&*uart, r"PASS|FAIL", opts.timeout)?;
        match res[0].as_str() {
            "PASS" => (),
            "FAIL" => bail!("device code reported a failure"),
            _ => (),
        };
    }

    // Kill executable (if running).
    if let Some(mut child) = child {
        match child.try_wait() {
            Ok(Some(status)) => log::info!("executable exited with: {status}"),
            Ok(None) => {
                log::info!("executable did not finish and will be killed");
                let _ = child.kill();
            }
            Err(e) => {
                println!("error attempting to get executable status: {e}");
                log::info!("killing executable");
                let _ = child.kill();
            }
        }
    }

    Ok(())
}

// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context, Result};
use clap::Parser;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

use opentitanlib::test_utils::init::InitializeTest;
use opentitanlib::uart::console::UartConsole;
use usb::UsbOpts;

#[derive(Parser, Debug)]
struct Opts {
    #[command(flatten)]
    init: InitializeTest,

    #[command(flatten)]
    usb: UsbOpts,

    #[arg(long, default_value = "Hello, OpenPRoT USB Serial!")]
    echo_string: String,
}

fn wait_for_usb_serial(
    expected_serial: &str,
    usb_vid: u16,
    usb_pid: u16,
    timeout: Duration,
) -> Result<serialport::SerialPortInfo> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        let ports = serialport::available_ports().context("Failed to list serial ports")?;
        for info in ports {
            if let serialport::SerialPortType::UsbPort(usb_info) = &info.port_type {
                if usb_info.vid == usb_vid && usb_info.pid == usb_pid {
                    if let Some(ref serial) = usb_info.serial_number {
                        if serial == expected_serial {
                            return Ok(info);
                        }
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    bail!("USB serial port not found within timeout");
}

fn run_echo_test(port_name: &str, test_data: &str) -> Result<()> {
    let mut port = serialport::new(port_name, 115_200)
        .timeout(Duration::from_secs(2))
        .open()
        .context("Failed to open serial port")?;

    log::info!("Sending test data: {:?}", test_data);
    port.write_all(test_data.as_bytes())
        .context("Failed to write to serial port")?;

    let mut buf = vec![0; test_data.len()];
    log::info!("Reading back data...");
    port.read_exact(&mut buf)
        .context("Failed to read from serial port")?;

    let received = String::from_utf8_lossy(&buf);
    log::info!("Received data: {:?}", received);

    if received != test_data {
        bail!("Echo data mismatch");
    }
    log::info!("Echo test passed!");
    Ok(())
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    opts.init.init_logging();

    let transport = opts.init.init_target()?;

    log::info!("Resetting target...");
    transport.reset(opentitanlib::app::UartRx::Clear)?;

    // Wait until test is running.
    let uart = transport.uart("console")?;
    log::info!("waiting for RUNNING on console...");
    UartConsole::wait_for(&*uart, r"RUNNING", Duration::from_secs(30))?;

    opts.usb.apply_strappings(&transport, true)?;
    // Enable VBUS sense on the board if necessary.
    if opts.usb.vbus_control_available() {
        opts.usb.enable_vbus(&transport, true)?;
    }
    // Sense VBUS if available.
    if opts.usb.vbus_sense_available() {
        if !opts.usb.vbus_present(&transport)? {
            bail!("OT USB does not appear to be connected to a host (VBUS not detected)");
        }
    }

    // Learn serial number
    log::info!("waiting for Serial Number on console...");
    let res = UartConsole::wait_for(
        &*uart,
        r"Serial Number: ([0-9a-fA-F]{64})",
        Duration::from_secs(10),
    )?;
    let serial_num = res[1].clone();
    log::info!("Captured Serial Number: {}", serial_num);

    log::info!("waiting for USB serial port with serial {}...", serial_num);
    let port_info = wait_for_usb_serial(
        &serial_num,
        opts.usb.vid,
        opts.usb.pid,
        Duration::from_secs(10),
    )?;
    log::info!("Found USB serial port: {}", port_info.port_name);

    run_echo_test(&port_info.port_name, &opts.echo_string)?;

    Ok(())
}

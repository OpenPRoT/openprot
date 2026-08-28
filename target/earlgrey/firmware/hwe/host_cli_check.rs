// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use clap::Parser;
use opentitanlib::test_utils::init::InitializeTest;
use opentitanlib::uart::console::UartConsole;
use usb::UsbOpts;

#[derive(Parser, Debug)]
struct Opts {
    #[command(flatten)]
    init: InitializeTest,

    #[command(flatten)]
    usb: UsbOpts,

    #[arg(long, default_value_t = 15)]
    timeout_secs: u64,
}

fn wait_for_usb_serial(
    usb_vid: u16,
    usb_pid: u16,
    timeout: Duration,
) -> Result<serialport::SerialPortInfo> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Ok(ports) = serialport::available_ports() {
            for info in ports {
                if let serialport::SerialPortType::UsbPort(usb_info) = &info.port_type {
                    if usb_info.vid == usb_vid && usb_info.pid == usb_pid {
                        return Ok(info);
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    bail!("USB CDC-ACM serial port not found within timeout");
}

fn test_uart_cli(transport: &opentitanlib::app::TransportWrapper, timeout: Duration) -> Result<()> {
    log::info!("Testing UART0 hardware console CLI...");
    let uart = transport.uart("console")?;

    log::info!("Sending 'help\\r' to UART0 console...");
    uart.write(b"help\r")
        .context("Failed to write 'help\\r' to UART0")?;

    UartConsole::wait_for(&*uart, r"Platform CLI Commands:", timeout)
        .context("Did not receive 'Platform CLI Commands:' on UART0")?;
    UartConsole::wait_for(&*uart, r"help  - Display this help message", timeout)
        .context("Did not receive help message description on UART0")?;

    log::info!("UART0 CLI help command verified successfully!");
    Ok(())
}

fn test_usb_cli(port_name: &str, timeout: Duration) -> Result<()> {
    log::info!("Testing USB CDC-ACM virtual serial CLI on {}...", port_name);
    let mut port = serialport::new(port_name, 115_200)
        .timeout(Duration::from_millis(500))
        .open()
        .context("Failed to open USB CDC-ACM serial port")?;

    log::info!("Sending 'help\\r' to USB CDC-ACM port...");
    port.write_all(b"help\r")
        .context("Failed to write 'help\\r' to USB CDC-ACM")?;
    port.flush().context("Failed to flush USB CDC-ACM port")?;

    let start = Instant::now();
    let mut output = String::new();
    let mut buf = [0u8; 256];

    while start.elapsed() < timeout {
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                output.push_str(&String::from_utf8_lossy(&buf[..n]));
                if output.contains("Platform CLI Commands:")
                    && output.contains("help  - Display this help message")
                {
                    log::info!("USB CDC-ACM CLI help command verified successfully!");
                    return Ok(());
                }
            }
            Ok(_) => std::thread::sleep(Duration::from_millis(50)),
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(e) => return Err(e).context("Error reading from USB CDC-ACM port"),
        }
    }

    bail!(
        "Timed out waiting for help response on USB CDC-ACM. Received output:\n{}",
        output
    );
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    opts.init.init_logging();

    let transport = opts.init.init_target()?;

    log::info!("Resetting target...");
    transport.reset(opentitanlib::app::UartRx::Clear)?;

    let uart = transport.uart("console")?;
    log::info!("Waiting for HWE boot and Running state on UART0 console...");
    UartConsole::wait_for(
        &*uart,
        r"Platform State: Running",
        Duration::from_secs(opts.timeout_secs),
    )?;
    log::info!("HWE boot confirmed in Running state!");

    // 1. Verify UART0 console CLI
    test_uart_cli(&transport, Duration::from_secs(opts.timeout_secs))?;

    // 2. Setup USB and verify USB CDC-ACM CLI
    opts.usb.apply_strappings(&transport, true)?;
    if opts.usb.vbus_control_available() {
        opts.usb.enable_vbus(&transport, true)?;
    }
    if opts.usb.vbus_sense_available() && !opts.usb.vbus_present(&transport)? {
        bail!("OT USB does not appear to be connected to host (VBUS not detected)");
    }

    log::info!(
        "Waiting for USB CDC-ACM port (VID: 0x{:04x}, PID: 0x{:04x})...",
        opts.usb.vid,
        opts.usb.pid
    );
    let port_info = wait_for_usb_serial(
        opts.usb.vid,
        opts.usb.pid,
        Duration::from_secs(opts.timeout_secs),
    )?;
    log::info!("Found USB CDC-ACM port: {}", port_info.port_name);

    test_usb_cli(&port_info.port_name, Duration::from_secs(opts.timeout_secs))?;

    log::info!("All CLI tests passed on both UART0 and USB CDC-ACM!");
    Ok(())
}

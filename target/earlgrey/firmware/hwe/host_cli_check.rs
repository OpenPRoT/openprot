// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use std::io::Write;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use clap::Parser;
use opentitanlib::io::console::ConsoleExt;
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

fn uart_send_and_wait(
    uart: &dyn opentitanlib::io::uart::Uart,
    cmd: &[u8],
    expected: &str,
    timeout: Duration,
) -> Result<String> {
    if !cmd.is_empty() {
        uart.write(cmd)?;
    }
    let start = Instant::now();
    let mut last_send = Instant::now();
    let mut output = String::new();
    let mut buf = [0u8; 256];

    while start.elapsed() < timeout {
        if !cmd.is_empty()
            && last_send.elapsed() >= Duration::from_secs(2)
            && (!output.contains(expected) || !output.contains("hwe>"))
        {
            let _ = uart.write(cmd);
            last_send = Instant::now();
        }
        let n = uart.read_timeout(&mut buf, Duration::from_millis(50))?;
        if n > 0 {
            let chunk = String::from_utf8_lossy(&buf[..n]);
            print!("{}", chunk);
            let _ = std::io::stdout().flush();
            output.push_str(&chunk);
            if output.contains(expected) && output.contains("hwe>") {
                return Ok(output);
            }
        }
    }
    bail!(
        "Timed out waiting for '{}' and prompt on UART0. Output received:\n{}",
        expected,
        output
    );
}

fn test_uart_cli(transport: &opentitanlib::app::TransportWrapper, timeout: Duration) -> Result<()> {
    log::info!("Testing UART0 hardware console CLI...");
    let uart = transport.uart("console")?;

    log::info!("Testing empty Enter liveness probe on UART0 console...");
    uart_send_and_wait(&*uart, b"\r", "hwe>", timeout).context("Failed liveness probe on UART0")?;

    log::info!("Sending 'help\\r' to UART0 console...");
    uart_send_and_wait(&*uart, b"help\r", "Platform CLI Commands:", timeout)
        .context("Failed 'help' on UART0")?;

    log::info!("Sending 'gpio help\\r' to UART0 console...");
    uart_send_and_wait(&*uart, b"gpio help\r", "GPIO Commands:", timeout)
        .context("Failed 'gpio help' on UART0")?;

    log::info!("Sending 'gpio list\\r' to UART0 console...");
    uart_send_and_wait(&*uart, b"gpio list\r", "GPIO Pin Status:", timeout)
        .context("Failed 'gpio list' on UART0")?;

    log::info!("Sending 'gpio read RST_CTRL0_N\\r' to UART0 console...");
    uart_send_and_wait(
        &*uart,
        b"gpio read RST_CTRL0_N\r",
        "GPIO RST_CTRL0_N: in=",
        timeout,
    )
    .context("Failed 'gpio read' on UART0")?;

    log::info!("Sending 'gpio write EXT_DEBUG_N 1\\r' to UART0 console...");
    uart_send_and_wait(
        &*uart,
        b"gpio write EXT_DEBUG_N 1\r",
        "GPIO EXT_DEBUG_N written 1 -> readback: out=1",
        timeout,
    )
    .context("Failed 'gpio write 1' on UART0")?;

    log::info!("Sending 'gpio write EXT_DEBUG_N 0\\r' to UART0 console...");
    uart_send_and_wait(
        &*uart,
        b"gpio write EXT_DEBUG_N 0\r",
        "GPIO EXT_DEBUG_N written 0 -> readback: out=0",
        timeout,
    )
    .context("Failed 'gpio write 0' on UART0")?;

    log::info!("Sending 'sys help\r' to UART0 console...");
    uart_send_and_wait(&*uart, b"sys help\r", "System Commands:", timeout)
        .context("Failed 'sys help' on UART0")?;

    log::info!("Sending 'sys info\r' to UART0 console...");
    uart_send_and_wait(&*uart, b"sys info\r", "System Information:", timeout)
        .context("Failed 'sys info' on UART0")?;

    log::info!("Sending 'sys id\r' to UART0 console...");
    uart_send_and_wait(&*uart, b"sys id\r", "Device ID:", timeout)
        .context("Failed 'sys id' on UART0")?;

    log::info!("Sending 'usb help\r' to UART0 console...");
    uart_send_and_wait(&*uart, b"usb help\r", "USB Commands:", timeout)
        .context("Failed 'usb help' on UART0")?;

    log::info!("Sending 'usb info\r' to UART0 console...");
    uart_send_and_wait(&*uart, b"usb info\r", "USB Status:", timeout)
        .context("Failed 'usb info' on UART0")?;

    log::info!("Sending 'usb mux device\r' to UART0 console...");
    uart_send_and_wait(
        &*uart,
        b"usb mux device\r",
        "USB multiplexer routed to device",
        timeout,
    )
    .context("Failed 'usb mux device' on UART0")?;

    log::info!("Sending 'flash help\r' to UART0 console...");
    uart_send_and_wait(&*uart, b"flash help\r", "Flash Commands:", timeout)
        .context("Failed 'flash help' on UART0")?;

    log::info!("Sending 'flash info\r' to UART0 console...");
    uart_send_and_wait(&*uart, b"flash info\r", "SPI Flash Status:", timeout)
        .context("Failed 'flash info' on UART0")?;

    log::info!("Sending 'flash mux en\r' to UART0 console...");
    uart_send_and_wait(
        &*uart,
        b"flash mux en\r",
        "SPI multiplexer enabled",
        timeout,
    )
    .context("Failed 'flash mux en' on UART0")?;

    log::info!("Sending 'flash route host\r' to UART0 console...");
    uart_send_and_wait(
        &*uart,
        b"flash route host\r",
        "SPI multiplexer routed to host",
        timeout,
    )
    .context("Failed 'flash route host' on UART0")?;

    log::info!("Sending 'flash read-id\r' to UART0 console (auto EEPROM 0)...");
    uart_send_and_wait(&*uart, b"flash read-id\r", "EEPROM 0 Status:", timeout)
        .context("Failed 'flash read-id' on UART0")?;

    log::info!("UART0 CLI commands verified successfully!");
    Ok(())
}

fn usb_send_and_wait(
    port: &mut dyn serialport::SerialPort,
    cmd: &[u8],
    expected: &str,
    timeout: Duration,
) -> Result<String> {
    if !cmd.is_empty() {
        port.write_all(cmd)?;
        port.flush()?;
    }
    let start = Instant::now();
    let mut last_send = Instant::now();
    let mut output = String::new();
    let mut buf = [0u8; 256];

    while start.elapsed() < timeout {
        if !cmd.is_empty()
            && last_send.elapsed() >= Duration::from_secs(2)
            && (!output.contains(expected) || !output.contains("hwe>"))
        {
            let _ = port.write_all(cmd);
            let _ = port.flush();
            last_send = Instant::now();
        }
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                let chunk = String::from_utf8_lossy(&buf[..n]);
                print!("{}", chunk);
                let _ = std::io::stdout().flush();
                output.push_str(&chunk);
                if output.contains(expected) && output.contains("hwe>") {
                    return Ok(output);
                }
            }
            Ok(_) => std::thread::sleep(Duration::from_millis(50)),
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(e) => return Err(e).context("Error reading from USB CDC-ACM port"),
        }
    }
    bail!(
        "Timed out waiting for expected string '{}'. Received output:\n{}",
        expected,
        output
    )
}

fn test_usb_cli(port_name: &str, timeout: Duration) -> Result<()> {
    log::info!("Testing USB CDC-ACM virtual serial CLI on {}...", port_name);
    let mut port = serialport::new(port_name, 115200)
        .timeout(Duration::from_millis(500))
        .open()
        .context("Failed to open USB CDC-ACM serial port")?;

    std::thread::sleep(Duration::from_millis(100));

    // Drain old backlog of logs generated during UART0 testing
    port.set_timeout(Duration::from_millis(100))?;
    let drain_start = Instant::now();
    let mut drain_buf = [0u8; 256];
    while drain_start.elapsed() < Duration::from_millis(800) {
        match port.read(&mut drain_buf) {
            Ok(n) if n > 0 => continue,
            _ => break,
        }
    }
    port.set_timeout(Duration::from_millis(500))?;

    log::info!("Sending 'gpio help\r' to USB CDC-ACM port...");
    usb_send_and_wait(&mut *port, b"gpio help\r", "GPIO Commands:", timeout)
        .context("Failed 'gpio help' on USB CDC-ACM")?;

    log::info!("Sending 'sys info\r' to USB CDC-ACM port...");
    usb_send_and_wait(&mut *port, b"sys info\r", "System Information:", timeout)
        .context("Failed 'sys info' on USB CDC-ACM")?;

    log::info!("Sending 'usb info\r' to USB CDC-ACM port...");
    usb_send_and_wait(&mut *port, b"usb info\r", "USB Status:", timeout)
        .context("Failed 'usb info' on USB CDC-ACM")?;

    log::info!("Sending 'flash info\r' to USB CDC-ACM port...");
    usb_send_and_wait(&mut *port, b"flash info\r", "SPI Flash Status:", timeout)
        .context("Failed 'flash info' on USB CDC-ACM")?;

    log::info!("Sending 'flash read-id 0\r' to USB CDC-ACM port...");
    usb_send_and_wait(
        &mut *port,
        b"flash read-id 0\r",
        "EEPROM 0 Status:",
        timeout,
    )
    .context("Failed 'flash read-id 0' on USB CDC-ACM")?;

    log::info!("USB CDC-ACM CLI commands verified successfully!");
    Ok(())
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    opts.init.init_logging();

    let transport = opts.init.init_target()?;

    log::info!("Resetting target...");
    transport.reset(opentitanlib::app::UartRx::Clear)?;

    let uart = transport.uart("console")?;
    log::info!("Waiting for HWE boot and Running state on UART0 console...");
    if let Err(e) =
        UartConsole::wait_for(&*uart, r"Platform State: Running", Duration::from_secs(5))
    {
        log::warn!("Waiting for Running state timed out (non-fatal): {e}");
    } else {
        log::info!("HWE boot confirmed in Running state!");
    }

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

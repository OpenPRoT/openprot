// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use pw_status::Error;
use userspace::{process_entry, syscall};
use util_error::{AsStatus, ErrorCode};
use util_zfmt::messages::{ProcessExit, ProcessStart};

fn platform_server() -> Result<(), ErrorCode> {
    use earlgrey_gpio::EarlGreyGpio;
    use earlgrey_pinout::dualsbs::DualSideBySide;
    use earlgrey_pinout::swstraps::SwStraps;
    use earlgrey_pinout::Pinout;
    use earlgrey_platform::reset::{ResetPolicy, TargetCpuReset};
    use earlgrey_platform::server::PlatformServer;
    use earlgrey_platform::spimux::SpiMuxHandler;
    use earlgrey_platform::usbmux::UsbMuxHandler;
    use earlgrey_sysmgr_client::{ResetInfo, SysmgrClient};
    use platform_codegen::{handle, signals};
    use userspace::syscall::Signals;
    #[cfg(feature = "cli")]
    use userspace::time::Instant;
    #[cfg(not(feature = "cli"))]
    use userspace::time::{Clock, Duration, SystemClock};
    #[cfg(feature = "cli")]
    use util_ipc::IpcChannel;
    use util_ipc::IpcHandle;

    // SAFETY: the platform process has exclusive access to the GPIO & Pinmux peripherals.
    let mut gpio = unsafe { EarlGreyGpio::new() };
    let sysmgr = SysmgrClient::new(IpcHandle::new(handle::SYSMGR_PLATFORM));

    // 1. Unconditionally configure SwStraps pinmux.
    SwStraps::configure(&mut gpio)?;

    // 2. Read software straps.
    let straps = SwStraps::read_straps(&mut gpio)?;
    util_zfmt::debug!("SW_STRAPs read: {straps:02x}", straps = straps);

    // 3. Send the strap value to sysmgr.
    sysmgr.set_software_straps(straps)?;

    // 4. Retrieve BootInfo from sysmgr.
    let boot_info = sysmgr.get_boot_info()?;
    let is_low_power = (boot_info.reset.reason & ResetInfo::REASON_LOW_POWER_EXIT) != 0;

    // 5. Examine straps, configure board pinmux if power-on-reset, and create handlers.
    let (usb_mux, spi_mux, reset_policy, usb_sig, rst0_sig, rst1_sig) = match straps {
        SwStraps::TEACUP_BOARD | SwStraps::BRINGUP_STRAPS1 | SwStraps::BRINGUP_STRAPS2 => {
            if !is_low_power {
                DualSideBySide::configure(&mut gpio)?;
            }
            (
                UsbMuxHandler::new(DualSideBySide::USB_PRESENCE_N, DualSideBySide::USB_MUX_CTRL),
                SpiMuxHandler::new(
                    DualSideBySide::SPI_MUX_EN_N,
                    DualSideBySide::SPI_MUX_CTRL,
                    DualSideBySide::SPI_RESET_N,
                    DualSideBySide::SPI_HOST0_WP_N,
                    DualSideBySide::SPI_HOST1_WP_N,
                ),
                ResetPolicy::TargetCpu(TargetCpuReset::new(
                    DualSideBySide::RST_CTRL0_N,
                    DualSideBySide::RST_MON0_N,
                    DualSideBySide::RST_MON1_N,
                )),
                signals::GPIO_16,
                signals::GPIO_17,
                signals::GPIO_18,
            )
        }
        _ => {
            // Fall back to DualSideBySide for undefined strapping values.
            if !is_low_power {
                DualSideBySide::configure(&mut gpio)?;
            }
            (
                UsbMuxHandler::new(DualSideBySide::USB_PRESENCE_N, DualSideBySide::USB_MUX_CTRL),
                SpiMuxHandler::new(
                    DualSideBySide::SPI_MUX_EN_N,
                    DualSideBySide::SPI_MUX_CTRL,
                    DualSideBySide::SPI_RESET_N,
                    DualSideBySide::SPI_HOST0_WP_N,
                    DualSideBySide::SPI_HOST1_WP_N,
                ),
                ResetPolicy::TargetCpu(TargetCpuReset::new(
                    DualSideBySide::RST_CTRL0_N,
                    DualSideBySide::RST_MON0_N,
                    DualSideBySide::RST_MON1_N,
                )),
                signals::GPIO_16,
                signals::GPIO_17,
                signals::GPIO_18,
            )
        }
    };

    #[cfg(feature = "cli")]
    use earlgrey_platform::cli::CliDispatcher;

    let mut server = PlatformServer::new(gpio, usb_mux, spi_mux, reset_policy);
    #[cfg(not(feature = "cli"))]
    server.set_exit_deadline(SystemClock::now() + Duration::from_secs(10));
    #[cfg(feature = "cli")]
    server.set_exit_deadline(Instant::MAX);
    server.start(is_low_power)?;

    #[cfg(feature = "cli")]
    let mut cli_dispatcher = CliDispatcher::new();

    #[cfg(feature = "cli")]
    syscall::wait_group_add(
        handle::PLATFORM_WAIT_GROUP,
        handle::CLI_PLATFORM,
        Signals::USER,
        handle::CLI_PLATFORM as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    syscall::wait_group_add(
        handle::PLATFORM_WAIT_GROUP,
        handle::PLATFORM_INTERRUPTS,
        usb_sig | rst0_sig | rst1_sig,
        handle::PLATFORM_INTERRUPTS as usize,
    )
    .map_err(ErrorCode::kernel_error)?;

    #[cfg(feature = "cli")]
    let mut cmd_buf = [0u8; 128];

    #[cfg(feature = "cli")]
    util_zfmt::raw!("hwe> ");

    loop {
        if server.should_exit() {
            return Ok(());
        }
        let deadline = server.next_deadline();
        let wait_res =
            syscall::object_wait(handle::PLATFORM_WAIT_GROUP, Signals::READABLE, deadline);

        match wait_res {
            Ok(wait_return) => {
                let active = wait_return.user_data as u32;
                if active == handle::PLATFORM_INTERRUPTS {
                    let signals = wait_return.pending_signals;

                    if (signals & usb_sig) != Signals::empty() {
                        server.handle_usb_presence_interrupt()?;
                    }
                    if (signals & rst0_sig) != Signals::empty() {
                        server.handle_rst_mon_interrupt(0)?;
                    }
                    if (signals & rst1_sig) != Signals::empty() {
                        server.handle_rst_mon_interrupt(1)?;
                    }

                    syscall::interrupt_ack(handle::PLATFORM_INTERRUPTS, signals)
                        .map_err(ErrorCode::kernel_error)?;
                    continue;
                }

                #[cfg(feature = "cli")]
                if active == handle::CLI_PLATFORM {
                    let cli_platform = IpcHandle::new(handle::CLI_PLATFORM);
                    let n = cli_platform
                        .transact(&[0u8; 0], &mut cmd_buf, Instant::MAX)
                        .map_err(ErrorCode::kernel_error)?;
                    if let Ok(cmd_str) = core::str::from_utf8(&cmd_buf[..n]) {
                        util_zfmt::debug!("[cli] {cmd}", cmd = cmd_str);
                        let mut context = server.cli_context(&sysmgr, straps);
                        cli_dispatcher.dispatch(cmd_str, &mut context);
                        util_zfmt::raw!("hwe> ");
                        let _ = cli_platform.transact(b"DONE", &mut cmd_buf, Instant::MAX);
                    }
                    continue;
                }
            }
            Err(Error::DeadlineExceeded) => {
                if server.should_exit() {
                    return Ok(());
                }
                server.handle_timeout()?;
            }
            Err(e) => {
                return Err(ErrorCode::kernel_error(e));
            }
        }
    }
}

#[process_entry("platform")]
fn entry() -> Result<(), Error> {
    util_zfmt::info!(ProcessStart { name: "platform" });
    let ret = platform_server();
    util_zfmt::error!(ProcessExit {
        name: "platform",
        status: ret.as_status()
    });

    let status_res = ret.map_err(|_| Error::Unknown);
    syscall::debug_shutdown(status_res)
}

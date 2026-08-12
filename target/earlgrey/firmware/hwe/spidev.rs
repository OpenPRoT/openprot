// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPI Device service process for Earlgrey HWE firmware.

#![no_std]
#![no_main]

use aligned::{Aligned, A4};
use earlgrey_spi_device::{JedecIdConfig, Mode, SpiDev, SpiDevCfg};
use pw_status::Error;
use spi_device::SpiDevice;
use spidev_codegen::{handle, signals};
use userspace::time::Instant;
use userspace::{process_entry, syscall};
use util_error::{AsStatus, ErrorCode};
use util_sfdp::create_default_sfdp_table;
use util_zfmt::messages::{ProcessExit, ProcessStart};

fn spidev_server() -> Result<(), ErrorCode> {
    // SAFETY: the spidev process has exclusive access to the SPI Device peripheral.
    let mut dev = unsafe { SpiDev::new(spi_device::RegisterBlock::new(SpiDevice::PTR)) };

    let cfg = SpiDevCfg {
        jedec: JedecIdConfig::GOOGLE,
        mailbox: None,
        mode: Mode::Flashmode,
        initial_address_mode_4b: true,
    };

    dev.init(&cfg)?;

    let sfdp_table = create_default_sfdp_table(64 * 1024 * 1024);
    dev.set_sfdp(&sfdp_table);

    util_zfmt::debug!("spidev: initialized SPI device in Flash mode with SFDP");

    let mut payload_buf: Aligned<A4, [u8; 256]> = Aligned([0u8; 256]);

    loop {
        let wait_result = syscall::object_wait(
            handle::SPIDEV_INTERRUPTS,
            signals::SPI_DEVICE_UPLOAD_CMDFIFO_NOT_EMPTY,
            Instant::MAX,
        )
        .map_err(|e| ErrorCode::kernel_error(e))?;

        while let Some(cmd) = dev.poll(&mut payload_buf) {
            let opcode = cmd.opcode.0;
            util_zfmt::debug!(
                "spidev: received command opcode 0x{opcode:02x}",
                opcode = opcode
            );
            dev.retire_cmd();
        }

        let _ = syscall::interrupt_ack(handle::SPIDEV_INTERRUPTS, wait_result.pending_signals);
    }
}

#[process_entry("spidev")]
fn entry() -> Result<(), Error> {
    util_zfmt::info!(ProcessStart { name: "spidev" });
    let ret = spidev_server();
    util_zfmt::error!(ProcessExit {
        name: "spidev",
        status: ret.as_status()
    });

    let status_res = ret.map_err(|_| Error::Unknown);
    syscall::debug_shutdown(status_res)
}

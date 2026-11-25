// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I2C slave RX burst test — master side (device A)
//!
//! Writes BURST_LEN sequence-tagged frames to SLAVE_ADDR on I2C2 (Bus 2) with
//! no delay between them, then emits TEST_RESULT:PASS.
//!
//! Sent as an IPC client behind this board's own `i2c_server`, the same shape
//! the PLDM firmware-update test's update agent uses (IPC client -> DMA-mode
//! i2c_server), rather than a bare-metal register driver. A write here goes
//! through `write_dma_mode` on this board's controller, so a fix proven
//! against this test exercises the same master-side code path PLDM traffic
//! does.
//!
//! Device B is flashed and booted first (see `BUILD.bazel`'s
//! `system_image_test`), so by the time this image is even running, device B
//! has already called `enable_notification()` and is already listening.
//! There is nothing to wait for and nothing to retry: the first write can go
//! out immediately.
//!
//! Nothing paces the loop: frames go out back-to-back. The slave ACKs each
//! address and then holds SCL low ("slave wait for FW") until its consumer has
//! taken the previous frame, so a write blocks longer rather than failing.

#![no_main]
#![no_std]

use app_i2c_burst_master::handle;
use embedded_hal::i2c::I2c;
use i2c_client::I2cClient;
use i2c_client_ipc::IpcTransport;
use userspace::entry;
use userspace::syscall;

/// Slave address the burst goes to.
const SLAVE_ADDR: u8 = 0x42;

/// Number of frames sent back-to-back. Must match `BURST_LEN` in `slave_burst_main.rs`.
const BURST_LEN: u8 = 8;

fn frame(seq: u8) -> [u8; 4] {
    [seq, 0xA5, 0x5A, !seq]
}

#[entry]
fn entry() {
    let mut master = I2cClient::new(IpcTransport::new(handle::I2C));

    pw_log::info!("Master starting burst");

    for seq in 0..BURST_LEN {
        if master.write(SLAVE_ADDR, &frame(seq)).is_err() {
            pw_log::error!("master write failed at seq={}", seq as u32);
            let _ = syscall::debug_shutdown(Err(pw_status::Error::DataLoss));
            loop {}
        }
    }

    pw_log::info!("Master sent {} frames back-to-back", BURST_LEN as u32);
    let _ = syscall::debug_shutdown(Ok(()));
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

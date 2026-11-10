// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Recovery agent for the emulator, through the MCU's AXI view of the
//! i3c-core.
//!
//! The emulator's BMC only talks to Caliptra core's recovery registers, so
//! the test plays the agent itself. The emulator loads the recovery FIFO from
//! `TTI_TX_DATA_PORT` while `REC_INTF_CFG.REC_INTF_BYPASS` is set (with
//! `emulator_i3c_recovery_fifo.patch`), so the test sets bypass, writes up to
//! one FIFO's worth, and clears bypass before the emulator steps again. The
//! firmware never sees bypass set. This tests the firmware's register
//! sequence and FIFO handling, not the I3C bus side.
//!
//! These AXI writes only stand in for the BMC's I3C writes, and the i3c-core
//! RTL would not accept them: bypass is meant to stay set until reset, and
//! `RECOVERY_CTRL.ACTIVATE_REC_IMG` is write-1-to-clear from AXI. What the
//! firmware reads back matches the I3C flow on hardware.

use caliptra_mcu_hw_model::{DefaultHwModel, McuHwModel, McuManager};
use caliptra_ss_registers::i3c::regs::I3c;
use core::mem::offset_of;
use veer_ocp_recovery::Reg;

const FIFO_DEPTH_WORDS: usize = 64;
const RECOVERY_MODE: u32 = 3;
const RECOVERY_PENDING: u32 = 4;
const AWAITING_IMAGE: u32 = 1;
const FIFO_EMPTY: u32 = 1 << 0;
const REC_INTF_BYPASS: u32 = 1 << 0;
const ACTIVATE: u32 = 0xf << 16;

/// Waits for the request for `image`, sends it and activates it.
pub fn serve(hw: &mut DefaultHwModel, image: &[u32]) {
    let tx_data_port = offset_of!(I3c, tti_tx_data_port);

    step_until(hw, "a request for an image", |hw| {
        read(hw, Reg::DeviceStatus0 as usize) & 0xff == RECOVERY_MODE
            && read(hw, Reg::RecoveryStatus as usize) & 0xf == AWAITING_IMAGE
    });
    let index = (read(hw, Reg::RecoveryStatus as usize) >> 4) & 0xf;
    assert_eq!(
        index,
        u32::from(image::INDEX),
        "firmware asked for the wrong image"
    );

    let len = u32::try_from(image.len()).expect("image too long");
    write(hw, Reg::IndirectFifoCtrl1 as usize, len);
    for chunk in image.chunks(FIFO_DEPTH_WORDS) {
        step_until(hw, "an empty FIFO", |hw| {
            read(hw, Reg::IndirectFifoStatus0 as usize) & FIFO_EMPTY != 0
        });
        let cfg = read(hw, Reg::RecIntfCfg as usize);
        write(hw, Reg::RecIntfCfg as usize, cfg | REC_INTF_BYPASS);
        for &word in chunk {
            write(hw, tx_data_port, word);
        }
        write(hw, Reg::RecIntfCfg as usize, cfg);
    }

    step_until(hw, "recovery pending", |hw| {
        read(hw, Reg::DeviceStatus0 as usize) & 0xff == RECOVERY_PENDING
    });
    write(hw, Reg::RecoveryCtrl as usize, ACTIVATE);
}

/// Steps the model until `done` returns true, checking every few thousand
/// cycles. Panics if the firmware exits first.
fn step_until(
    hw: &mut DefaultHwModel,
    what: &str,
    mut done: impl FnMut(&mut DefaultHwModel) -> bool,
) {
    const CHECK_INTERVAL: u32 = 4096;
    loop {
        if done(hw) {
            return;
        }
        if let Some(status) = hw.output().exit_status() {
            panic!("firmware exited with {status:?} while waiting for {what}");
        }
        for _ in 0..CHECK_INTERVAL {
            hw.step();
        }
    }
}

fn read(hw: &mut DefaultHwModel, offset: usize) -> u32 {
    fn read_with<M: McuManager>(mut mcu: M, offset: usize) -> u32 {
        let addr = M::I3C_ADDR as usize + offset;
        // SAFETY: BusMmio turns the address into a bus read; nothing is
        // dereferenced.
        unsafe { caliptra_ureg::Mmio::read_volatile(&mcu.mmio_mut(), addr as *const u32) }
    }
    read_with(hw.mcu_manager(), offset)
}

fn write(hw: &mut DefaultHwModel, offset: usize, val: u32) {
    fn write_with<M: McuManager>(mut mcu: M, offset: usize, val: u32) {
        let addr = M::I3C_ADDR as usize + offset;
        // SAFETY: BusMmio turns the address into a bus write; nothing is
        // dereferenced.
        unsafe { caliptra_ureg::MmioMut::write_volatile(&mcu.mmio_mut(), addr as *mut u32, val) }
    }
    write_with(hw.mcu_manager(), offset, val)
}

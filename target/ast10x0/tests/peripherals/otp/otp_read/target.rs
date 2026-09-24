// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 OTP read-only board test, ported from OpenPRoT/aspeed-rust PR #38.

#![no_std]
#![no_main]

use ast10x0_peripherals::otp::common::{
    AspeedChipVersion, Logger, NoOpLogger, OtpError, StrapStatus,
};
use ast10x0_peripherals::otp::OtpController;
use console_backend::console_backend_write_all;
use target_common::{declare_target, TargetInterface};
use {console_backend as _, entry as _};

pub struct Target {}

fn otp_err_name(e: OtpError) -> &'static str {
    match e {
        OtpError::InvalidAddress => "InvalidAddress",
        OtpError::InvalidBufSize => "InvalidBufSize",
        OtpError::MemoryLocked => "MemoryLocked",
        OtpError::WriteFailed => "WriteFailed",
        OtpError::ReadFailed => "ReadFailed",
        OtpError::LockFailed => "LockFailed",
        OtpError::VerificationFailed => "VerificationFailed",
        OtpError::WriteExhausted => "WriteExhausted",
        OtpError::NoSession => "NoSession",
        OtpError::RegionProtected => "RegionProtected",
        OtpError::AlignmentError => "AlignmentError",
        OtpError::BoundaryError => "BoundaryError",
        OtpError::Timeout => "Timeout",
        OtpError::UnknownRevId => "UnknownRevId",
        _ => "Unknown",
    }
}

/// OTPCFG in OTP memory (OTPCFG0-31).
fn test_otp_read_conf<L: Logger>(otp: &OtpController<L>, conf_reg: u32) -> bool {
    pw_log::info!("########## test read OTPCFG ######");

    let mut data: [u32; 32] = [0; 32];
    match otp.aspeed_otp_read_conf(0, &mut data) {
        Ok(()) => {
            for (i, each) in data.iter().enumerate() {
                pw_log::info!(
                    "read OTPCFG0x{:x} ok: 0x{:08x} (PASS)",
                    (conf_reg + i as u32) as u32,
                    *each as u32
                );
            }
            true
        }
        Err(e) => {
            pw_log::error!(
                "read OTPCFG0x{:x} err: {} (FAIL)",
                conf_reg as u32,
                otp_err_name(e) as &str
            );
            false
        }
    }
}

fn test_otp_read_strap<L: Logger>(otp: &OtpController<L>, start: u32, count: u32) -> bool {
    pw_log::info!("########## test read OTPSTRAP ######");
    let mut strap_status: [StrapStatus; 64] = [StrapStatus {
        value: false,
        protected: false,
        options: [0; 7],
        remaining_writes: 6,
        writable_option: 0xff,
    }; 64];
    pw_log::info!("BIT(hex)  Value  Option             Status");
    pw_log::info!("------------------------------------------");
    if otp.otp_strap_status(&mut strap_status).is_ok() {
        for i in start..start + count {
            let s = &strap_status[i as usize];
            pw_log::info!(
                "0x{:08x} val={} opt=[{} {} {} {} {} {}] protected={} remaining={}",
                i as u32,
                s.value as u32,
                s.options[0] as u32,
                s.options[1] as u32,
                s.options[2] as u32,
                s.options[3] as u32,
                s.options[4] as u32,
                s.options[5] as u32,
                s.protected as u32,
                s.remaining_writes as u32
            );
        }
        true
    } else {
        pw_log::error!("read otp strap fail! (FAIL)");
        false
    }
}

fn test_otp_read_data<L: Logger>(otp: &OtpController<L>, conf_reg: u32) -> bool {
    pw_log::info!("########## test read OTPDATA ######");

    let mut data: [u32; 32] = [0; 32];
    match otp.aspeed_otp_read_data(0, &mut data) {
        Ok(()) => {
            for (i, each) in data.iter().enumerate() {
                pw_log::info!(
                    "read OTPDATA0x{:x} ok: 0x{:08x} (PASS)",
                    (conf_reg + i as u32) as u32,
                    *each as u32
                );
            }
            true
        }
        Err(e) => {
            pw_log::error!(
                "read OTPDATA0x{:x} err: {} (FAIL)",
                conf_reg as u32,
                otp_err_name(e) as &str
            );
            false
        }
    }
}

fn run_otp_read_test() -> bool {
    // SAFETY: this kernel-only test is the sole owner of the SCU and secure/OTP peripherals.
    let otp = unsafe { OtpController::new(ast1060_pac::Scu::PTR, NoOpLogger) };
    pw_log::info!("=== AST10x0 OTP read-only test ===");

    let version = otp.chip_version();
    pw_log::info!("chip_version=0x{:08x}", version as u32);
    if version == AspeedChipVersion::Unknown {
        pw_log::error!("chip_version is Unknown; aborting (FAIL)");
        return false;
    }

    let conf_ok = test_otp_read_conf(&otp, 0);
    let strap_ok = test_otp_read_strap(&otp, 0, 64);
    let data_ok = test_otp_read_data(&otp, 0);
    conf_ok && strap_ok && data_ok
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 OTP read-only test";

    fn main() -> ! {
        let sentinel = if run_otp_read_test() {
            b"TEST_RESULT:PASS\n"
        } else {
            b"TEST_RESULT:FAIL\n"
        };
        let _ = console_backend_write_all(sentinel);

        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);

// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

//! AST10x0 HACE AES KAT suite.
//!
//! Verifies AES-128/256 ECB/CBC encrypt+decrypt against NIST SP 800-38A
//! vectors, plus InvalidInput enforcement for non-block-size input and a
//! 4 KB CBC round-trip path.

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::hace::{AesCipher, HaceDevice, HaceError};
use codegen as _;
use console_backend::console_backend_write_all;
use entry as _;
use target_common::{declare_target, TargetInterface};

pub struct Target {}

const ERR_AES_FAILED: &str = "hace aes op failed";
const ERR_VERIFY_FAILED: &str = "hace aes mismatch";
const ERR_A4: &str = "hace aes delta-A4 not enforced";

// DMA scratch buffers. Static RAM is required for SG-DMA access.
static mut AES_IN: [u8; 4096] = [0u8; 4096];
static mut AES_OUT: [u8; 4096] = [0u8; 4096];

#[inline]
fn pat(i: usize) -> u8 {
    (i % 251) as u8
}

fn check(name: &str, actual: &[u8], expected: &[u8]) -> Result<(), &'static str> {
    if actual != expected {
        pw_log::error!("{}: mismatch", name as &str);
        let a0 = u32::from_be_bytes([actual[0], actual[1], actual[2], actual[3]]);
        let e0 = u32::from_be_bytes([expected[0], expected[1], expected[2], expected[3]]);
        let az = u32::from_be_bytes([
            actual[actual.len() - 4],
            actual[actual.len() - 3],
            actual[actual.len() - 2],
            actual[actual.len() - 1],
        ]);
        pw_log::error!(
            "  actual[0..4]={:08x} actual[last4]={:08x} expected[0..4]={:08x}",
            a0 as u32,
            az as u32,
            e0 as u32
        );
        return Err(ERR_VERIFY_FAILED);
    }
    pw_log::info!("{}: PASS", name as &str);
    Ok(())
}

/// Run one AES KAT case and compare output bytes.
macro_rules! kat {
    ($name:expr, $op:ident, $key:expr, $iv:expr, $inp:expr, $expected:expr) => {{
        pw_log::info!("case: {}", $name as &str);
        let n = $inp.len();
        // SAFETY: serial single-threaded use of the DMA scratch buffers.
        let inb = unsafe { &mut *core::ptr::addr_of_mut!(AES_IN) };
        let outb = unsafe { &mut *core::ptr::addr_of_mut!(AES_OUT) };
        inb[..n].copy_from_slice($inp);
        outb[..n].fill(0);
        // SAFETY: test runs once at boot with exclusive HACE access.
        let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
        // SAFETY: single-threaded test; no concurrent HACE access.
        let mut aes = unsafe { AesCipher::from_device(&mut device) };
        kat!(@call aes, $op, $key, $iv, inb, outb, n);
        check($name, &outb[..n], $expected)?;
    }};
    (@call $aes:ident, ecb_encrypt, $key:expr, $iv:expr, $inb:ident, $outb:ident, $n:expr) => {
        $aes.ecb_encrypt($key, &$inb[..$n], &mut $outb[..$n]).map_err(|_| ERR_AES_FAILED)?
    };
    (@call $aes:ident, ecb_decrypt, $key:expr, $iv:expr, $inb:ident, $outb:ident, $n:expr) => {
        $aes.ecb_decrypt($key, &$inb[..$n], &mut $outb[..$n]).map_err(|_| ERR_AES_FAILED)?
    };
    (@call $aes:ident, cbc_encrypt, $key:expr, $iv:expr, $inb:ident, $outb:ident, $n:expr) => {
        $aes.cbc_encrypt($key, $iv, &$inb[..$n], &mut $outb[..$n]).map_err(|_| ERR_AES_FAILED)?
    };
    (@call $aes:ident, cbc_decrypt, $key:expr, $iv:expr, $inb:ident, $outb:ident, $n:expr) => {
        $aes.cbc_decrypt($key, $iv, &$inb[..$n], &mut $outb[..$n]).map_err(|_| ERR_AES_FAILED)?
    };
}

include!("vectors.rs");

fn run_hace_aes_kats() -> Result<(), &'static str> {
    pw_log::info!("=== AST10x0 HACE AES KAT suite ===");

    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[],
        i2c_buses: &[],
    });
    // SAFETY: test runs once at boot with exclusive access to the board.
    unsafe {
        let _ = board.init();
    };

    // NIST SP 800-38A KATs.
    kat!(
        "ecb-128 encrypt",
        ecb_encrypt,
        &AES128_KEY,
        &CBC_IV,
        &PT64,
        &ECB128_CT
    );
    kat!(
        "ecb-128 decrypt",
        ecb_decrypt,
        &AES128_KEY,
        &CBC_IV,
        &ECB128_CT,
        &PT64
    );
    kat!(
        "ecb-256 encrypt",
        ecb_encrypt,
        &AES256_KEY,
        &CBC_IV,
        &PT64,
        &ECB256_CT
    );
    kat!(
        "ecb-256 decrypt",
        ecb_decrypt,
        &AES256_KEY,
        &CBC_IV,
        &ECB256_CT,
        &PT64
    );
    kat!(
        "cbc-128 encrypt",
        cbc_encrypt,
        &AES128_KEY,
        &CBC_IV,
        &PT64,
        &CBC128_CT
    );
    kat!(
        "cbc-128 decrypt",
        cbc_decrypt,
        &AES128_KEY,
        &CBC_IV,
        &CBC128_CT,
        &PT64
    );
    kat!(
        "cbc-256 encrypt",
        cbc_encrypt,
        &AES256_KEY,
        &CBC_IV,
        &PT64,
        &CBC256_CT
    );
    kat!(
        "cbc-256 decrypt",
        cbc_decrypt,
        &AES256_KEY,
        &CBC_IV,
        &CBC256_CT,
        &PT64
    );

    // Non-block-size input must return InvalidInput.
    {
        pw_log::info!("case: {}", "delta-A4 reject 17B" as &str);
        let inb = unsafe { &mut *core::ptr::addr_of_mut!(AES_IN) };
        let outb = unsafe { &mut *core::ptr::addr_of_mut!(AES_OUT) };
        for k in 0..17 {
            inb[k] = pat(k);
        }
        let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
        let mut aes = unsafe { AesCipher::from_device(&mut device) };
        match aes.ecb_encrypt(&AES128_KEY, &inb[..17], &mut outb[..17]) {
            Err(HaceError::InvalidInput) => pw_log::info!("delta-A4 reject 17B: PASS"),
            _ => return Err(ERR_A4),
        }
    }

    // Large-buffer CBC round-trip path.
    {
        const N: usize = 4096;
        pw_log::info!("case: {} ({} B)", "cbc-256 roundtrip" as &str, N as u32);
        let inb = unsafe { &mut *core::ptr::addr_of_mut!(AES_IN) };
        let ctb = unsafe { &mut *core::ptr::addr_of_mut!(AES_OUT) };
        for k in 0..N {
            inb[k] = pat(k);
        }
        // Encrypt AES_IN -> AES_OUT.
        {
            let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
            let mut aes = unsafe { AesCipher::from_device(&mut device) };
            aes.cbc_encrypt(&AES256_KEY, &CBC_IV, &inb[..N], &mut ctb[..N])
                .map_err(|_| ERR_AES_FAILED)?;
        }
        // Decrypt AES_OUT back into AES_IN and verify.
        {
            let inb = unsafe { &mut *core::ptr::addr_of_mut!(AES_IN) };
            let ctb = unsafe { &*core::ptr::addr_of!(AES_OUT) };
            let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
            let mut aes = unsafe { AesCipher::from_device(&mut device) };
            aes.cbc_decrypt(&AES256_KEY, &CBC_IV, &ctb[..N], &mut inb[..N])
                .map_err(|_| ERR_AES_FAILED)?;
            for k in 0..N {
                if inb[k] != pat(k) {
                    pw_log::error!("cbc-256 roundtrip: plaintext mismatch");
                    return Err(ERR_VERIFY_FAILED);
                }
            }
            pw_log::info!("cbc-256 roundtrip: PASS");
        }
    }

    pw_log::info!("=== AST10x0 HACE AES KAT suite complete ===");
    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 HACE AES KAT";

    fn main() -> ! {
        let sentinel: &[u8] = match run_hace_aes_kats() {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(error) => {
                pw_log::error!("HACE AES KAT suite failed: {}", error as &str);
                b"TEST_RESULT:FAIL\n"
            }
        };

        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);

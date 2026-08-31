// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use earlgrey_cryptolib::{
    sha256, sha384, sha512, BlindedKey, HardenedBool, KeyConfig, KeyMode, KeySecurityLevel,
    LibVersion, UnblindedKey,
};
use pw_status::Result;
use userspace::entry;
use zerocopy::IntoBytes;

const TEST_INPUT: &[u8] = b"Hello, OpenPRoT!";
const EXPECTED_SHA256_INPUT: [u8; 32] = [
    0xb3, 0xf1, 0xc2, 0x5a, 0xb1, 0x8a, 0xea, 0x2d, 0x1c, 0x8c, 0xa2, 0x15, 0x69, 0x27, 0x92, 0xdf,
    0xc0, 0x41, 0x76, 0xcb, 0x35, 0x6c, 0xfb, 0x18, 0x5d, 0x8c, 0x23, 0xa7, 0xbc, 0x85, 0x3f, 0x24,
];

const EXPECTED_SHA256_EMPTY: [u8; 32] = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
];

const EXPECTED_SHA384_EMPTY: [u8; 48] = [
    0x38, 0xb0, 0x60, 0xa7, 0x51, 0xac, 0x96, 0x38, 0x4c, 0xd9, 0x32, 0x7e, 0xb1, 0xb1, 0xe3, 0x6a,
    0x21, 0xfd, 0xb7, 0x11, 0x14, 0xbe, 0x07, 0x43, 0x4c, 0x0c, 0xc7, 0xbf, 0x63, 0xf6, 0xe1, 0xda,
    0x27, 0x4e, 0xde, 0xbf, 0xe7, 0x6f, 0x65, 0xfb, 0xd5, 0x1a, 0xd2, 0xf1, 0x48, 0x98, 0xb9, 0x5b,
];

const EXPECTED_SHA512_EMPTY: [u8; 64] = [
    0xcf, 0x83, 0xe1, 0x35, 0x7e, 0xef, 0xb8, 0xbd, 0xf1, 0x54, 0x28, 0x50, 0xd6, 0x6d, 0x80, 0x07,
    0xd6, 0x20, 0xe4, 0x05, 0x0b, 0x57, 0x15, 0xdc, 0x83, 0xf4, 0xa9, 0x21, 0xd3, 0x6c, 0xe9, 0xce,
    0x47, 0xd0, 0xd1, 0x3c, 0x5d, 0x85, 0xf2, 0xb0, 0xff, 0x83, 0x18, 0xd2, 0x87, 0x7e, 0xec, 0x2f,
    0x63, 0xb9, 0x31, 0xbd, 0x47, 0x41, 0x7a, 0x81, 0xa5, 0x38, 0x32, 0x7a, 0xf9, 0x27, 0xda, 0x3e,
];

fn test_cryptolib_smoke() -> Result<()> {
    // Test 1: Safe SHA-256 on "Hello, OpenPRoT!"
    pw_log::info!("Testing SHA-256 on \"Hello, OpenPRoT!\"...");
    let mut digest256 = [0u8; 32];
    match sha256(TEST_INPUT, &mut digest256) {
        Ok(()) => {
            if digest256 == EXPECTED_SHA256_INPUT {
                pw_log::info!("SHA-256 test string matched.");
            } else {
                pw_log::error!("SHA-256 test string digest mismatch!");
                return Err(pw_status::Error::Internal.into());
            }
        }
        Err(e) => {
            pw_log::error!("SHA-256 test string failed: {}", e.as_str());
            return Err(pw_status::Error::Internal.into());
        }
    }

    // Test 2: Safe SHA-256 on empty string
    pw_log::info!("Testing SHA-256 on empty input...");
    let mut empty_digest256 = [0u8; 32];
    match sha256(b"", &mut empty_digest256) {
        Ok(()) => {
            if empty_digest256 == EXPECTED_SHA256_EMPTY {
                pw_log::info!("SHA-256 empty input matched.");
            } else {
                pw_log::error!("SHA-256 empty input digest mismatch!");
                return Err(pw_status::Error::Internal.into());
            }
        }
        Err(e) => {
            pw_log::error!("SHA-256 empty input failed: {}", e.as_str());
            return Err(pw_status::Error::Internal.into());
        }
    }

    // Test 3: Safe SHA-384 on empty string
    pw_log::info!("Testing SHA-384 on empty input...");
    let mut empty_digest384 = [0u8; 48];
    match sha384(b"", &mut empty_digest384) {
        Ok(()) => {
            if empty_digest384 == EXPECTED_SHA384_EMPTY {
                pw_log::info!("SHA-384 empty input matched.");
            } else {
                pw_log::error!("SHA-384 empty input digest mismatch!");
                return Err(pw_status::Error::Internal.into());
            }
        }
        Err(e) => {
            pw_log::error!("SHA-384 empty input failed: {}", e.as_str());
            return Err(pw_status::Error::Internal.into());
        }
    }

    // Test 4: Safe SHA-512 on empty string
    pw_log::info!("Testing SHA-512 on empty input...");
    let mut empty_digest512 = [0u8; 64];
    match sha512(b"", &mut empty_digest512) {
        Ok(()) => {
            if empty_digest512 == EXPECTED_SHA512_EMPTY {
                pw_log::info!("SHA-512 empty input matched.");
            } else {
                pw_log::error!("SHA-512 empty input digest mismatch!");
                return Err(pw_status::Error::Internal.into());
            }
        }
        Err(e) => {
            pw_log::error!("SHA-512 empty input failed: {}", e.as_str());
            return Err(pw_status::Error::Internal.into());
        }
    }

    // Test 5: Zerocopy BlindedKey and UnblindedKey serialization and pointer operations
    pw_log::info!("Testing Zerocopy key types and pointer rewrites...");
    let mut blinded = BlindedKey {
        config: KeyConfig {
            version: LibVersion::V1,
            key_mode: KeyMode::AesGcm,
            key_length: 32,
            hw_backed: HardenedBool::False,
            exportable: HardenedBool::True,
            security_level: KeySecurityLevel::Low,
        },
        keyblob_length: 64,
        keyblob: 0,
        checksum: 0x12345678,
    };

    let mut key_buf = [0x55u32; 16];
    blinded.set_keyblob(key_buf.as_mut_ptr());
    if blinded.keyblob_ptr() != key_buf.as_mut_ptr() {
        pw_log::error!("BlindedKey pointer conversion mismatch!");
        return Err(pw_status::Error::Internal.into());
    }

    // Verify zerocopy byte representation
    let blinded_bytes = blinded.as_bytes();
    if blinded_bytes.len() != core::mem::size_of::<BlindedKey>() {
        pw_log::error!("BlindedKey size mismatch in zerocopy serialization!");
        return Err(pw_status::Error::Internal.into());
    }

    let mut unblinded = UnblindedKey {
        key_mode: KeyMode::AesGcm,
        key_length: 32,
        key: 0,
        checksum: 0x87654321,
    };
    unblinded.set_key(key_buf.as_mut_ptr());
    if unblinded.key_ptr() != key_buf.as_mut_ptr() {
        pw_log::error!("UnblindedKey pointer conversion mismatch!");
        return Err(pw_status::Error::Internal.into());
    }
    let unblinded_bytes = unblinded.as_bytes();
    if unblinded_bytes.len() != core::mem::size_of::<UnblindedKey>() {
        pw_log::error!("UnblindedKey size mismatch in zerocopy serialization!");
        return Err(pw_status::Error::Internal.into());
    }

    pw_log::info!("Zerocopy key types verified successfully.");

    Ok(())
}

#[entry]
fn entry() -> Result<()> {
    pw_log::info!("🔄 RUNNING CRYPTOLIB SMOKE TEST");
    let ret = test_cryptolib_smoke();

    if ret.is_err() {
        pw_log::error!("❌ FAIL");
    } else {
        pw_log::info!("✅ PASS");
    }

    ret
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    pw_log::error!("FAIL: panic in cryptolib smoke test");
    loop {}
}

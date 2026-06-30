// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![no_std]
#![no_main]

use ast10x0_board::{Ast10x0Board, Ast10x0BoardDescriptor};
use ast10x0_peripherals::hace::{HaceDevice, HaceDigest, HaceHmac, HmacKey};
use codegen as _;
use console_backend::console_backend_write_all;
use entry as _;
use openprot_hal_blocking::digest::scoped::{DigestInit, DigestOp};
use openprot_hal_blocking::digest::{Sha2_256, Sha2_384, Sha2_512};
use openprot_hal_blocking::mac::scoped::{MacInit, MacOp};
use openprot_hal_blocking::mac::{HmacSha2_256, HmacSha2_384, HmacSha2_512};
use target_common::{declare_target, TargetInterface};

pub struct Target {}

const ERR_HASH_FAILED: &str = "hace hash failed";
const ERR_VERIFY_FAILED: &str = "hace digest mismatch";

// NIST FIPS 180-4 example messages.
const NIST_56: &[u8] = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"; // 448-bit
const NIST_112: &[u8] = b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"; // 896-bit

// Deterministic streaming pattern: byte i = (i % 251). 251 is prime, so the
// pattern does not align with the 64/128-byte block size.
#[inline]
fn pat(i: usize) -> u8 {
    (i % 251) as u8
}

// Streaming input scratch. Static (not stack) so it lives in DMA-reachable .bss
// RAM and avoids a 4 KiB stack frame. Single-threaded test => serial use only.
static mut INPUT_BUF: [u8; 4096] = [0u8; 4096];

fn check(name: &str, actual: &[u8], expected: &[u8]) -> Result<(), &'static str> {
    if actual != expected {
        pw_log::error!("{}: digest mismatch", name as &str);
        let a0 = u32::from_be_bytes([actual[0], actual[1], actual[2], actual[3]]);
        let a1 = u32::from_be_bytes([actual[4], actual[5], actual[6], actual[7]]);
        let az = u32::from_be_bytes([
            actual[actual.len() - 4],
            actual[actual.len() - 3],
            actual[actual.len() - 2],
            actual[actual.len() - 1],
        ]);
        let e0 = u32::from_be_bytes([expected[0], expected[1], expected[2], expected[3]]);
        pw_log::error!(
            "  actual[0..8]={:08x}{:08x} actual[last4]={:08x} expected[0..4]={:08x}",
            a0 as u32,
            a1 as u32,
            az as u32,
            e0 as u32
        );
        return Err(ERR_VERIFY_FAILED);
    }
    pw_log::info!("{}: PASS", name as &str);
    Ok(())
}

/// One-shot KAT: single `update` of `$input`, compare to `$expected`.
macro_rules! oneshot_case {
    ($name:expr, $ty:ty, $val:expr, $input:expr, $expected:expr) => {{
        pw_log::info!("case: {}", $name as &str);
        // SAFETY: test runs once at boot with exclusive HACE access.
        let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
        // SAFETY: single-threaded test; no concurrent HACE access.
        let mut dev = unsafe { HaceDigest::<$ty>::from_device(&mut device) };
        let mut op = dev.init($val).map_err(|_| ERR_HASH_FAILED)?;
        op.update($input).map_err(|_| ERR_HASH_FAILED)?;
        let digest = op.finalize().map_err(|_| ERR_HASH_FAILED)?;
        check($name, digest.as_bytes(), &$expected)?;
    }};
}

/// Production streaming path: feed `pat(0..$total)` in `$chunk`-byte updates,
/// then finalize. Asserts the engine's accumulated digest equals the standard
/// SHA of the whole input (goal.md §3.5 gating test; D2 branch is dormant here).
macro_rules! stream_case {
    ($name:expr, $ty:ty, $val:expr, $total:expr, $chunk:expr, $expected:expr) => {{
        pw_log::info!(
            "case: {} ({} B in {} B chunks)",
            $name as &str,
            $total as u32,
            $chunk as u32
        );
        // SAFETY: test runs once at boot with exclusive HACE access.
        let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
        // SAFETY: single-threaded test; no concurrent HACE access.
        let mut dev = unsafe { HaceDigest::<$ty>::from_device(&mut device) };
        let mut op = dev.init($val).map_err(|_| ERR_HASH_FAILED)?;
        let mut off = 0usize;
        while off < $total {
            let n = core::cmp::min($chunk, $total - off);
            // SAFETY: serial single-threaded use of INPUT_BUF.
            let buf = unsafe { &mut *core::ptr::addr_of_mut!(INPUT_BUF) };
            for k in 0..n {
                buf[k] = pat(off + k);
            }
            op.update(&buf[..n]).map_err(|_| ERR_HASH_FAILED)?;
            off += n;
        }
        let digest = op.finalize().map_err(|_| ERR_HASH_FAILED)?;
        check($name, digest.as_bytes(), &$expected)?;
    }};
}

/// RFC-2104 HMAC KAT: one `update` of `$data` under `$key`, compare to
/// `$expected` (RFC-4231 tag). `$algo` is the `HmacSha2_*` marker.
macro_rules! hmac_case {
    ($name:expr, $algo:expr, $key:expr, $data:expr, $expected:expr) => {{
        pw_log::info!("case: {}", $name as &str);
        // Feed the message from the aligned RAM scratch (same as stream_case!):
        // engine DMA wants a RAM source, not a rodata string literal.
        let src: &[u8] = $data;
        // SAFETY: serial single-threaded use of INPUT_BUF.
        let buf = unsafe { &mut *core::ptr::addr_of_mut!(INPUT_BUF) };
        buf[..src.len()].copy_from_slice(src);
        let data: &[u8] = &buf[..src.len()];
        // SAFETY: test runs once at boot with exclusive HACE access.
        let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
        // SAFETY: single-threaded test; no concurrent HACE access.
        let mut hmac = unsafe { HaceHmac::from_device(&mut device) };
        let key = HmacKey::from_slice($key).map_err(|_| ERR_HASH_FAILED)?;
        let mut op = hmac.init($algo, key).map_err(|_| ERR_HASH_FAILED)?;
        op.update(data).map_err(|_| ERR_HASH_FAILED)?;
        let tag = op.finalize().map_err(|_| ERR_HASH_FAILED)?;
        check($name, tag.as_bytes(), &$expected)?;
    }};
}

include!("vectors.rs");

fn run_hace_sha2_kats() -> Result<(), &'static str> {
    pw_log::info!("=== AST10x0 HACE SHA-2 KAT suite ===");

    let board = Ast10x0Board::new(Ast10x0BoardDescriptor {
        pinctrl_groups: &[],
        i2c_buses: &[],
    });
    // SAFETY: test runs once at boot with exclusive access to the board.
    unsafe {
        let _ = board.init();
    };

    // --- SHA-256 ---
    oneshot_case!("sha256 empty", Sha2_256, Sha2_256, b"", EMPTY_256);
    oneshot_case!("sha256 abc", Sha2_256, Sha2_256, b"abc", ABC_256);
    oneshot_case!("sha256 nist-448", Sha2_256, Sha2_256, NIST_56, NIST56_256);

    // --- SHA-384 ---
    oneshot_case!("sha384 abc", Sha2_384, Sha2_384, b"abc", ABC_384);
    oneshot_case!("sha384 nist-896", Sha2_384, Sha2_384, NIST_112, NIST112_384);

    // --- SHA-512 ---
    oneshot_case!("sha512 abc", Sha2_512, Sha2_512, b"abc", ABC_512);
    oneshot_case!("sha512 nist-896", Sha2_512, Sha2_512, NIST_112, NIST112_512);

    // --- Production streaming path: 9000 B fed as 4096 + 4096 + 808 ---
    // 4096 is a multiple of both 64 and 128, so every full chunk lands on an
    // exact block boundary: this is the dominant PFR workload (goal.md §3.5).
    stream_case!(
        "sha256 stream-9000",
        Sha2_256,
        Sha2_256,
        9000usize,
        4096usize,
        STREAM9000_256
    );
    stream_case!(
        "sha384 stream-9000",
        Sha2_384,
        Sha2_384,
        9000usize,
        4096usize,
        STREAM9000_384
    );
    stream_case!(
        "sha512 stream-9000",
        Sha2_512,
        Sha2_512,
        9000usize,
        4096usize,
        STREAM9000_512
    );

    // --- D2 delta case (goal.md D2) ---
    // SHA-256, block 64. update(100): buffers a 36-byte remainder. update(28):
    // 36 + 28 == 64 -> remainder == 0, the exact-block-boundary branch. The
    // authority leaves `bufcnt` stale here (wrong hash); the port sets it to 0
    // and so yields the *correct* SHA-256 of the 128-byte message. This pins
    // the port's correct behavior — the one place port != Zephyr, on a pattern
    // no real consumer produces.
    {
        pw_log::info!("case: {}", "sha256 d2-boundary" as &str);
        // SAFETY: test runs once at boot with exclusive HACE access.
        let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
        // SAFETY: single-threaded test; no concurrent HACE access.
        let mut dev = unsafe { HaceDigest::<Sha2_256>::from_device(&mut device) };
        let mut op = dev.init(Sha2_256).map_err(|_| ERR_HASH_FAILED)?;
        // SAFETY: serial single-threaded use of INPUT_BUF.
        let buf = unsafe { &mut *core::ptr::addr_of_mut!(INPUT_BUF) };
        for k in 0..128 {
            buf[k] = pat(k);
        }
        op.update(&buf[0..100]).map_err(|_| ERR_HASH_FAILED)?;
        op.update(&buf[100..128]).map_err(|_| ERR_HASH_FAILED)?;
        let digest = op.finalize().map_err(|_| ERR_HASH_FAILED)?;
        check("sha256 d2-boundary", digest.as_bytes(), &D2_128_256)?;
    }

    // --- Diagnostic: SHA of the 131-byte (2-block) key used by RFC-4231 #6,
    //     i.e. the exact input the HMAC key-reduction sub-hash computes. ---
    oneshot_case!("sha384 key6-2block", Sha2_384, Sha2_384, &HMAC_K6, K6_384);
    oneshot_case!("sha512 key6-2block", Sha2_512, Sha2_512, &HMAC_K6, K6_512);

    // --- Diagnostic: reconstruct HMAC-SHA512 #6 stage-by-stage using ONLY the
    //     proven public digest API. If this yields the correct tag, the bug is
    //     in hmac.rs; if inner/tag is wrong here too, it's the digest/engine. ---
    {
        let dmsg: &[u8] = b"Test Using Larger Than Block-Size Key - Hash Key First";
        // K0 = SHA512(K6) zero-padded to 128.
        let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
        let mut dd = unsafe { HaceDigest::<Sha2_512>::from_device(&mut device) };
        let mut op = dd.init(Sha2_512).map_err(|_| ERR_HASH_FAILED)?;
        op.update(&HMAC_K6).map_err(|_| ERR_HASH_FAILED)?;
        let red = op.finalize().map_err(|_| ERR_HASH_FAILED)?;
        let mut k0 = [0u8; 128];
        k0[..64].copy_from_slice(red.as_bytes());
        let mut ipad = [0u8; 128];
        let mut opad = [0u8; 128];
        ipad.copy_from_slice(&k0);
        opad.copy_from_slice(&k0);
        for i in 0..128 {
            ipad[i] ^= 0x36;
            opad[i] ^= 0x5c;
        }
        // inner = SHA512(ipad ‖ msg)
        let ilen = 128 + dmsg.len();
        // SAFETY: serial single-threaded use of INPUT_BUF.
        let buf = unsafe { &mut *core::ptr::addr_of_mut!(INPUT_BUF) };
        buf[..128].copy_from_slice(&ipad);
        buf[128..ilen].copy_from_slice(dmsg);
        let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
        let mut dd = unsafe { HaceDigest::<Sha2_512>::from_device(&mut device) };
        let mut op = dd.init(Sha2_512).map_err(|_| ERR_HASH_FAILED)?;
        op.update(&buf[..ilen]).map_err(|_| ERR_HASH_FAILED)?;
        let innr = op.finalize().map_err(|_| ERR_HASH_FAILED)?;
        check("dbg sha512 inner", innr.as_bytes(), &HMAC6_512_INNER)?;
        // tag = SHA512(opad ‖ inner)
        let mut ib = [0u8; 64];
        ib.copy_from_slice(innr.as_bytes());
        let buf = unsafe { &mut *core::ptr::addr_of_mut!(INPUT_BUF) };
        buf[..128].copy_from_slice(&opad);
        buf[128..128 + 64].copy_from_slice(&ib);
        let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
        let mut dd = unsafe { HaceDigest::<Sha2_512>::from_device(&mut device) };
        let mut op = dd.init(Sha2_512).map_err(|_| ERR_HASH_FAILED)?;
        op.update(&buf[..128 + 64]).map_err(|_| ERR_HASH_FAILED)?;
        let t = op.finalize().map_err(|_| ERR_HASH_FAILED)?;
        check("dbg sha512 hmac6 manual", t.as_bytes(), &HMAC_C6_512)?;
    }

    // --- HMAC (software RFC-2104 over the HACE hasher), RFC-4231 vectors ---
    // Cases 1-4,6,7. Case 6/7 use a 131-byte key (> block size) exercising the
    // RFC-2104-correct `key_len > block_size` reduction path.
    hmac_case!(
        "hmac-sha256 rfc4231-1",
        HmacSha2_256,
        &HMAC_K1,
        b"Hi There",
        HMAC_C1_256
    );
    hmac_case!(
        "hmac-sha256 rfc4231-2",
        HmacSha2_256,
        b"Jefe",
        b"what do ya want for nothing?",
        HMAC_C2_256
    );
    hmac_case!(
        "hmac-sha256 rfc4231-3",
        HmacSha2_256,
        &HMAC_K3,
        &HMAC_D3,
        HMAC_C3_256
    );
    hmac_case!(
        "hmac-sha256 rfc4231-4",
        HmacSha2_256,
        &HMAC_K4,
        &HMAC_D4,
        HMAC_C4_256
    );
    hmac_case!(
        "hmac-sha256 rfc4231-6",
        HmacSha2_256,
        &HMAC_K6,
        b"Test Using Larger Than Block-Size Key - Hash Key First",
        HMAC_C6_256
    );
    hmac_case!("hmac-sha256 rfc4231-7", HmacSha2_256, &HMAC_K7, b"This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm.", HMAC_C7_256);

    hmac_case!(
        "hmac-sha384 rfc4231-1",
        HmacSha2_384,
        &HMAC_K1,
        b"Hi There",
        HMAC_C1_384
    );
    hmac_case!(
        "hmac-sha384 rfc4231-2",
        HmacSha2_384,
        b"Jefe",
        b"what do ya want for nothing?",
        HMAC_C2_384
    );
    hmac_case!(
        "hmac-sha384 rfc4231-3",
        HmacSha2_384,
        &HMAC_K3,
        &HMAC_D3,
        HMAC_C3_384
    );
    hmac_case!(
        "hmac-sha384 rfc4231-4",
        HmacSha2_384,
        &HMAC_K4,
        &HMAC_D4,
        HMAC_C4_384
    );
    hmac_case!(
        "hmac-sha384 rfc4231-6",
        HmacSha2_384,
        &HMAC_K6,
        b"Test Using Larger Than Block-Size Key - Hash Key First",
        HMAC_C6_384
    );
    hmac_case!("hmac-sha384 rfc4231-7", HmacSha2_384, &HMAC_K7, b"This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm.", HMAC_C7_384);

    // Isolation: K6_512 == SHA512(K6). Feeding it as a 64-byte (<=block) key
    // takes hmac.rs's NON-reduce branch but yields the identical K0, so it must
    // equal HMAC(K6,msg). If this PASSES, the in-init kd reduction is the bug.
    hmac_case!(
        "hmac-sha512 prereduced6",
        HmacSha2_512,
        &K6_512,
        b"Test Using Larger Than Block-Size Key - Hash Key First",
        HMAC_C6_512
    );

    // --- Trace: replicate hmac.rs::init() reduce branch exactly for SHA-512 ---
    // hmac.rs copies K6 into HMAC_KEY_NC (.ram_nc) then calls one_shot!(Sha2_512).
    // Here we do the same copy via INPUT_BUF (.bss, also DMA-safe after D1 fix)
    // and hash it. If this gives K6_512, the reduce one_shot! is correct and the
    // bug is in finalize(). If it gives something else, the reduce path is broken.
    {
        pw_log::info!("case: {}", "dbg sha512 reduce-k6-via-bss" as &str);
        let buf = unsafe { &mut *core::ptr::addr_of_mut!(INPUT_BUF) };
        buf[..HMAC_K6.len()].copy_from_slice(&HMAC_K6);
        let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
        let mut dd = unsafe { HaceDigest::<Sha2_512>::from_device(&mut device) };
        let mut op = dd.init(Sha2_512).map_err(|_| ERR_HASH_FAILED)?;
        op.update(&buf[..HMAC_K6.len()]).map_err(|_| ERR_HASH_FAILED)?;
        let kh = op.finalize().map_err(|_| ERR_HASH_FAILED)?;
        check("dbg sha512 reduce-k6-via-bss", kh.as_bytes(), &K6_512)?;
    }

    hmac_case!(
        "hmac-sha512 rfc4231-1",
        HmacSha2_512,
        &HMAC_K1,
        b"Hi There",
        HMAC_C1_512
    );
    hmac_case!(
        "hmac-sha512 rfc4231-2",
        HmacSha2_512,
        b"Jefe",
        b"what do ya want for nothing?",
        HMAC_C2_512
    );
    hmac_case!(
        "hmac-sha512 rfc4231-3",
        HmacSha2_512,
        &HMAC_K3,
        &HMAC_D3,
        HMAC_C3_512
    );
    hmac_case!(
        "hmac-sha512 rfc4231-4",
        HmacSha2_512,
        &HMAC_K4,
        &HMAC_D4,
        HMAC_C4_512
    );
    hmac_case!(
        "hmac-sha512 rfc4231-6",
        HmacSha2_512,
        &HMAC_K6,
        b"Test Using Larger Than Block-Size Key - Hash Key First",
        HMAC_C6_512
    );
    hmac_case!("hmac-sha512 rfc4231-7", HmacSha2_512, &HMAC_K7, b"This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm.", HMAC_C7_512);

    // Streaming HMAC: split RFC-4231 case 7 data across multiple `update`s;
    // must equal the one-shot tag (exercises MacOp::update chaining).
    {
        pw_log::info!("case: {}", "hmac-sha256 streamed-7" as &str);
        // SAFETY: test runs once at boot with exclusive HACE access.
        let mut device = unsafe { HaceDevice::new_global(|_| core::hint::spin_loop()) };
        // SAFETY: single-threaded test; no concurrent HACE access.
        let mut hmac = unsafe { HaceHmac::from_device(&mut device) };
        let key = HmacKey::from_slice(&HMAC_K7).map_err(|_| ERR_HASH_FAILED)?;
        let mut op = hmac.init(HmacSha2_256, key).map_err(|_| ERR_HASH_FAILED)?;
        let data: &[u8] = b"This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm.";
        for chunk in data.chunks(13) {
            op.update(chunk).map_err(|_| ERR_HASH_FAILED)?;
        }
        let tag = op.finalize().map_err(|_| ERR_HASH_FAILED)?;
        check("hmac-sha256 streamed-7", tag.as_bytes(), &HMAC_C7_256)?;
    }

    pw_log::info!("=== AST10x0 HACE SHA-2 KAT suite complete ===");
    Ok(())
}

impl TargetInterface for Target {
    const NAME: &'static str = "AST10x0 HACE SHA-2 KAT";

    fn main() -> ! {
        let sentinel: &[u8] = match run_hace_sha2_kats() {
            Ok(()) => b"TEST_RESULT:PASS\n",
            Err(error) => {
                pw_log::error!("HACE SHA-2 KAT suite failed: {}", error as &str);
                b"TEST_RESULT:FAIL\n"
            }
        };

        let _ = console_backend_write_all(sentinel);
        #[expect(clippy::empty_loop)]
        loop {}
    }
}

declare_target!(Target);

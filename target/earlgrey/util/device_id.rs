// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Device ID formatting utilities.

use zerocopy::IntoBytes;

const HEX_CHARS: [u8; 16] = *b"0123456789abcdef";

/// Formats a 256-bit Device ID ([u32; 8]) into a hex string buffer.
///
/// The formatting matches the byte order used when exposing the Device ID
/// via USB string descriptors (little-endian bytes of each word, from word 0 to 7).
///
/// The buffer must be at least 64 bytes long.
pub fn format_device_id<'a>(device_id: &[u32; 8], buf: &'a mut [u8]) -> Result<&'a str, ()> {
    if buf.len() < 64 {
        return Err(());
    }
    let device_id_bytes = device_id.as_bytes();

    for i in 0..32 {
        let byte = device_id_bytes[i];
        buf[i * 2] = HEX_CHARS[(byte >> 4) as usize];
        buf[i * 2 + 1] = HEX_CHARS[(byte & 0xf) as usize];
    }
    // SAFETY: buf[..64] was populated entirely with valid ASCII hex characters from HEX_CHARS.
    Ok(unsafe { core::str::from_utf8_unchecked(&buf[..64]) })
}

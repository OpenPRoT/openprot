// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use zerocopy::{Immutable, IntoBytes};

const HEX: [u8; 16] = *b"0123456789ABCDEF";

pub fn hexdump<T>(data: &T)
where
    T: IntoBytes + Immutable + ?Sized,
{
    let data = data.as_bytes();
    for (i, d) in data.chunks(16).enumerate() {
        let mut buf = [b' '; 80];
        let mut offset = i * 16;
        for j in 0..8 {
            buf[7 - j] = HEX[offset & 15];
            offset >>= 4;
        }
        for j in 0..d.len() {
            let byte = d[j];
            buf[10 + j * 3] = HEX[(byte >> 4) as usize];
            buf[11 + j * 3] = HEX[(byte & 15) as usize];
            buf[60 + j] = if (0x20..0x7f).contains(&byte) {
                byte
            } else {
                b'.'
            };
        }
        let end = 60 + d.len();
        // nosemgrep
        let line = unsafe {
            // SAFETY: buf is constructed only from ASCII codepoints.
            core::str::from_utf8_unchecked(&buf[..end])
        };
        #[cfg(target_os = "none")]
        pw_log::debug!("{}", line as &str);
        #[cfg(not(target_os = "none"))]
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hexdump() {
        let data = b"Hello, World! This is a test of hexdump utility.";
        hexdump(data);
    }
}

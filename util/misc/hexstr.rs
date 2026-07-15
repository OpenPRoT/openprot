// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use zerocopy::{Immutable, IntoBytes};

const HEX: [u8; 16] = *b"0123456789ABCDEF";

pub fn hexstr<'a, T>(data: &T, buf: &'a mut [u8]) -> Option<&'a str>
where
    T: IntoBytes + Immutable + ?Sized,
{
    let data = data.as_bytes();
    if buf.len() < data.len() * 2 {
        return None;
    }
    for i in 0..data.len() {
        let byte = data[i];
        buf[i * 2] = HEX[(byte >> 4) as usize];
        buf[i * 2 + 1] = HEX[(byte & 15) as usize];
    }
    let end = data.len() * 2;
    // nosemgrep
    unsafe {
        // SAFETY: buf is constructed only from ASCII codepoints.
        Some(core::str::from_utf8_unchecked(&buf[..end]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hexstr() {
        let data = b"12345";
        let mut buf = [0u8; 20];
        let result = hexstr(data, &mut buf);
        assert_eq!(result, Some("3132333435"));
    }

    #[test]
    fn test_hexstr_too_short() {
        let data = b"12345";
        let mut buf = [0u8; 2];
        let result = hexstr(data, &mut buf);
        assert_eq!(result, None);
    }
}

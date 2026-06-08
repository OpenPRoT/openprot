//! Hexdump and hexadecimal string conversion utilities.

use zerocopy::{Immutable, IntoBytes};

const HEX: [u8; 16] = *b"0123456789ABCDEF";

/// Prints a hexdump of the given data to the log.
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
            offset = offset >> 4;
        }
        for (j, &byte) in d.iter().enumerate() {
            buf[10 + j * 3] = HEX[(byte >> 4) as usize];
            buf[11 + j * 3] = HEX[(byte & 15) as usize];
            buf[60 + j] = if byte >= 0x20 && byte < 0x7f {
                byte
            } else {
                b'.'
            };
        }
        let end = 60 + d.len();
        // SAFETY: The buffer contains only ASCII codepoints.
        let line = unsafe { core::str::from_utf8_unchecked(&buf[..end]) };
        pw_log::info!("{}", line as &str);
    }
}

/// Converts the given data to a hexadecimal string.
///
/// The hexadecimal string is written into the `dest` buffer, and a slice of
/// the populated part of `dest` is returned as a `&str`.
pub fn hexstr<'a, T>(dest: &'a mut [u8], data: &T) -> &'a str
where
    T: IntoBytes + Immutable + ?Sized,
{
    let data = data.as_bytes();
    let mut i = 0;
    for &byte in data.iter() {
        dest[i] = HEX[(byte >> 4) as usize];
        dest[i + 1] = HEX[(byte & 15) as usize];
        i += 2;
    }
    // SAFETY: the hex chars emitted into `dest` are ASCII codepoints.
    unsafe { core::str::from_utf8_unchecked(&dest[..i]) }
}

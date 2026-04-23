// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use ufmt::uWrite;
use zerocopy::{Immutable, IntoBytes};

use crate::Console;

const HEX: [u8; 16] = *b"0123456789ABCDEF";

pub fn hexdump_write<W, T>(writer: &mut W, data: &T)
where
    W: uWrite,
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
        for (j, &byte) in d.iter().enumerate() {
            buf[10 + j * 3] = HEX[(byte >> 4) as usize];
            buf[11 + j * 3] = HEX[(byte & 15) as usize];
            buf[60 + j] = if (0x20..0x7f).contains(&byte) {
                byte
            } else {
                b'.'
            };
        }
        let end = 60 + d.len();
        buf[end] = b'\n';
        let line = unsafe { core::str::from_utf8_unchecked(&buf[..end + 1]) };
        let _ = writer.write_str(line);
    }
}

pub fn hexdump<T>(data: &T)
where
    T: IntoBytes + Immutable + ?Sized,
{
    hexdump_write(&mut Console, data);
}

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
    // SAFETY: the hex chars emitted into `dest` are ASCII.
    unsafe { core::str::from_utf8_unchecked(&dest[..i]) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::Infallible;

    struct TestConsole {
        pub buf: [u8; 1024],
        pub len: usize,
    }

    impl TestConsole {
        //fn reset(&mut self) { self.len = 0; }
        fn as_slice(&self) -> &[u8] {
            &self.buf[..self.len]
        }
    }

    impl Default for TestConsole {
        fn default() -> Self {
            Self {
                buf: [0u8; 1024],
                len: 0,
            }
        }
    }

    impl uWrite for TestConsole {
        type Error = Infallible;
        fn write_str(&mut self, s: &str) -> Result<(), Infallible> {
            let s = s.as_bytes();
            let end = self.len + s.len();
            self.buf[self.len..end].copy_from_slice(s);
            self.len = end;
            Ok(())
        }
    }

    // From: http://www.abrahamlincolnonline.org/lincoln/speeches/gettysburg.htm
    const GETTYSBURG_PRELUDE: &str = "\
Four score and seven years ago our fathers brought forth on this \
continent, a new nation, conceived in Liberty, and dedicated to the \
proposition that all men are created equal.";

    const GETTYSBURG_PRELUDE_HEXDUMP: &[u8] = b"\
00000000  46 6F 75 72 20 73 63 6F 72 65 20 61 6E 64 20 73   Four score and s\n\
00000010  65 76 65 6E 20 79 65 61 72 73 20 61 67 6F 20 6F   even years ago o\n\
00000020  75 72 20 66 61 74 68 65 72 73 20 62 72 6F 75 67   ur fathers broug\n\
00000030  68 74 20 66 6F 72 74 68 20 6F 6E 20 74 68 69 73   ht forth on this\n\
00000040  20 63 6F 6E 74 69 6E 65 6E 74 2C 20 61 20 6E 65    continent, a ne\n\
00000050  77 20 6E 61 74 69 6F 6E 2C 20 63 6F 6E 63 65 69   w nation, concei\n\
00000060  76 65 64 20 69 6E 20 4C 69 62 65 72 74 79 2C 20   ved in Liberty, \n\
00000070  61 6E 64 20 64 65 64 69 63 61 74 65 64 20 74 6F   and dedicated to\n\
00000080  20 74 68 65 20 70 72 6F 70 6F 73 69 74 69 6F 6E    the proposition\n\
00000090  20 74 68 61 74 20 61 6C 6C 20 6D 65 6E 20 61 72    that all men ar\n\
000000A0  65 20 63 72 65 61 74 65 64 20 65 71 75 61 6C 2E   e created equal.\n\
";

    // This is the SHA256 digest of the Gettysburg prelude.
    const GETTYSBURG_DIGEST: [u8; 32] = [
        0x1e, 0x6f, 0xd4, 0x03, 0x0f, 0x90, 0x34, 0xcd, 0x77, 0x57, 0x08, 0xa3, 0x96, 0xc3, 0x24,
        0xed, 0x42, 0x0e, 0xc5, 0x87, 0xeb, 0x3d, 0xd4, 0x33, 0xe2, 0x9f, 0x6a, 0xc0, 0x8b, 0x8c,
        0xc7, 0xba,
    ];

    const GETTYSBURG_DIGEST_HEXSTR: &str =
        "1E6FD4030F9034CD775708A396C324ED420EC587EB3DD433E29F6AC08B8CC7BA";

    #[test]
    fn test_hexdump_short() {
        let buf = [0u8, 1, 2, 3, 4, 5, 100, 128, 160];
        let mut console = TestConsole::default();
        hexdump_write(&mut console, &buf);
        assert_eq!(
            console.as_slice(),
            b"00000000  00 01 02 03 04 05 64 80 A0                        ......d..\n"
        );
    }

    #[test]
    fn test_hexdump_long() {
        let mut console = TestConsole::default();
        hexdump_write(&mut console, GETTYSBURG_PRELUDE);
        assert_eq!(console.as_slice(), GETTYSBURG_PRELUDE_HEXDUMP);
    }

    #[test]
    fn test_hexstr() {
        let mut dest = [0u8; 100];
        let result = hexstr(&mut dest, &GETTYSBURG_DIGEST);
        assert_eq!(result, GETTYSBURG_DIGEST_HEXSTR);
    }
}

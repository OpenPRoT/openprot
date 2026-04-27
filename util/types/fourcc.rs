// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

pub mod __private {
    #[allow(unused_imports)]
    pub use ufmt;
}

/// A trait for `FourCC` identifiers.
/// This is unsafe to implement; you must be sure that the target type is 4 bytes in size and that
/// each byte contains a printable ascii character.
pub unsafe trait FourCC: Sized {
    fn as_str(&self) -> &str {
        unsafe {
            let ptr = self as *const Self as *const u8;
            let s = core::slice::from_raw_parts(ptr, 4);
            core::str::from_utf8_unchecked(s)
        }
    }
}

#[macro_export]
macro_rules! impl_fourcc {
    ($t:ty) => {
        const _: () = {
            use $crate::fourcc::FourCC;
            use $crate::fourcc::__private::ufmt;

            unsafe impl FourCC for $t {}

            impl ufmt::uDisplay for $t {
                fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
                where
                    W: ufmt::uWrite + ?Sized,
                {
                    let s = unsafe {
                        let ptr = self as *const Self as *const u8;
                        core::slice::from_raw_parts(ptr, 4)
                    };
                    for byte in s {
                        if (0x20..0x7f).contains(byte) {
                            ufmt::uwrite!(f, "{}", *byte as char)?;
                        } else {
                            ufmt::uwrite!(f, "\\x{:02x}", *byte)?;
                        }
                    }
                    Ok(())
                }
            }

            impl ufmt::uDebug for $t {
                fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
                where
                    W: ufmt::uWrite + ?Sized,
                {
                    ufmt::uDisplay::fmt(self, f)
                }
            }
        };
    };
}

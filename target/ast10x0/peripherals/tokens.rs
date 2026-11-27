// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Ownership tokens for the AST10x0 peripherals.
//!
//! [`Peripherals::take`] hands out one zero-sized token per `ast1060_pac` peripheral, once. A HAL
//! that takes a token by value proves, in the type system, that it is the only owner of that
//! peripheral. [`Peripherals::steal`] is the `unsafe` escape hatch, as in embassy-stm32,
//! embassy-nrf and esp-hal. See `target/ast10x0/MMIO.md`.
//!
//! Each token wraps the zero-sized `ast1060_pac` peripheral it stands for, but gives no register
//! access outside this crate. A driver consumes the token and keeps the PAC peripheral, whose
//! safe `Deref` reaches the register block. This file is the MMIO boundary for that path: the
//! only place a PAC peripheral is created (see `target/ast10x0/MMIO.md`).

use core::sync::atomic::{AtomicBool, Ordering};

static TAKEN: AtomicBool = AtomicBool::new(false);

macro_rules! peripherals {
    ($($field:ident: $Token:ident),* $(,)?) => {
        $(
            #[doc = concat!("Ownership token for the `ast1060_pac::", stringify!($Token), "` peripheral.")]
            ///
            /// Zero-sized. Not `Clone` or `Copy`, and only [`Peripherals`] can create it.
            #[derive(Debug)]
            pub struct $Token(ast1060_pac::$Token);

            #[allow(dead_code, reason = "used as each peripheral's driver moves to tokens (#756)")]
            impl $Token {
                /// Create the token without going through [`Peripherals`].
                ///
                /// # Safety
                /// Same contract as [`Peripherals::steal`], for this one peripheral.
                pub(crate) unsafe fn steal() -> Self {
                    // SAFETY: the caller upholds the ownership contract above.
                    Self(unsafe { ast1060_pac::$Token::steal() })
                }

                /// Give up the token for the PAC peripheral it owns. Crate-private: only a
                /// driver that has consumed the token may reach the registers.
                pub(crate) fn into_pac(self) -> ast1060_pac::$Token {
                    self.0
                }
            }
        )*

        /// One ownership token per AST10x0 peripheral.
        pub struct Peripherals {
            $(
                #[doc = concat!("Token for `", stringify!($Token), "`.")]
                pub $field: $Token,
            )*
        }

        impl Peripherals {
            /// Returns all the peripheral tokens, once. Later calls return `None`.
            #[must_use]
            pub fn take() -> Option<Self> {
                if TAKEN.swap(true, Ordering::AcqRel) {
                    None
                } else {
                    // SAFETY: the flag guarantees this is the first and only call.
                    Some(unsafe { Self::steal_tokens() })
                }
            }

            /// Returns all the peripheral tokens without checking whether they were taken.
            ///
            /// # Safety
            /// Every peripheral reached through the returned tokens, or through tokens from an
            /// earlier `take()`/`steal()`, must have a single owner at a time. Use this only
            /// where an owner cannot be passed down (an interrupt handler, a panic path) and say
            /// in a `// SAFETY:` comment who owns each peripheral that is used.
            #[must_use]
            pub unsafe fn steal() -> Self {
                TAKEN.store(true, Ordering::Release);
                // SAFETY: the caller upholds the ownership contract above.
                unsafe { Self::steal_tokens() }
            }

            /// # Safety
            /// Same contract as [`Self::steal`].
            unsafe fn steal_tokens() -> Self {
                Self {
                    // SAFETY: the caller upholds the ownership contract of `Self::steal`.
                    $($field: unsafe { $Token::steal() },)*
                }
            }
        }
    };
}

peripherals! {
    spi: Spi,
    spi1: Spi1,
    i3cglobal: I3cglobal,
    i3c: I3c,
    i3c1: I3c1,
    i3c2: I3c2,
    i3c3: I3c3,
    gpio: Gpio,
    sgpiom: Sgpiom,
    scu: Scu,
    uart: Uart,
    uartdma: Uartdma,
    i2c: I2c,
    i2c1: I2c1,
    i2c2: I2c2,
    i2c3: I2c3,
    i2c4: I2c4,
    i2c5: I2c5,
    i2c6: I2c6,
    i2c7: I2c7,
    i2c8: I2c8,
    i2c9: I2c9,
    i2c10: I2c10,
    i2c11: I2c11,
    i2c12: I2c12,
    i2c13: I2c13,
    i2cglobal: I2cglobal,
    i2cbuff: I2cbuff,
    i2cbuff1: I2cbuff1,
    i2cbuff2: I2cbuff2,
    i2cbuff3: I2cbuff3,
    i2cbuff4: I2cbuff4,
    i2cbuff5: I2cbuff5,
    i2cbuff6: I2cbuff6,
    i2cbuff7: I2cbuff7,
    i2cbuff8: I2cbuff8,
    i2cbuff9: I2cbuff9,
    i2cbuff10: I2cbuff10,
    i2cbuff11: I2cbuff11,
    i2cbuff12: I2cbuff12,
    i2cbuff13: I2cbuff13,
    hace: Hace,
    secure: Secure,
    i2cfilter: I2cfilter,
    i2c_filter_thr: I2cFilterThr,
    i2c_filter_thr1: I2cFilterThr1,
    i2c_filter_thr2: I2cFilterThr2,
    i2c_filter_thr3: I2cFilterThr3,
    spipf: Spipf,
    spipf1: Spipf1,
    spipf2: Spipf2,
    spipf3: Spipf3,
    wdt: Wdt,
    wdt1: Wdt1,
    wdt2: Wdt2,
    wdt3: Wdt3,
    fmc: Fmc,
    timer: Timer,
    timer1: Timer1,
    timer2: Timer2,
    timer3: Timer3,
    timer4: Timer4,
    timer5: Timer5,
    timer6: Timer6,
    timer7: Timer7,
    timerg: Timerg,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_is_once() {
        // One test fn: `TAKEN` is process-global, so a second test would race this one.
        assert!(Peripherals::take().is_some());
        assert!(Peripherals::take().is_none());
        // SAFETY: test only; no peripheral is touched.
        let _ = unsafe { Peripherals::steal() };
        assert!(Peripherals::take().is_none());
    }
}

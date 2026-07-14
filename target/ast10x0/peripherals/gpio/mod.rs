// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 GPIO peripheral driver.
//!
//! Each GPIO bank (A–U) is exposed as a sub-module (`gpioa`, `gpiob`, …).
//! Obtain a bank via `GPIOX::new_global()` (unsafe), then call `split()` to
//! get the individual typed pins.

mod registers;
mod types;

pub use registers::GpioRegisters;
pub use types::{
    Floating, GpioError, GpioExt, Input, InputMode, InterruptMode, OpenDrain, OpenDrainMode,
    Output, OutputMode, PullDown, PullUp, PushPull, Tristate,
};

macro_rules! gpio_macro {
    ($GPIOX:ident, $gpiox:ident, $x:literal, $pos:literal, $data_val_reg:ident,
        $dir_reg:ident, $int_en_reg:ident, $int_sen_t0:ident,
        $int_sen_t1:ident, $int_sen_t2:ident, $int_sts_reg:ident,
        $rst_tolerant_reg:ident, $deb1_reg:ident, $deb2_reg:ident,
        $cmd_src0_reg:ident, $cmd_src1_reg:ident, $data_read_reg:ident,
        $intput_mask_reg:ident, [
            $($PXi:ident: ($pxi:ident, $i:literal, $MODE:ty),)+
        ]) => {

        pub mod $gpiox {
            use super::*;
            use ast1060_pac as device;
            use core::marker::PhantomData;
            use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin};

            pub struct $GPIOX {
                _regs: GpioRegisters,
            }

            impl $GPIOX {
                /// Create a GPIO bank instance from a [`GpioRegisters`] handle.
                ///
                /// # Safety
                ///
                /// Caller must ensure exclusive access to the GPIO register block is
                /// coordinated for the lifetime of this value.
                pub const unsafe fn new(regs: GpioRegisters) -> Self {
                    Self { _regs: regs }
                }

                /// Create a GPIO bank instance using the global GPIO register block.
                ///
                /// # Safety
                ///
                /// Caller must ensure exclusive access to the singleton GPIO peripheral
                /// is coordinated for the lifetime of this value.
                pub unsafe fn new_global() -> Self {
                    Self {
                        _regs: unsafe { GpioRegisters::new_global() },
                    }
                }

                /// Initialize command-source and debounce registers for this bank.
                pub fn init(&self) {
                    let p = self._regs.regs();
                    p.$cmd_src0_reg().modify(|r, w| unsafe {
                        w.bits(r.bits() & !(0xff << $pos))
                    });
                    p.$cmd_src1_reg().modify(|r, w| unsafe {
                        w.bits(r.bits() & !(0xff << $pos))
                    });
                    p.$deb1_reg().modify(|r, w| unsafe {
                        w.bits(r.bits() & !(0xff << $pos))
                    });
                    p.$deb2_reg().modify(|r, w| unsafe {
                        w.bits(r.bits() & !(0xff << $pos))
                    });
                }
            }

            pub struct Parts {
                $(
                    pub $pxi: $PXi<$MODE>,
                )+
            }

            impl GpioExt for $GPIOX {
                type Parts = Parts;

                fn split(self) -> Self::Parts {
                    Parts {
                        $(
                            $pxi: $PXi {
                                _mode: PhantomData,
                            },
                        )+
                    }
                }
            }

            $(
                pub struct $PXi<MODE> {
                    _mode: PhantomData<MODE>,
                }

                impl<MODE> $PXi<MODE> {
                    /// Configures the pin as a pulled-down input.
                    #[must_use]
                    pub fn into_pull_down_input(self) -> $PXi<Input<PullDown>> {
                        let p = unsafe { &*device::Gpio::ptr() };
                        p.$dir_reg().modify(|r, w| unsafe {
                            w.bits(r.bits() & !(1u32 << ($pos + $i)))
                        });
                        p.$data_val_reg().modify(|r, w| unsafe {
                            w.bits(r.bits() & !(1u32 << ($pos + $i)))
                        });
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin as a pulled-up input.
                    #[must_use]
                    pub fn into_pull_up_input(self) -> $PXi<Input<PullUp>> {
                        let p = unsafe { &*device::Gpio::ptr() };
                        p.$dir_reg().modify(|r, w| unsafe {
                            w.bits(r.bits() & !(1u32 << ($pos + $i)))
                        });
                        p.$data_val_reg().modify(|r, w| unsafe {
                            w.bits(r.bits() | (1u32 << ($pos + $i)))
                        });
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin as an open-drain output.
                    #[must_use]
                    pub fn into_open_drain_output<ODM>(self) -> $PXi<Output<OpenDrain<ODM>>>
                    where
                        ODM: OpenDrainMode,
                    {
                        let p = unsafe { &*device::Gpio::ptr() };
                        p.$data_val_reg().modify(|r, w| unsafe {
                            w.bits(r.bits() | (1u32 << ($pos + $i)))
                        });
                        p.$dir_reg().modify(|r, w| unsafe {
                            w.bits(r.bits() | (1u32 << ($pos + $i)))
                        });
                        $PXi { _mode: PhantomData }
                    }

                    /// Configures the pin as a push-pull output.
                    #[must_use]
                    pub fn into_push_pull_output(self) -> $PXi<Output<PushPull>> {
                        let p = unsafe { &*device::Gpio::ptr() };
                        p.$dir_reg().modify(|r, w| unsafe {
                            w.bits(r.bits() | (1u32 << ($pos + $i)))
                        });
                        p.$data_val_reg().modify(|r, w| unsafe {
                            w.bits(r.bits() | (1u32 << ($pos + $i)))
                        });
                        $PXi { _mode: PhantomData }
                    }
                }

                impl<MODE> StatefulOutputPin for $PXi<Output<MODE>>
                where
                    MODE: OutputMode,
                {
                    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
                        let p = unsafe { &*device::Gpio::ptr() };
                        Ok(
                            (p.$data_read_reg().read().bits() & (1u32 << ($pos + $i)))
                                == (1u32 << ($pos + $i)),
                        )
                    }

                    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
                        self.is_set_high().map(|v| !v)
                    }
                }

                impl<MODE> OutputPin for $PXi<Output<MODE>>
                where
                    MODE: OutputMode,
                {
                    fn set_high(&mut self) -> Result<(), Self::Error> {
                        let p = unsafe { &*device::Gpio::ptr() };
                        p.$data_val_reg().modify(|r, w| unsafe {
                            w.bits(r.bits() | (1u32 << ($pos + $i)))
                        });
                        Ok(())
                    }

                    fn set_low(&mut self) -> Result<(), Self::Error> {
                        let p = unsafe { &*device::Gpio::ptr() };
                        p.$data_val_reg().modify(|r, w| unsafe {
                            w.bits(r.bits() & !(1u32 << ($pos + $i)))
                        });
                        Ok(())
                    }
                }

                impl<MODE> InputPin for $PXi<Input<MODE>>
                where
                    MODE: InputMode,
                {
                    fn is_high(&mut self) -> Result<bool, Self::Error> {
                        let p = unsafe { &*device::Gpio::ptr() };
                        Ok(
                            (p.$data_read_reg().read().bits() & (1u32 << ($pos + $i)))
                                == (1u32 << ($pos + $i)),
                        )
                    }

                    fn is_low(&mut self) -> Result<bool, Self::Error> {
                        self.is_high().map(|v| !v)
                    }
                }

                impl<MODE> $PXi<Input<MODE>>
                where
                    MODE: InputMode,
                {
                    /// Enable or disable interrupts on this pin.
                    pub fn set_interrupt_mode(&mut self, mode: InterruptMode) {
                        let p = unsafe { &*device::Gpio::ptr() };
                        match mode {
                            InterruptMode::LevelHigh => {
                                p.$int_sen_t0().modify(|r, w| unsafe {
                                    w.bits(r.bits() | (1u32 << ($pos + $i)))
                                });
                                p.$int_sen_t1().modify(|r, w| unsafe {
                                    w.bits(r.bits() | (1u32 << ($pos + $i)))
                                });
                                p.$int_sen_t2().modify(|r, w| unsafe {
                                    w.bits(r.bits() & !(1u32 << ($pos + $i)))
                                });
                                p.$int_en_reg().modify(|r, w| unsafe {
                                    w.bits(r.bits() | (1u32 << ($pos + $i)))
                                });
                            }
                            InterruptMode::LevelLow => {
                                p.$int_sen_t0().modify(|r, w| unsafe {
                                    w.bits(r.bits() & !(1u32 << ($pos + $i)))
                                });
                                p.$int_sen_t1().modify(|r, w| unsafe {
                                    w.bits(r.bits() | (1u32 << ($pos + $i)))
                                });
                                p.$int_sen_t2().modify(|r, w| unsafe {
                                    w.bits(r.bits() & !(1u32 << ($pos + $i)))
                                });
                                p.$int_en_reg().modify(|r, w| unsafe {
                                    w.bits(r.bits() | (1u32 << ($pos + $i)))
                                });
                            }
                            InterruptMode::EdgeRising => {
                                p.$int_sen_t0().modify(|r, w| unsafe {
                                    w.bits(r.bits() | (1u32 << ($pos + $i)))
                                });
                                p.$int_sen_t1().modify(|r, w| unsafe {
                                    w.bits(r.bits() & !(1u32 << ($pos + $i)))
                                });
                                p.$int_sen_t2().modify(|r, w| unsafe {
                                    w.bits(r.bits() & !(1u32 << ($pos + $i)))
                                });
                                p.$int_en_reg().modify(|r, w| unsafe {
                                    w.bits(r.bits() | (1u32 << ($pos + $i)))
                                });
                            }
                            InterruptMode::EdgeFalling => {
                                p.$int_sen_t0().modify(|r, w| unsafe {
                                    w.bits(r.bits() & !(1u32 << ($pos + $i)))
                                });
                                p.$int_sen_t1().modify(|r, w| unsafe {
                                    w.bits(r.bits() & !(1u32 << ($pos + $i)))
                                });
                                p.$int_sen_t2().modify(|r, w| unsafe {
                                    w.bits(r.bits() & !(1u32 << ($pos + $i)))
                                });
                                // Note: original uses `| !` (OR with NOT) here, preserved
                                // faithfully.
                                p.$int_en_reg().modify(|r, w| unsafe {
                                    w.bits(r.bits() | !(1u32 << ($pos + $i)))
                                });
                            }
                            InterruptMode::EdgeBoth => {
                                p.$int_sen_t2().modify(|r, w| unsafe {
                                    w.bits(r.bits() | (1u32 << ($pos + $i)))
                                });
                                p.$int_en_reg().modify(|r, w| unsafe {
                                    w.bits(r.bits() | (1u32 << ($pos + $i)))
                                });
                            }
                            InterruptMode::Disabled => {
                                p.$int_en_reg().modify(|r, w| unsafe {
                                    w.bits(r.bits() & !(1u32 << ($pos + $i)))
                                });
                            }
                        }
                    }

                    /// Returns the current interrupt-pending status for this pin.
                    #[must_use]
                    pub fn get_interrupt_status(&self) -> bool {
                        let p = unsafe { &*device::Gpio::ptr() };
                        (p.$int_sts_reg().read().bits() & (1u32 << ($pos + $i)))
                            == (1u32 << ($pos + $i))
                    }

                    /// Clear the pending interrupt for this pin.
                    pub fn clear_interrupt(&self) {
                        let p = unsafe { &*device::Gpio::ptr() };
                        p.$int_sts_reg().write(|w| unsafe { w.bits(1u32 << ($pos + $i)) });
                    }

                    /// Set the command-source bits for this pin.
                    pub fn set_cmd_src(&self, cmd_src0: u32, cmd_src1: u32) {
                        let p = unsafe { &*device::Gpio::ptr() };
                        p.$cmd_src0_reg().modify(|r, w| unsafe {
                            w.bits(
                                (r.bits() & !(1u32 << ($pos + $i)))
                                    | (cmd_src0 << ($pos + $i)),
                            )
                        });
                        p.$cmd_src1_reg().modify(|r, w| unsafe {
                            w.bits(
                                (r.bits() & !(1u32 << ($pos + $i)))
                                    | (cmd_src1 << ($pos + $i)),
                            )
                        });
                    }

                    /// Select the debounce timer for this pin.
                    pub fn select_debounce_timer(&self, deb_setting1: u32, deb_setting2: u32) {
                        let p = unsafe { &*device::Gpio::ptr() };
                        p.$deb1_reg().modify(|r, w| unsafe {
                            w.bits(
                                (r.bits() & !(1u32 << ($pos + $i)))
                                    | (deb_setting1 << ($pos + $i)),
                            )
                        });
                        p.$deb2_reg().modify(|r, w| unsafe {
                            w.bits(
                                (r.bits() & !(1u32 << ($pos + $i)))
                                    | (deb_setting2 << ($pos + $i)),
                            )
                        });
                    }
                }

                impl<MODE> embedded_hal::digital::ErrorType for $PXi<MODE> {
                    type Error = GpioError;
                }
            )+
        }
    };
}

// GPIO ABCD — register group 0x000–0x01c
gpio_macro!(
    GPIOA, gpioa, 'a', 0, gpio000, gpio004, gpio008, gpio00c, gpio010, gpio014, gpio018,
    gpio01c, gpio040, gpio044, gpio060, gpio064, gpio0c0, gpio1d0,
    [
        PA0: (pa0, 0, Tristate),
        PA1: (pa1, 1, Tristate),
        PA2: (pa2, 2, Tristate),
        PA3: (pa3, 3, Tristate),
        PA4: (pa4, 4, Tristate),
        PA5: (pa5, 5, Tristate),
        PA6: (pa6, 6, Tristate),
        PA7: (pa7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOB, gpiob, 'b', 8, gpio000, gpio004, gpio008, gpio00c, gpio010, gpio014, gpio018,
    gpio01c, gpio040, gpio044, gpio060, gpio064, gpio0c0, gpio1d0,
    [
        PB0: (pb0, 0, Tristate),
        PB1: (pb1, 1, Tristate),
        PB2: (pb2, 2, Tristate),
        PB3: (pb3, 3, Tristate),
        PB4: (pb4, 4, Tristate),
        PB5: (pb5, 5, Tristate),
        PB6: (pb6, 6, Tristate),
        PB7: (pb7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOC, gpioc, 'c', 16, gpio000, gpio004, gpio008, gpio00c, gpio010, gpio014, gpio018,
    gpio01c, gpio040, gpio044, gpio060, gpio064, gpio0c0, gpio1d0,
    [
        PC0: (pc0, 0, Tristate),
        PC1: (pc1, 1, Tristate),
        PC2: (pc2, 2, Tristate),
        PC3: (pc3, 3, Tristate),
        PC4: (pc4, 4, Tristate),
        PC5: (pc5, 5, Tristate),
        PC6: (pc6, 6, Tristate),
        PC7: (pc7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOD, gpiod, 'd', 24, gpio000, gpio004, gpio008, gpio00c, gpio010, gpio014, gpio018,
    gpio01c, gpio040, gpio044, gpio060, gpio064, gpio0c0, gpio1d0,
    [
        PD0: (pd0, 0, Tristate),
        PD1: (pd1, 1, Tristate),
        PD2: (pd2, 2, Tristate),
        PD3: (pd3, 3, Tristate),
        PD4: (pd4, 4, Tristate),
        PD5: (pd5, 5, Tristate),
        PD6: (pd6, 6, Tristate),
        PD7: (pd7, 7, Tristate),
    ]
);

// GPIO EFGH — register group 0x020–0x03c
gpio_macro!(
    GPIOE, gpioe, 'e', 0, gpio020, gpio024, gpio028, gpio02c, gpio030, gpio034, gpio038,
    gpio03c, gpio048, gpio04c, gpio068, gpio06c, gpio0c4, gpio1d4,
    [
        PE0: (pe0, 0, Tristate),
        PE1: (pe1, 1, Tristate),
        PE2: (pe2, 2, Tristate),
        PE3: (pe3, 3, Tristate),
        PE4: (pe4, 4, Tristate),
        PE5: (pe5, 5, Tristate),
        PE6: (pe6, 6, Tristate),
        PE7: (pe7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOF, gpiof, 'f', 8, gpio020, gpio024, gpio028, gpio02c, gpio030, gpio034, gpio038,
    gpio03c, gpio048, gpio04c, gpio068, gpio06c, gpio0c4, gpio1d4,
    [
        PF0: (pf0, 0, Tristate),
        PF1: (pf1, 1, Tristate),
        PF2: (pf2, 2, Tristate),
        PF3: (pf3, 3, Tristate),
        PF4: (pf4, 4, Tristate),
        PF5: (pf5, 5, Tristate),
        PF6: (pf6, 6, Tristate),
        PF7: (pf7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOG, gpiog, 'g', 16, gpio020, gpio024, gpio028, gpio02c, gpio030, gpio034, gpio038,
    gpio03c, gpio048, gpio04c, gpio068, gpio06c, gpio0c4, gpio1d4,
    [
        PG0: (pg0, 0, Tristate),
        PG1: (pg1, 1, Tristate),
        PG2: (pg2, 2, Tristate),
        PG3: (pg3, 3, Tristate),
        PG4: (pg4, 4, Tristate),
        PG5: (pg5, 5, Tristate),
        PG6: (pg6, 6, Tristate),
        PG7: (pg7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOH, gpioh, 'h', 24, gpio020, gpio024, gpio028, gpio02c, gpio030, gpio034, gpio038,
    gpio03c, gpio048, gpio04c, gpio068, gpio06c, gpio0c4, gpio1d4,
    [
        PH0: (ph0, 0, Tristate),
        PH1: (ph1, 1, Tristate),
        PH2: (ph2, 2, Tristate),
        PH3: (ph3, 3, Tristate),
        PH4: (ph4, 4, Tristate),
        PH5: (ph5, 5, Tristate),
        PH6: (ph6, 6, Tristate),
        PH7: (ph7, 7, Tristate),
    ]
);

// GPIO IJKL — register group 0x070–0x0b8
gpio_macro!(
    GPIOI, gpioi, 'i', 0, gpio070, gpio074, gpio098, gpio09c, gpio0a0, gpio0a4, gpio0a8,
    gpio0ac, gpio0b0, gpio0b4, gpio090, gpio094, gpio0b8, gpio0c8,
    [
        PI0: (pi0, 0, Tristate),
        PI1: (pi1, 1, Tristate),
        PI2: (pi2, 2, Tristate),
        PI3: (pi3, 3, Tristate),
        PI4: (pi4, 4, Tristate),
        PI5: (pi5, 5, Tristate),
        PI6: (pi6, 6, Tristate),
        PI7: (pi7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOJ, gpioj, 'j', 8, gpio070, gpio074, gpio098, gpio09c, gpio0a0, gpio0a4, gpio0a8,
    gpio0ac, gpio0b0, gpio0b4, gpio090, gpio094, gpio0b8, gpio0c8,
    [
        PJ0: (pj0, 0, Tristate),
        PJ1: (pj1, 1, Tristate),
        PJ2: (pj2, 2, Tristate),
        PJ3: (pj3, 3, Tristate),
        PJ4: (pj4, 4, Tristate),
        PJ5: (pj5, 5, Tristate),
        PJ6: (pj6, 6, Tristate),
        PJ7: (pj7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOK, gpiok, 'k', 16, gpio070, gpio074, gpio098, gpio09c, gpio0a0, gpio0a4, gpio0a8,
    gpio0ac, gpio0b0, gpio0b4, gpio090, gpio094, gpio0b8, gpio0c8,
    [
        PK0: (pk0, 0, Tristate),
        PK1: (pk1, 1, Tristate),
        PK2: (pk2, 2, Tristate),
        PK3: (pk3, 3, Tristate),
        PK4: (pk4, 4, Tristate),
        PK5: (pk5, 5, Tristate),
        PK6: (pk6, 6, Tristate),
        PK7: (pk7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOL, gpiol, 'l', 24, gpio070, gpio074, gpio098, gpio09c, gpio0a0, gpio0a4, gpio0a8,
    gpio0ac, gpio0b0, gpio0b4, gpio090, gpio094, gpio0b8, gpio0c8,
    [
        PL0: (pl0, 0, Tristate),
        PL1: (pl1, 1, Tristate),
        PL2: (pl2, 2, Tristate),
        PL3: (pl3, 3, Tristate),
        PL4: (pl4, 4, Tristate),
        PL5: (pl5, 5, Tristate),
        PL6: (pl6, 6, Tristate),
        PL7: (pl7, 7, Tristate),
    ]
);

// GPIO MNOP — register group 0x078–0x108
gpio_macro!(
    GPIOM, gpiom, 'm', 0, gpio078, gpio07c, gpio0e8, gpio0ec, gpio0f0, gpio0f4, gpio0f8,
    gpio0fc, gpio100, gpio104, gpio0e0, gpio0e4, gpio0cc, gpio108,
    [
        PM0: (pm0, 0, Tristate),
        PM1: (pm1, 1, Tristate),
        PM2: (pm2, 2, Tristate),
        PM3: (pm3, 3, Tristate),
        PM4: (pm4, 4, Tristate),
        PM5: (pm5, 5, Tristate),
        PM6: (pm6, 6, Tristate),
        PM7: (pm7, 7, Tristate),
    ]
);

gpio_macro!(
    GPION, gpion, 'n', 8, gpio078, gpio07c, gpio0e8, gpio0ec, gpio0f0, gpio0f4, gpio0f8,
    gpio0fc, gpio100, gpio104, gpio0e0, gpio0e4, gpio0cc, gpio108,
    [
        PN0: (pn0, 0, Tristate),
        PN1: (pn1, 1, Tristate),
        PN2: (pn2, 2, Tristate),
        PN3: (pn3, 3, Tristate),
        PN4: (pn4, 4, Tristate),
        PN5: (pn5, 5, Tristate),
        PN6: (pn6, 6, Tristate),
        PN7: (pn7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOO, gpioo, 'o', 16, gpio078, gpio07c, gpio0e8, gpio0ec, gpio0f0, gpio0f4, gpio0f8,
    gpio0fc, gpio100, gpio104, gpio0e0, gpio0e4, gpio0cc, gpio108,
    [
        PO0: (po0, 0, Tristate),
        PO1: (po1, 1, Tristate),
        PO2: (po2, 2, Tristate),
        PO3: (po3, 3, Tristate),
        PO4: (po4, 4, Tristate),
        PO5: (po5, 5, Tristate),
        PO6: (po6, 6, Tristate),
        PO7: (po7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOP, gpiop, 'p', 24, gpio078, gpio07c, gpio0e8, gpio0ec, gpio0f0, gpio0f4, gpio0f8,
    gpio0fc, gpio100, gpio104, gpio0e0, gpio0e4, gpio0cc, gpio108,
    [
        PP0: (pp0, 0, Tristate),
        PP1: (pp1, 1, Tristate),
        PP2: (pp2, 2, Tristate),
        PP3: (pp3, 3, Tristate),
        PP4: (pp4, 4, Tristate),
        PP5: (pp5, 5, Tristate),
        PP6: (pp6, 6, Tristate),
        PP7: (pp7, 7, Tristate),
    ]
);

// GPIO QRST — register group 0x080–0x138
gpio_macro!(
    GPIOQ, gpioq, 'q', 0, gpio080, gpio084, gpio118, gpio11c, gpio120, gpio124, gpio128,
    gpio12c, gpio130, gpio134, gpio110, gpio114, gpio0d0, gpio138,
    [
        PQ0: (pq0, 0, Tristate),
        PQ1: (pq1, 1, Tristate),
        PQ2: (pq2, 2, Tristate),
        PQ3: (pq3, 3, Tristate),
        PQ4: (pq4, 4, Tristate),
        PQ5: (pq5, 5, Tristate),
        PQ6: (pq6, 6, Tristate),
        PQ7: (pq7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOR, gpior, 'r', 8, gpio080, gpio084, gpio118, gpio11c, gpio120, gpio124, gpio128,
    gpio12c, gpio130, gpio134, gpio110, gpio114, gpio0d0, gpio138,
    [
        PR0: (pr0, 0, Tristate),
        PR1: (pr1, 1, Tristate),
        PR2: (pr2, 2, Tristate),
        PR3: (pr3, 3, Tristate),
        PR4: (pr4, 4, Tristate),
        PR5: (pr5, 5, Tristate),
        PR6: (pr6, 6, Tristate),
        PR7: (pr7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOS, gpios, 's', 16, gpio080, gpio084, gpio118, gpio11c, gpio120, gpio124, gpio128,
    gpio12c, gpio130, gpio134, gpio110, gpio114, gpio0d0, gpio138,
    [
        PS0: (ps0, 0, Tristate),
        PS1: (ps1, 1, Tristate),
        PS2: (ps2, 2, Tristate),
        PS3: (ps3, 3, Tristate),
        PS4: (ps4, 4, Tristate),
        PS5: (ps5, 5, Tristate),
        PS6: (ps6, 6, Tristate),
        PS7: (ps7, 7, Tristate),
    ]
);

gpio_macro!(
    GPIOT, gpiot, 't', 24, gpio080, gpio084, gpio118, gpio11c, gpio120, gpio124, gpio128,
    gpio12c, gpio130, gpio134, gpio110, gpio114, gpio0d0, gpio138,
    [
        PT0: (pt0, 0, Tristate),
        PT1: (pt1, 1, Tristate),
        PT2: (pt2, 2, Tristate),
        PT3: (pt3, 3, Tristate),
        PT4: (pt4, 4, Tristate),
        PT5: (pt5, 5, Tristate),
        PT6: (pt6, 6, Tristate),
        PT7: (pt7, 7, Tristate),
    ]
);

// GPIO U — register group 0x088–0x168
gpio_macro!(
    GPIOU, gpiou, 'u', 0, gpio088, gpio08c, gpio148, gpio14c, gpio150, gpio154, gpio158,
    gpio15c, gpio160, gpio164, gpio140, gpio144, gpio0d4, gpio168,
    [
        PU0: (pu0, 0, Tristate),
        PU1: (pu1, 1, Tristate),
        PU2: (pu2, 2, Tristate),
        PU3: (pu3, 3, Tristate),
        PU4: (pu4, 4, Tristate),
        PU5: (pu5, 5, Tristate),
        PU6: (pu6, 6, Tristate),
        PU7: (pu7, 7, Tristate),
    ]
);

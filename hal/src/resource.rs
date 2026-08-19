// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Peripheral-agnostic pin vocabulary: the [`Pin`] token marker, the [`Capability`]/[`Routes`] typed
//! datum binding, and the [`pins!`](crate::pins!) generator.

/// A tentative register write.
// TODO: any direct-MMIO bit/field mux; can't express ordered/timed write sequences, poll/read-back mux,
// non-`u32` registers, or off-chip (serial IO-expander) routing.
#[derive(Clone, Copy)]
pub struct FieldWrite {
    /// Register offset the modification targets.
    pub offset: u32,
    /// Bits to OR in.
    pub set: u32,
    /// Bits to AND out.
    pub clr: u32,
}

/// A helper to make a fieldwrite for setting a single bit.
#[must_use]
pub const fn set(offset: u32, bit: u8) -> FieldWrite {
    FieldWrite {
        offset,
        set: 1 << bit,
        clr: 0,
    }
}

/// Clear a single bit.
#[must_use]
pub const fn clear(offset: u32, bit: u8) -> FieldWrite {
    FieldWrite {
        offset,
        set: 0,
        clr: 1 << bit,
    }
}

/// Write a `mask`-wide field: `value` into the field at `shift` (crossbar select).
#[must_use]
pub const fn field(offset: u32, shift: u8, mask: u32, value: u32) -> FieldWrite {
    let reg_mask = mask << shift;
    let set = (value << shift) & reg_mask;
    FieldWrite {
        offset,
        set,
        clr: reg_mask & !set,
    }
}

impl FieldWrite {
    /// Merge another write to the **same** register into this one; const-panics on a set/clear
    /// conflict, so a contradictory config fails to compile rather than silently racing the bits.
    pub(crate) const fn coalesce(&mut self, w: FieldWrite) {
        assert!(
            self.offset == w.offset,
            "coalesce of writes to different registers"
        );
        assert!(
            self.set & w.clr == 0 && self.clr & w.set == 0,
            "config sets and clears the same bit"
        );
        self.set |= w.set;
        self.clr |= w.clr;
    }
}

/// A pin capability (GPIO, I2C-SCL, …): a type whose `Data` is the SoC-defined datum a pin carries
/// for that role. The HAL never reads `Data`; each peripheral defines the type and its own applier.
pub trait Capability {
    /// Data associated to a pin for enabling use of the corresponding capability.
    type Data: Copy;
}

/// A pin's binding for capability `C`.
pub trait Routes<C: Capability> {
    /// List of FieldWrites which must happen for routing the pin to its capability
    const ROUTE: &'static [FieldWrite];
    /// Data associated to the runtime usage of the pin capability
    const DATA: C::Data;
}

/// A compile time pin ownership token needed to access its register bits for routing, or runtime.
pub trait Pin {}

/// Generate a chip's pins.
#[macro_export]
macro_rules! pins {
    (
        $(
            $pin:ident { $( $cap:path : $route:expr => $data:expr ),+ $(,)? }
        ),+ $(,)?
    ) => {
        $(
            #[allow(non_camel_case_types)]
            pub struct $pin(());

            impl $crate::resource::Pin for $pin {}

            $(
                impl $crate::resource::Routes<$cap> for $pin {
                    const ROUTE: &'static [$crate::resource::FieldWrite] = {
                        // A route holds only `FieldWrite`s, so its ctors are in scope here
                        #[allow(unused_imports)]
                        use $crate::resource::{clear, field, set};
                        $route
                    };
                    const DATA: <$cap as $crate::resource::Capability>::Data = $data;
                }
            )+
        )+

        /// Every physical pin this chip exposes, as owned singleton tokens created once at boot.
        pub struct PinTokens {
            $(
                pub $pin: $pin
            ),+
        }

        /// Create this chip's pin tokens — an unforgeable capability asserting each pin names real hardware.
        /// # Safety
        /// The caller must ensure:
        /// - This is called **once** per process. A second call aliases every pin — two owners
        ///   of the same hardware, defeating the exclusivity the token type otherwise enforces.
        /// - The `pins!` table is truthful: each pin's `Routes<C>::ROUTE`/`DATA` names the correct,
        ///   valid mux bits, register bases, and slot for real silicon on this chip.
        #[must_use]
        pub const unsafe fn create_pins() -> PinTokens {
            PinTokens { $( $pin: $pin(()) ),+ }
        }
    };
}

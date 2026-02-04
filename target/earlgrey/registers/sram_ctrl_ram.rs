#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
pub mod regs {
    #![doc = r" Types that represent the values held by registers."]
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    pub mod selector {}
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
}

#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct OtpCtrl {
    _priv: (),
}
impl OtpCtrl {
    pub const PTR: *mut u32 = 0x40138000 as *mut u32;
    #[doc = r" # Safety"]
    #[doc = r""]
    #[doc = r" Caller must ensure that all concurrent use of this"]
    #[doc = r" peripheral in the firmware is done so in a compatible"]
    #[doc = r" way. The simplest way to enforce this is to only call"]
    #[doc = r" this function once."]
    #[inline(always)]
    pub unsafe fn new() -> Self {
        Self { _priv: () }
    }
    #[doc = r" Returns a register block that can be used to read"]
    #[doc = r" registers from this peripheral, but cannot write."]
    #[inline(always)]
    pub fn regs(&self) -> RegisterBlock<ureg::RealMmio<'_>> {
        RegisterBlock {
            ptr: Self::PTR,
            mmio: core::default::Default::default(),
        }
    }
    #[doc = r" Return a register block that can be used to read and"]
    #[doc = r" write this peripheral's registers."]
    #[inline(always)]
    pub fn regs_mut(&mut self) -> RegisterBlock<ureg::RealMmioMut<'_>> {
        RegisterBlock {
            ptr: Self::PTR,
            mmio: core::default::Default::default(),
        }
    }
}
#[derive(Clone, Copy)]
pub struct RegisterBlock<TMmio: ureg::Mmio + core::borrow::Borrow<TMmio>> {
    ptr: *mut u32,
    mmio: TMmio,
}
impl<TMmio: ureg::Mmio + core::default::Default> RegisterBlock<TMmio> {
    #[doc = r" # Safety"]
    #[doc = r""]
    #[doc = r" The caller is responsible for ensuring that ptr is valid for"]
    #[doc = r" volatile reads and writes at any of the offsets in this register"]
    #[doc = r" block."]
    #[inline(always)]
    pub unsafe fn new(ptr: *mut u32) -> Self {
        Self {
            ptr,
            mmio: core::default::Default::default(),
        }
    }
}
impl<TMmio: ureg::Mmio> RegisterBlock<TMmio> {
    #[doc = r" # Safety"]
    #[doc = r""]
    #[doc = r" The caller is responsible for ensuring that ptr is valid for"]
    #[doc = r" volatile reads and writes at any of the offsets in this register"]
    #[doc = r" block."]
    #[inline(always)]
    pub unsafe fn new_with_mmio(ptr: *mut u32, mmio: TMmio) -> Self {
        Self { ptr, mmio }
    }
    #[doc = "\n\nRead value: [`regs::Csr0ReadVal`]; Write value: [`regs::Csr0WriteVal`]"]
    #[inline(always)]
    pub fn csr0(&self) -> ureg::RegRef<crate::meta::Csr0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "\n\nRead value: [`regs::Csr1ReadVal`]; Write value: [`regs::Csr1WriteVal`]"]
    #[inline(always)]
    pub fn csr1(&self) -> ureg::RegRef<crate::meta::Csr1, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "\n\nRead value: [`regs::Csr2ReadVal`]; Write value: [`regs::Csr2WriteVal`]"]
    #[inline(always)]
    pub fn csr2(&self) -> ureg::RegRef<crate::meta::Csr2, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "\n\nRead value: [`regs::Csr3ReadVal`]; Write value: [`regs::Csr3WriteVal`]"]
    #[inline(always)]
    pub fn csr3(&self) -> ureg::RegRef<crate::meta::Csr3, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "\n\nRead value: [`regs::Csr4ReadVal`]; Write value: [`regs::Csr4WriteVal`]"]
    #[inline(always)]
    pub fn csr4(&self) -> ureg::RegRef<crate::meta::Csr4, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "\n\nRead value: [`regs::Csr5ReadVal`]; Write value: [`regs::Csr5WriteVal`]"]
    #[inline(always)]
    pub fn csr5(&self) -> ureg::RegRef<crate::meta::Csr5, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "\n\nRead value: [`regs::Csr6ReadVal`]; Write value: [`regs::Csr6WriteVal`]"]
    #[inline(always)]
    pub fn csr6(&self) -> ureg::RegRef<crate::meta::Csr6, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "\n\nRead value: [`regs::Csr7ReadVal`]; Write value: [`regs::Csr7WriteVal`]"]
    #[inline(always)]
    pub fn csr7(&self) -> ureg::RegRef<crate::meta::Csr7, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
}
pub mod regs {
    #![doc = r" Types that represent the values held by registers."]
    #[derive(Clone, Copy)]
    pub struct Csr0ReadVal(u32);
    impl Csr0ReadVal {
        #[inline(always)]
        pub fn field0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[inline(always)]
        pub fn field1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[inline(always)]
        pub fn field2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[inline(always)]
        pub fn field3(&self) -> u32 {
            (self.0 >> 4) & 0x3ff
        }
        #[inline(always)]
        pub fn field4(&self) -> u32 {
            (self.0 >> 16) & 0x7ff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Csr0WriteVal {
            Csr0WriteVal(self.0)
        }
    }
    impl From<u32> for Csr0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Csr0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr0WriteVal(u32);
    impl Csr0WriteVal {
        #[inline(always)]
        pub fn field0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[inline(always)]
        pub fn field1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[inline(always)]
        pub fn field2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[inline(always)]
        pub fn field3(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 4)) | ((val & 0x3ff) << 4))
        }
        #[inline(always)]
        pub fn field4(self, val: u32) -> Self {
            Self((self.0 & !(0x7ff << 16)) | ((val & 0x7ff) << 16))
        }
    }
    impl From<u32> for Csr0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Csr0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr1ReadVal(u32);
    impl Csr1ReadVal {
        #[inline(always)]
        pub fn field0(&self) -> u32 {
            (self.0 >> 0) & 0x7f
        }
        #[inline(always)]
        pub fn field1(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[inline(always)]
        pub fn field2(&self) -> u32 {
            (self.0 >> 8) & 0x7f
        }
        #[inline(always)]
        pub fn field3(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[inline(always)]
        pub fn field4(&self) -> u32 {
            (self.0 >> 16) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Csr1WriteVal {
            Csr1WriteVal(self.0)
        }
    }
    impl From<u32> for Csr1ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr1ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Csr1ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr1WriteVal(u32);
    impl Csr1WriteVal {
        #[inline(always)]
        pub fn field0(self, val: u32) -> Self {
            Self((self.0 & !(0x7f << 0)) | ((val & 0x7f) << 0))
        }
        #[inline(always)]
        pub fn field1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[inline(always)]
        pub fn field2(self, val: u32) -> Self {
            Self((self.0 & !(0x7f << 8)) | ((val & 0x7f) << 8))
        }
        #[inline(always)]
        pub fn field3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[inline(always)]
        pub fn field4(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 16)) | ((val & 0xffff) << 16))
        }
    }
    impl From<u32> for Csr1WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr1WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Csr1WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr2ReadVal(u32);
    impl Csr2ReadVal {
        #[inline(always)]
        pub fn field0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Csr2WriteVal {
            Csr2WriteVal(self.0)
        }
    }
    impl From<u32> for Csr2ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr2ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Csr2ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr2WriteVal(u32);
    impl Csr2WriteVal {
        #[inline(always)]
        pub fn field0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for Csr2WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr2WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Csr2WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr3ReadVal(u32);
    impl Csr3ReadVal {
        #[inline(always)]
        pub fn field0(&self) -> u32 {
            (self.0 >> 0) & 7
        }
        #[inline(always)]
        pub fn field1(&self) -> u32 {
            (self.0 >> 4) & 0x3ff
        }
        #[inline(always)]
        pub fn field2(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[inline(always)]
        pub fn field3(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[inline(always)]
        pub fn field4(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[inline(always)]
        pub fn field5(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[inline(always)]
        pub fn field6(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[inline(always)]
        pub fn field7(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[inline(always)]
        pub fn field8(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Csr3WriteVal {
            Csr3WriteVal(self.0)
        }
    }
    impl From<u32> for Csr3ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr3ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Csr3ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr3WriteVal(u32);
    impl Csr3WriteVal {
        #[inline(always)]
        pub fn field2_clear(self) -> Self {
            Self(self.0 | (1 << 16))
        }
    }
    impl From<u32> for Csr3WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr3WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Csr3WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr4ReadVal(u32);
    impl Csr4ReadVal {
        #[inline(always)]
        pub fn field0(&self) -> u32 {
            (self.0 >> 0) & 0x3ff
        }
        #[inline(always)]
        pub fn field1(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[inline(always)]
        pub fn field2(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[inline(always)]
        pub fn field3(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Csr4WriteVal {
            Csr4WriteVal(self.0)
        }
    }
    impl From<u32> for Csr4ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr4ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Csr4ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr4WriteVal(u32);
    impl Csr4WriteVal {
        #[inline(always)]
        pub fn field0(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 0)) | ((val & 0x3ff) << 0))
        }
        #[inline(always)]
        pub fn field1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[inline(always)]
        pub fn field2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[inline(always)]
        pub fn field3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
    }
    impl From<u32> for Csr4WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr4WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Csr4WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr5ReadVal(u32);
    impl Csr5ReadVal {
        #[inline(always)]
        pub fn field0(&self) -> u32 {
            (self.0 >> 0) & 0x3f
        }
        #[inline(always)]
        pub fn field1(&self) -> u32 {
            (self.0 >> 6) & 3
        }
        #[inline(always)]
        pub fn field2(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[inline(always)]
        pub fn field3(&self) -> u32 {
            (self.0 >> 9) & 7
        }
        #[inline(always)]
        pub fn field4(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[inline(always)]
        pub fn field5(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[inline(always)]
        pub fn field6(&self) -> u32 {
            (self.0 >> 16) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Csr5WriteVal {
            Csr5WriteVal(self.0)
        }
    }
    impl From<u32> for Csr5ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr5ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Csr5ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr5WriteVal(u32);
    impl Csr5WriteVal {
        #[inline(always)]
        pub fn field0(self, val: u32) -> Self {
            Self((self.0 & !(0x3f << 0)) | ((val & 0x3f) << 0))
        }
        #[inline(always)]
        pub fn field1(self, val: u32) -> Self {
            Self((self.0 & !(3 << 6)) | ((val & 3) << 6))
        }
        #[inline(always)]
        pub fn field6(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 16)) | ((val & 0xffff) << 16))
        }
    }
    impl From<u32> for Csr5WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr5WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Csr5WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr6ReadVal(u32);
    impl Csr6ReadVal {
        #[inline(always)]
        pub fn field0(&self) -> u32 {
            (self.0 >> 0) & 0x3ff
        }
        #[inline(always)]
        pub fn field1(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[inline(always)]
        pub fn field2(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[inline(always)]
        pub fn field3(&self) -> u32 {
            (self.0 >> 16) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Csr6WriteVal {
            Csr6WriteVal(self.0)
        }
    }
    impl From<u32> for Csr6ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr6ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Csr6ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr6WriteVal(u32);
    impl Csr6WriteVal {
        #[inline(always)]
        pub fn field0(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 0)) | ((val & 0x3ff) << 0))
        }
        #[inline(always)]
        pub fn field1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[inline(always)]
        pub fn field2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[inline(always)]
        pub fn field3(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 16)) | ((val & 0xffff) << 16))
        }
    }
    impl From<u32> for Csr6WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr6WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Csr6WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Csr7ReadVal(u32);
    impl Csr7ReadVal {
        #[inline(always)]
        pub fn field0(&self) -> u32 {
            (self.0 >> 0) & 0x3f
        }
        #[inline(always)]
        pub fn field1(&self) -> u32 {
            (self.0 >> 8) & 7
        }
        #[inline(always)]
        pub fn field2(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[inline(always)]
        pub fn field3(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
    }
    impl From<u32> for Csr7ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Csr7ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Csr7ReadVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    pub mod selector {}
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type Csr0 = ureg::ReadWriteReg32<0, crate::regs::Csr0ReadVal, crate::regs::Csr0WriteVal>;
    pub type Csr1 = ureg::ReadWriteReg32<0, crate::regs::Csr1ReadVal, crate::regs::Csr1WriteVal>;
    pub type Csr2 = ureg::ReadWriteReg32<0, crate::regs::Csr2ReadVal, crate::regs::Csr2WriteVal>;
    pub type Csr3 = ureg::ReadWriteReg32<0, crate::regs::Csr3ReadVal, crate::regs::Csr3WriteVal>;
    pub type Csr4 = ureg::ReadWriteReg32<0, crate::regs::Csr4ReadVal, crate::regs::Csr4WriteVal>;
    pub type Csr5 = ureg::ReadWriteReg32<0, crate::regs::Csr5ReadVal, crate::regs::Csr5WriteVal>;
    pub type Csr6 = ureg::ReadWriteReg32<0, crate::regs::Csr6ReadVal, crate::regs::Csr6WriteVal>;
    pub type Csr7 = ureg::ReadOnlyReg32<crate::regs::Csr7ReadVal>;
}

#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct Ast {
    _priv: (),
}
impl Ast {
    pub const PTR: *mut u32 = 0x40480000 as *mut u32;
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
    #[doc = "AST Register 0 for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega0(&self) -> ureg::RegRef<crate::meta::Rega0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 1 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega1(&self) -> ureg::RegRef<crate::meta::Rega1, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 2 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega2(&self) -> ureg::RegRef<crate::meta::Rega2, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 3 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega3(&self) -> ureg::RegRef<crate::meta::Rega3, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 4 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega4(&self) -> ureg::RegRef<crate::meta::Rega4, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 5 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega5(&self) -> ureg::RegRef<crate::meta::Rega5, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 6 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega6(&self) -> ureg::RegRef<crate::meta::Rega6, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 7 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega7(&self) -> ureg::RegRef<crate::meta::Rega7, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 8 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega8(&self) -> ureg::RegRef<crate::meta::Rega8, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 9 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega9(&self) -> ureg::RegRef<crate::meta::Rega9, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 10 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega10(&self) -> ureg::RegRef<crate::meta::Rega10, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 11 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega11(&self) -> ureg::RegRef<crate::meta::Rega11, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 13 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega12(&self) -> ureg::RegRef<crate::meta::Rega12, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 13 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega13(&self) -> ureg::RegRef<crate::meta::Rega13, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 14 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega14(&self) -> ureg::RegRef<crate::meta::Rega14, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x38 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 15 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega15(&self) -> ureg::RegRef<crate::meta::Rega15, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 16 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega16(&self) -> ureg::RegRef<crate::meta::Rega16, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x40 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 17 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega17(&self) -> ureg::RegRef<crate::meta::Rega17, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 18 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega18(&self) -> ureg::RegRef<crate::meta::Rega18, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x48 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 19 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega19(&self) -> ureg::RegRef<crate::meta::Rega19, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 20 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega20(&self) -> ureg::RegRef<crate::meta::Rega20, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x50 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 21 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega21(&self) -> ureg::RegRef<crate::meta::Rega21, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x54 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 22 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega22(&self) -> ureg::RegRef<crate::meta::Rega22, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x58 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 23 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega23(&self) -> ureg::RegRef<crate::meta::Rega23, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x5c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 24 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega24(&self) -> ureg::RegRef<crate::meta::Rega24, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x60 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 25 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega25(&self) -> ureg::RegRef<crate::meta::Rega25, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x64 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 26 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega26(&self) -> ureg::RegRef<crate::meta::Rega26, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x68 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 27 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega27(&self) -> ureg::RegRef<crate::meta::Rega27, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x6c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 28 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega28(&self) -> ureg::RegRef<crate::meta::Rega28, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x70 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 29 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega29(&self) -> ureg::RegRef<crate::meta::Rega29, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x74 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 30 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega30(&self) -> ureg::RegRef<crate::meta::Rega30, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x78 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 31 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega31(&self) -> ureg::RegRef<crate::meta::Rega31, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x7c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 32 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega32(&self) -> ureg::RegRef<crate::meta::Rega32, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x80 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 33 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega33(&self) -> ureg::RegRef<crate::meta::Rega33, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x84 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 34 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega34(&self) -> ureg::RegRef<crate::meta::Rega34, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x88 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 35 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega35(&self) -> ureg::RegRef<crate::meta::Rega35, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x8c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 36 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega36(&self) -> ureg::RegRef<crate::meta::Rega36, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x90 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST 37 Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn rega37(&self) -> ureg::RegRef<crate::meta::Rega37, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x94 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST Last Register for OTP/ROM Write Testing\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn regal(&self) -> ureg::RegRef<crate::meta::Regal, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x98 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "AST Registers Array-B to set address space size\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn regb(&self) -> ureg::Array<5, ureg::RegRef<crate::meta::Regb, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x200 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
}
pub mod regs {
    #![doc = r" Types that represent the values held by registers."]
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    pub mod selector {}
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type Rega0 = ureg::ReadOnlyReg32<u32>;
    pub type Rega1 = ureg::ReadOnlyReg32<u32>;
    pub type Rega2 = ureg::ReadWriteReg32<2, u32, u32>;
    pub type Rega3 = ureg::ReadWriteReg32<3, u32, u32>;
    pub type Rega4 = ureg::ReadWriteReg32<4, u32, u32>;
    pub type Rega5 = ureg::ReadWriteReg32<5, u32, u32>;
    pub type Rega6 = ureg::ReadWriteReg32<6, u32, u32>;
    pub type Rega7 = ureg::ReadWriteReg32<7, u32, u32>;
    pub type Rega8 = ureg::ReadWriteReg32<8, u32, u32>;
    pub type Rega9 = ureg::ReadWriteReg32<9, u32, u32>;
    pub type Rega10 = ureg::ReadWriteReg32<0xa, u32, u32>;
    pub type Rega11 = ureg::ReadWriteReg32<0xb, u32, u32>;
    pub type Rega12 = ureg::ReadWriteReg32<0xc, u32, u32>;
    pub type Rega13 = ureg::ReadWriteReg32<0xd, u32, u32>;
    pub type Rega14 = ureg::ReadWriteReg32<0xe, u32, u32>;
    pub type Rega15 = ureg::ReadWriteReg32<0xf, u32, u32>;
    pub type Rega16 = ureg::ReadWriteReg32<0x10, u32, u32>;
    pub type Rega17 = ureg::ReadWriteReg32<0x11, u32, u32>;
    pub type Rega18 = ureg::ReadWriteReg32<0x12, u32, u32>;
    pub type Rega19 = ureg::ReadWriteReg32<0x13, u32, u32>;
    pub type Rega20 = ureg::ReadWriteReg32<0x14, u32, u32>;
    pub type Rega21 = ureg::ReadWriteReg32<0x15, u32, u32>;
    pub type Rega22 = ureg::ReadWriteReg32<0x16, u32, u32>;
    pub type Rega23 = ureg::ReadWriteReg32<0x17, u32, u32>;
    pub type Rega24 = ureg::ReadWriteReg32<0x18, u32, u32>;
    pub type Rega25 = ureg::ReadWriteReg32<0x19, u32, u32>;
    pub type Rega26 = ureg::ReadWriteReg32<0x1a, u32, u32>;
    pub type Rega27 = ureg::ReadWriteReg32<0x1b, u32, u32>;
    pub type Rega28 = ureg::ReadOnlyReg32<u32>;
    pub type Rega29 = ureg::ReadWriteReg32<0x1d, u32, u32>;
    pub type Rega30 = ureg::ReadWriteReg32<0x1e, u32, u32>;
    pub type Rega31 = ureg::ReadWriteReg32<0x1f, u32, u32>;
    pub type Rega32 = ureg::ReadWriteReg32<0x20, u32, u32>;
    pub type Rega33 = ureg::ReadWriteReg32<0x21, u32, u32>;
    pub type Rega34 = ureg::ReadWriteReg32<0x22, u32, u32>;
    pub type Rega35 = ureg::ReadWriteReg32<0x23, u32, u32>;
    pub type Rega36 = ureg::ReadWriteReg32<0x24, u32, u32>;
    pub type Rega37 = ureg::ReadWriteReg32<0x25, u32, u32>;
    pub type Regal = ureg::WriteOnlyReg32<0x26, u32>;
    pub type Regb = ureg::ReadWriteReg32<0, u32, u32>;
}

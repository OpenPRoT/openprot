#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct RvPlic {
    _priv: (),
}
impl RvPlic {
    pub const PTR: *mut u32 = 0x48000000 as *mut u32;
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
    #[doc = "Interrupt Source 0 Priority\n\nRead value: [`regs::Prio0ReadVal`]; Write value: [`regs::Prio0WriteVal`]"]
    #[inline(always)]
    pub fn prio0(&self) -> ureg::RegRef<crate::meta::Prio0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 1 Priority\n\nRead value: [`regs::Prio1ReadVal`]; Write value: [`regs::Prio1WriteVal`]"]
    #[inline(always)]
    pub fn prio1(&self) -> ureg::RegRef<crate::meta::Prio1, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 2 Priority\n\nRead value: [`regs::Prio2ReadVal`]; Write value: [`regs::Prio2WriteVal`]"]
    #[inline(always)]
    pub fn prio2(&self) -> ureg::RegRef<crate::meta::Prio2, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 3 Priority\n\nRead value: [`regs::Prio3ReadVal`]; Write value: [`regs::Prio3WriteVal`]"]
    #[inline(always)]
    pub fn prio3(&self) -> ureg::RegRef<crate::meta::Prio3, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 4 Priority\n\nRead value: [`regs::Prio4ReadVal`]; Write value: [`regs::Prio4WriteVal`]"]
    #[inline(always)]
    pub fn prio4(&self) -> ureg::RegRef<crate::meta::Prio4, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 5 Priority\n\nRead value: [`regs::Prio5ReadVal`]; Write value: [`regs::Prio5WriteVal`]"]
    #[inline(always)]
    pub fn prio5(&self) -> ureg::RegRef<crate::meta::Prio5, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 6 Priority\n\nRead value: [`regs::Prio6ReadVal`]; Write value: [`regs::Prio6WriteVal`]"]
    #[inline(always)]
    pub fn prio6(&self) -> ureg::RegRef<crate::meta::Prio6, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 7 Priority\n\nRead value: [`regs::Prio7ReadVal`]; Write value: [`regs::Prio7WriteVal`]"]
    #[inline(always)]
    pub fn prio7(&self) -> ureg::RegRef<crate::meta::Prio7, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 8 Priority\n\nRead value: [`regs::Prio8ReadVal`]; Write value: [`regs::Prio8WriteVal`]"]
    #[inline(always)]
    pub fn prio8(&self) -> ureg::RegRef<crate::meta::Prio8, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 9 Priority\n\nRead value: [`regs::Prio9ReadVal`]; Write value: [`regs::Prio9WriteVal`]"]
    #[inline(always)]
    pub fn prio9(&self) -> ureg::RegRef<crate::meta::Prio9, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 10 Priority\n\nRead value: [`regs::Prio10ReadVal`]; Write value: [`regs::Prio10WriteVal`]"]
    #[inline(always)]
    pub fn prio10(&self) -> ureg::RegRef<crate::meta::Prio10, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 11 Priority\n\nRead value: [`regs::Prio11ReadVal`]; Write value: [`regs::Prio11WriteVal`]"]
    #[inline(always)]
    pub fn prio11(&self) -> ureg::RegRef<crate::meta::Prio11, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 12 Priority\n\nRead value: [`regs::Prio12ReadVal`]; Write value: [`regs::Prio12WriteVal`]"]
    #[inline(always)]
    pub fn prio12(&self) -> ureg::RegRef<crate::meta::Prio12, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 13 Priority\n\nRead value: [`regs::Prio13ReadVal`]; Write value: [`regs::Prio13WriteVal`]"]
    #[inline(always)]
    pub fn prio13(&self) -> ureg::RegRef<crate::meta::Prio13, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 14 Priority\n\nRead value: [`regs::Prio14ReadVal`]; Write value: [`regs::Prio14WriteVal`]"]
    #[inline(always)]
    pub fn prio14(&self) -> ureg::RegRef<crate::meta::Prio14, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x38 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 15 Priority\n\nRead value: [`regs::Prio15ReadVal`]; Write value: [`regs::Prio15WriteVal`]"]
    #[inline(always)]
    pub fn prio15(&self) -> ureg::RegRef<crate::meta::Prio15, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 16 Priority\n\nRead value: [`regs::Prio16ReadVal`]; Write value: [`regs::Prio16WriteVal`]"]
    #[inline(always)]
    pub fn prio16(&self) -> ureg::RegRef<crate::meta::Prio16, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x40 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 17 Priority\n\nRead value: [`regs::Prio17ReadVal`]; Write value: [`regs::Prio17WriteVal`]"]
    #[inline(always)]
    pub fn prio17(&self) -> ureg::RegRef<crate::meta::Prio17, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 18 Priority\n\nRead value: [`regs::Prio18ReadVal`]; Write value: [`regs::Prio18WriteVal`]"]
    #[inline(always)]
    pub fn prio18(&self) -> ureg::RegRef<crate::meta::Prio18, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x48 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 19 Priority\n\nRead value: [`regs::Prio19ReadVal`]; Write value: [`regs::Prio19WriteVal`]"]
    #[inline(always)]
    pub fn prio19(&self) -> ureg::RegRef<crate::meta::Prio19, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 20 Priority\n\nRead value: [`regs::Prio20ReadVal`]; Write value: [`regs::Prio20WriteVal`]"]
    #[inline(always)]
    pub fn prio20(&self) -> ureg::RegRef<crate::meta::Prio20, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x50 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 21 Priority\n\nRead value: [`regs::Prio21ReadVal`]; Write value: [`regs::Prio21WriteVal`]"]
    #[inline(always)]
    pub fn prio21(&self) -> ureg::RegRef<crate::meta::Prio21, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x54 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 22 Priority\n\nRead value: [`regs::Prio22ReadVal`]; Write value: [`regs::Prio22WriteVal`]"]
    #[inline(always)]
    pub fn prio22(&self) -> ureg::RegRef<crate::meta::Prio22, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x58 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 23 Priority\n\nRead value: [`regs::Prio23ReadVal`]; Write value: [`regs::Prio23WriteVal`]"]
    #[inline(always)]
    pub fn prio23(&self) -> ureg::RegRef<crate::meta::Prio23, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x5c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 24 Priority\n\nRead value: [`regs::Prio24ReadVal`]; Write value: [`regs::Prio24WriteVal`]"]
    #[inline(always)]
    pub fn prio24(&self) -> ureg::RegRef<crate::meta::Prio24, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x60 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 25 Priority\n\nRead value: [`regs::Prio25ReadVal`]; Write value: [`regs::Prio25WriteVal`]"]
    #[inline(always)]
    pub fn prio25(&self) -> ureg::RegRef<crate::meta::Prio25, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x64 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 26 Priority\n\nRead value: [`regs::Prio26ReadVal`]; Write value: [`regs::Prio26WriteVal`]"]
    #[inline(always)]
    pub fn prio26(&self) -> ureg::RegRef<crate::meta::Prio26, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x68 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 27 Priority\n\nRead value: [`regs::Prio27ReadVal`]; Write value: [`regs::Prio27WriteVal`]"]
    #[inline(always)]
    pub fn prio27(&self) -> ureg::RegRef<crate::meta::Prio27, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x6c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 28 Priority\n\nRead value: [`regs::Prio28ReadVal`]; Write value: [`regs::Prio28WriteVal`]"]
    #[inline(always)]
    pub fn prio28(&self) -> ureg::RegRef<crate::meta::Prio28, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x70 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 29 Priority\n\nRead value: [`regs::Prio29ReadVal`]; Write value: [`regs::Prio29WriteVal`]"]
    #[inline(always)]
    pub fn prio29(&self) -> ureg::RegRef<crate::meta::Prio29, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x74 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 30 Priority\n\nRead value: [`regs::Prio30ReadVal`]; Write value: [`regs::Prio30WriteVal`]"]
    #[inline(always)]
    pub fn prio30(&self) -> ureg::RegRef<crate::meta::Prio30, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x78 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 31 Priority\n\nRead value: [`regs::Prio31ReadVal`]; Write value: [`regs::Prio31WriteVal`]"]
    #[inline(always)]
    pub fn prio31(&self) -> ureg::RegRef<crate::meta::Prio31, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x7c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 32 Priority\n\nRead value: [`regs::Prio32ReadVal`]; Write value: [`regs::Prio32WriteVal`]"]
    #[inline(always)]
    pub fn prio32(&self) -> ureg::RegRef<crate::meta::Prio32, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x80 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 33 Priority\n\nRead value: [`regs::Prio33ReadVal`]; Write value: [`regs::Prio33WriteVal`]"]
    #[inline(always)]
    pub fn prio33(&self) -> ureg::RegRef<crate::meta::Prio33, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x84 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 34 Priority\n\nRead value: [`regs::Prio34ReadVal`]; Write value: [`regs::Prio34WriteVal`]"]
    #[inline(always)]
    pub fn prio34(&self) -> ureg::RegRef<crate::meta::Prio34, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x88 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 35 Priority\n\nRead value: [`regs::Prio35ReadVal`]; Write value: [`regs::Prio35WriteVal`]"]
    #[inline(always)]
    pub fn prio35(&self) -> ureg::RegRef<crate::meta::Prio35, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x8c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 36 Priority\n\nRead value: [`regs::Prio36ReadVal`]; Write value: [`regs::Prio36WriteVal`]"]
    #[inline(always)]
    pub fn prio36(&self) -> ureg::RegRef<crate::meta::Prio36, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x90 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 37 Priority\n\nRead value: [`regs::Prio37ReadVal`]; Write value: [`regs::Prio37WriteVal`]"]
    #[inline(always)]
    pub fn prio37(&self) -> ureg::RegRef<crate::meta::Prio37, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x94 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 38 Priority\n\nRead value: [`regs::Prio38ReadVal`]; Write value: [`regs::Prio38WriteVal`]"]
    #[inline(always)]
    pub fn prio38(&self) -> ureg::RegRef<crate::meta::Prio38, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x98 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 39 Priority\n\nRead value: [`regs::Prio39ReadVal`]; Write value: [`regs::Prio39WriteVal`]"]
    #[inline(always)]
    pub fn prio39(&self) -> ureg::RegRef<crate::meta::Prio39, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x9c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 40 Priority\n\nRead value: [`regs::Prio40ReadVal`]; Write value: [`regs::Prio40WriteVal`]"]
    #[inline(always)]
    pub fn prio40(&self) -> ureg::RegRef<crate::meta::Prio40, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xa0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 41 Priority\n\nRead value: [`regs::Prio41ReadVal`]; Write value: [`regs::Prio41WriteVal`]"]
    #[inline(always)]
    pub fn prio41(&self) -> ureg::RegRef<crate::meta::Prio41, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xa4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 42 Priority\n\nRead value: [`regs::Prio42ReadVal`]; Write value: [`regs::Prio42WriteVal`]"]
    #[inline(always)]
    pub fn prio42(&self) -> ureg::RegRef<crate::meta::Prio42, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xa8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 43 Priority\n\nRead value: [`regs::Prio43ReadVal`]; Write value: [`regs::Prio43WriteVal`]"]
    #[inline(always)]
    pub fn prio43(&self) -> ureg::RegRef<crate::meta::Prio43, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xac / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 44 Priority\n\nRead value: [`regs::Prio44ReadVal`]; Write value: [`regs::Prio44WriteVal`]"]
    #[inline(always)]
    pub fn prio44(&self) -> ureg::RegRef<crate::meta::Prio44, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xb0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 45 Priority\n\nRead value: [`regs::Prio45ReadVal`]; Write value: [`regs::Prio45WriteVal`]"]
    #[inline(always)]
    pub fn prio45(&self) -> ureg::RegRef<crate::meta::Prio45, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xb4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 46 Priority\n\nRead value: [`regs::Prio46ReadVal`]; Write value: [`regs::Prio46WriteVal`]"]
    #[inline(always)]
    pub fn prio46(&self) -> ureg::RegRef<crate::meta::Prio46, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xb8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 47 Priority\n\nRead value: [`regs::Prio47ReadVal`]; Write value: [`regs::Prio47WriteVal`]"]
    #[inline(always)]
    pub fn prio47(&self) -> ureg::RegRef<crate::meta::Prio47, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xbc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 48 Priority\n\nRead value: [`regs::Prio48ReadVal`]; Write value: [`regs::Prio48WriteVal`]"]
    #[inline(always)]
    pub fn prio48(&self) -> ureg::RegRef<crate::meta::Prio48, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 49 Priority\n\nRead value: [`regs::Prio49ReadVal`]; Write value: [`regs::Prio49WriteVal`]"]
    #[inline(always)]
    pub fn prio49(&self) -> ureg::RegRef<crate::meta::Prio49, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 50 Priority\n\nRead value: [`regs::Prio50ReadVal`]; Write value: [`regs::Prio50WriteVal`]"]
    #[inline(always)]
    pub fn prio50(&self) -> ureg::RegRef<crate::meta::Prio50, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 51 Priority\n\nRead value: [`regs::Prio51ReadVal`]; Write value: [`regs::Prio51WriteVal`]"]
    #[inline(always)]
    pub fn prio51(&self) -> ureg::RegRef<crate::meta::Prio51, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xcc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 52 Priority\n\nRead value: [`regs::Prio52ReadVal`]; Write value: [`regs::Prio52WriteVal`]"]
    #[inline(always)]
    pub fn prio52(&self) -> ureg::RegRef<crate::meta::Prio52, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xd0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 53 Priority\n\nRead value: [`regs::Prio53ReadVal`]; Write value: [`regs::Prio53WriteVal`]"]
    #[inline(always)]
    pub fn prio53(&self) -> ureg::RegRef<crate::meta::Prio53, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xd4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 54 Priority\n\nRead value: [`regs::Prio54ReadVal`]; Write value: [`regs::Prio54WriteVal`]"]
    #[inline(always)]
    pub fn prio54(&self) -> ureg::RegRef<crate::meta::Prio54, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xd8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 55 Priority\n\nRead value: [`regs::Prio55ReadVal`]; Write value: [`regs::Prio55WriteVal`]"]
    #[inline(always)]
    pub fn prio55(&self) -> ureg::RegRef<crate::meta::Prio55, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xdc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 56 Priority\n\nRead value: [`regs::Prio56ReadVal`]; Write value: [`regs::Prio56WriteVal`]"]
    #[inline(always)]
    pub fn prio56(&self) -> ureg::RegRef<crate::meta::Prio56, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xe0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 57 Priority\n\nRead value: [`regs::Prio57ReadVal`]; Write value: [`regs::Prio57WriteVal`]"]
    #[inline(always)]
    pub fn prio57(&self) -> ureg::RegRef<crate::meta::Prio57, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xe4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 58 Priority\n\nRead value: [`regs::Prio58ReadVal`]; Write value: [`regs::Prio58WriteVal`]"]
    #[inline(always)]
    pub fn prio58(&self) -> ureg::RegRef<crate::meta::Prio58, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xe8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 59 Priority\n\nRead value: [`regs::Prio59ReadVal`]; Write value: [`regs::Prio59WriteVal`]"]
    #[inline(always)]
    pub fn prio59(&self) -> ureg::RegRef<crate::meta::Prio59, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xec / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 60 Priority\n\nRead value: [`regs::Prio60ReadVal`]; Write value: [`regs::Prio60WriteVal`]"]
    #[inline(always)]
    pub fn prio60(&self) -> ureg::RegRef<crate::meta::Prio60, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xf0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 61 Priority\n\nRead value: [`regs::Prio61ReadVal`]; Write value: [`regs::Prio61WriteVal`]"]
    #[inline(always)]
    pub fn prio61(&self) -> ureg::RegRef<crate::meta::Prio61, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xf4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 62 Priority\n\nRead value: [`regs::Prio62ReadVal`]; Write value: [`regs::Prio62WriteVal`]"]
    #[inline(always)]
    pub fn prio62(&self) -> ureg::RegRef<crate::meta::Prio62, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xf8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 63 Priority\n\nRead value: [`regs::Prio63ReadVal`]; Write value: [`regs::Prio63WriteVal`]"]
    #[inline(always)]
    pub fn prio63(&self) -> ureg::RegRef<crate::meta::Prio63, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xfc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 64 Priority\n\nRead value: [`regs::Prio64ReadVal`]; Write value: [`regs::Prio64WriteVal`]"]
    #[inline(always)]
    pub fn prio64(&self) -> ureg::RegRef<crate::meta::Prio64, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x100 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 65 Priority\n\nRead value: [`regs::Prio65ReadVal`]; Write value: [`regs::Prio65WriteVal`]"]
    #[inline(always)]
    pub fn prio65(&self) -> ureg::RegRef<crate::meta::Prio65, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x104 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 66 Priority\n\nRead value: [`regs::Prio66ReadVal`]; Write value: [`regs::Prio66WriteVal`]"]
    #[inline(always)]
    pub fn prio66(&self) -> ureg::RegRef<crate::meta::Prio66, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x108 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 67 Priority\n\nRead value: [`regs::Prio67ReadVal`]; Write value: [`regs::Prio67WriteVal`]"]
    #[inline(always)]
    pub fn prio67(&self) -> ureg::RegRef<crate::meta::Prio67, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 68 Priority\n\nRead value: [`regs::Prio68ReadVal`]; Write value: [`regs::Prio68WriteVal`]"]
    #[inline(always)]
    pub fn prio68(&self) -> ureg::RegRef<crate::meta::Prio68, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x110 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 69 Priority\n\nRead value: [`regs::Prio69ReadVal`]; Write value: [`regs::Prio69WriteVal`]"]
    #[inline(always)]
    pub fn prio69(&self) -> ureg::RegRef<crate::meta::Prio69, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x114 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 70 Priority\n\nRead value: [`regs::Prio70ReadVal`]; Write value: [`regs::Prio70WriteVal`]"]
    #[inline(always)]
    pub fn prio70(&self) -> ureg::RegRef<crate::meta::Prio70, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x118 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 71 Priority\n\nRead value: [`regs::Prio71ReadVal`]; Write value: [`regs::Prio71WriteVal`]"]
    #[inline(always)]
    pub fn prio71(&self) -> ureg::RegRef<crate::meta::Prio71, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x11c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 72 Priority\n\nRead value: [`regs::Prio72ReadVal`]; Write value: [`regs::Prio72WriteVal`]"]
    #[inline(always)]
    pub fn prio72(&self) -> ureg::RegRef<crate::meta::Prio72, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x120 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 73 Priority\n\nRead value: [`regs::Prio73ReadVal`]; Write value: [`regs::Prio73WriteVal`]"]
    #[inline(always)]
    pub fn prio73(&self) -> ureg::RegRef<crate::meta::Prio73, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x124 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 74 Priority\n\nRead value: [`regs::Prio74ReadVal`]; Write value: [`regs::Prio74WriteVal`]"]
    #[inline(always)]
    pub fn prio74(&self) -> ureg::RegRef<crate::meta::Prio74, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x128 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 75 Priority\n\nRead value: [`regs::Prio75ReadVal`]; Write value: [`regs::Prio75WriteVal`]"]
    #[inline(always)]
    pub fn prio75(&self) -> ureg::RegRef<crate::meta::Prio75, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x12c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 76 Priority\n\nRead value: [`regs::Prio76ReadVal`]; Write value: [`regs::Prio76WriteVal`]"]
    #[inline(always)]
    pub fn prio76(&self) -> ureg::RegRef<crate::meta::Prio76, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x130 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 77 Priority\n\nRead value: [`regs::Prio77ReadVal`]; Write value: [`regs::Prio77WriteVal`]"]
    #[inline(always)]
    pub fn prio77(&self) -> ureg::RegRef<crate::meta::Prio77, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x134 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 78 Priority\n\nRead value: [`regs::Prio78ReadVal`]; Write value: [`regs::Prio78WriteVal`]"]
    #[inline(always)]
    pub fn prio78(&self) -> ureg::RegRef<crate::meta::Prio78, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x138 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 79 Priority\n\nRead value: [`regs::Prio79ReadVal`]; Write value: [`regs::Prio79WriteVal`]"]
    #[inline(always)]
    pub fn prio79(&self) -> ureg::RegRef<crate::meta::Prio79, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x13c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 80 Priority\n\nRead value: [`regs::Prio80ReadVal`]; Write value: [`regs::Prio80WriteVal`]"]
    #[inline(always)]
    pub fn prio80(&self) -> ureg::RegRef<crate::meta::Prio80, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x140 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 81 Priority\n\nRead value: [`regs::Prio81ReadVal`]; Write value: [`regs::Prio81WriteVal`]"]
    #[inline(always)]
    pub fn prio81(&self) -> ureg::RegRef<crate::meta::Prio81, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x144 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 82 Priority\n\nRead value: [`regs::Prio82ReadVal`]; Write value: [`regs::Prio82WriteVal`]"]
    #[inline(always)]
    pub fn prio82(&self) -> ureg::RegRef<crate::meta::Prio82, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x148 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 83 Priority\n\nRead value: [`regs::Prio83ReadVal`]; Write value: [`regs::Prio83WriteVal`]"]
    #[inline(always)]
    pub fn prio83(&self) -> ureg::RegRef<crate::meta::Prio83, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 84 Priority\n\nRead value: [`regs::Prio84ReadVal`]; Write value: [`regs::Prio84WriteVal`]"]
    #[inline(always)]
    pub fn prio84(&self) -> ureg::RegRef<crate::meta::Prio84, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x150 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 85 Priority\n\nRead value: [`regs::Prio85ReadVal`]; Write value: [`regs::Prio85WriteVal`]"]
    #[inline(always)]
    pub fn prio85(&self) -> ureg::RegRef<crate::meta::Prio85, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x154 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 86 Priority\n\nRead value: [`regs::Prio86ReadVal`]; Write value: [`regs::Prio86WriteVal`]"]
    #[inline(always)]
    pub fn prio86(&self) -> ureg::RegRef<crate::meta::Prio86, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x158 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 87 Priority\n\nRead value: [`regs::Prio87ReadVal`]; Write value: [`regs::Prio87WriteVal`]"]
    #[inline(always)]
    pub fn prio87(&self) -> ureg::RegRef<crate::meta::Prio87, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x15c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 88 Priority\n\nRead value: [`regs::Prio88ReadVal`]; Write value: [`regs::Prio88WriteVal`]"]
    #[inline(always)]
    pub fn prio88(&self) -> ureg::RegRef<crate::meta::Prio88, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x160 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 89 Priority\n\nRead value: [`regs::Prio89ReadVal`]; Write value: [`regs::Prio89WriteVal`]"]
    #[inline(always)]
    pub fn prio89(&self) -> ureg::RegRef<crate::meta::Prio89, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x164 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 90 Priority\n\nRead value: [`regs::Prio90ReadVal`]; Write value: [`regs::Prio90WriteVal`]"]
    #[inline(always)]
    pub fn prio90(&self) -> ureg::RegRef<crate::meta::Prio90, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x168 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 91 Priority\n\nRead value: [`regs::Prio91ReadVal`]; Write value: [`regs::Prio91WriteVal`]"]
    #[inline(always)]
    pub fn prio91(&self) -> ureg::RegRef<crate::meta::Prio91, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x16c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 92 Priority\n\nRead value: [`regs::Prio92ReadVal`]; Write value: [`regs::Prio92WriteVal`]"]
    #[inline(always)]
    pub fn prio92(&self) -> ureg::RegRef<crate::meta::Prio92, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x170 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 93 Priority\n\nRead value: [`regs::Prio93ReadVal`]; Write value: [`regs::Prio93WriteVal`]"]
    #[inline(always)]
    pub fn prio93(&self) -> ureg::RegRef<crate::meta::Prio93, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x174 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 94 Priority\n\nRead value: [`regs::Prio94ReadVal`]; Write value: [`regs::Prio94WriteVal`]"]
    #[inline(always)]
    pub fn prio94(&self) -> ureg::RegRef<crate::meta::Prio94, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x178 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 95 Priority\n\nRead value: [`regs::Prio95ReadVal`]; Write value: [`regs::Prio95WriteVal`]"]
    #[inline(always)]
    pub fn prio95(&self) -> ureg::RegRef<crate::meta::Prio95, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x17c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 96 Priority\n\nRead value: [`regs::Prio96ReadVal`]; Write value: [`regs::Prio96WriteVal`]"]
    #[inline(always)]
    pub fn prio96(&self) -> ureg::RegRef<crate::meta::Prio96, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x180 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 97 Priority\n\nRead value: [`regs::Prio97ReadVal`]; Write value: [`regs::Prio97WriteVal`]"]
    #[inline(always)]
    pub fn prio97(&self) -> ureg::RegRef<crate::meta::Prio97, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x184 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 98 Priority\n\nRead value: [`regs::Prio98ReadVal`]; Write value: [`regs::Prio98WriteVal`]"]
    #[inline(always)]
    pub fn prio98(&self) -> ureg::RegRef<crate::meta::Prio98, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x188 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 99 Priority\n\nRead value: [`regs::Prio99ReadVal`]; Write value: [`regs::Prio99WriteVal`]"]
    #[inline(always)]
    pub fn prio99(&self) -> ureg::RegRef<crate::meta::Prio99, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 100 Priority\n\nRead value: [`regs::Prio100ReadVal`]; Write value: [`regs::Prio100WriteVal`]"]
    #[inline(always)]
    pub fn prio100(&self) -> ureg::RegRef<crate::meta::Prio100, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x190 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 101 Priority\n\nRead value: [`regs::Prio101ReadVal`]; Write value: [`regs::Prio101WriteVal`]"]
    #[inline(always)]
    pub fn prio101(&self) -> ureg::RegRef<crate::meta::Prio101, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x194 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 102 Priority\n\nRead value: [`regs::Prio102ReadVal`]; Write value: [`regs::Prio102WriteVal`]"]
    #[inline(always)]
    pub fn prio102(&self) -> ureg::RegRef<crate::meta::Prio102, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x198 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 103 Priority\n\nRead value: [`regs::Prio103ReadVal`]; Write value: [`regs::Prio103WriteVal`]"]
    #[inline(always)]
    pub fn prio103(&self) -> ureg::RegRef<crate::meta::Prio103, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x19c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 104 Priority\n\nRead value: [`regs::Prio104ReadVal`]; Write value: [`regs::Prio104WriteVal`]"]
    #[inline(always)]
    pub fn prio104(&self) -> ureg::RegRef<crate::meta::Prio104, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1a0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 105 Priority\n\nRead value: [`regs::Prio105ReadVal`]; Write value: [`regs::Prio105WriteVal`]"]
    #[inline(always)]
    pub fn prio105(&self) -> ureg::RegRef<crate::meta::Prio105, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1a4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 106 Priority\n\nRead value: [`regs::Prio106ReadVal`]; Write value: [`regs::Prio106WriteVal`]"]
    #[inline(always)]
    pub fn prio106(&self) -> ureg::RegRef<crate::meta::Prio106, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1a8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 107 Priority\n\nRead value: [`regs::Prio107ReadVal`]; Write value: [`regs::Prio107WriteVal`]"]
    #[inline(always)]
    pub fn prio107(&self) -> ureg::RegRef<crate::meta::Prio107, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1ac / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 108 Priority\n\nRead value: [`regs::Prio108ReadVal`]; Write value: [`regs::Prio108WriteVal`]"]
    #[inline(always)]
    pub fn prio108(&self) -> ureg::RegRef<crate::meta::Prio108, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1b0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 109 Priority\n\nRead value: [`regs::Prio109ReadVal`]; Write value: [`regs::Prio109WriteVal`]"]
    #[inline(always)]
    pub fn prio109(&self) -> ureg::RegRef<crate::meta::Prio109, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1b4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 110 Priority\n\nRead value: [`regs::Prio110ReadVal`]; Write value: [`regs::Prio110WriteVal`]"]
    #[inline(always)]
    pub fn prio110(&self) -> ureg::RegRef<crate::meta::Prio110, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1b8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 111 Priority\n\nRead value: [`regs::Prio111ReadVal`]; Write value: [`regs::Prio111WriteVal`]"]
    #[inline(always)]
    pub fn prio111(&self) -> ureg::RegRef<crate::meta::Prio111, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1bc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 112 Priority\n\nRead value: [`regs::Prio112ReadVal`]; Write value: [`regs::Prio112WriteVal`]"]
    #[inline(always)]
    pub fn prio112(&self) -> ureg::RegRef<crate::meta::Prio112, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 113 Priority\n\nRead value: [`regs::Prio113ReadVal`]; Write value: [`regs::Prio113WriteVal`]"]
    #[inline(always)]
    pub fn prio113(&self) -> ureg::RegRef<crate::meta::Prio113, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 114 Priority\n\nRead value: [`regs::Prio114ReadVal`]; Write value: [`regs::Prio114WriteVal`]"]
    #[inline(always)]
    pub fn prio114(&self) -> ureg::RegRef<crate::meta::Prio114, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 115 Priority\n\nRead value: [`regs::Prio115ReadVal`]; Write value: [`regs::Prio115WriteVal`]"]
    #[inline(always)]
    pub fn prio115(&self) -> ureg::RegRef<crate::meta::Prio115, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1cc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 116 Priority\n\nRead value: [`regs::Prio116ReadVal`]; Write value: [`regs::Prio116WriteVal`]"]
    #[inline(always)]
    pub fn prio116(&self) -> ureg::RegRef<crate::meta::Prio116, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1d0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 117 Priority\n\nRead value: [`regs::Prio117ReadVal`]; Write value: [`regs::Prio117WriteVal`]"]
    #[inline(always)]
    pub fn prio117(&self) -> ureg::RegRef<crate::meta::Prio117, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1d4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 118 Priority\n\nRead value: [`regs::Prio118ReadVal`]; Write value: [`regs::Prio118WriteVal`]"]
    #[inline(always)]
    pub fn prio118(&self) -> ureg::RegRef<crate::meta::Prio118, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1d8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 119 Priority\n\nRead value: [`regs::Prio119ReadVal`]; Write value: [`regs::Prio119WriteVal`]"]
    #[inline(always)]
    pub fn prio119(&self) -> ureg::RegRef<crate::meta::Prio119, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1dc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 120 Priority\n\nRead value: [`regs::Prio120ReadVal`]; Write value: [`regs::Prio120WriteVal`]"]
    #[inline(always)]
    pub fn prio120(&self) -> ureg::RegRef<crate::meta::Prio120, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1e0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 121 Priority\n\nRead value: [`regs::Prio121ReadVal`]; Write value: [`regs::Prio121WriteVal`]"]
    #[inline(always)]
    pub fn prio121(&self) -> ureg::RegRef<crate::meta::Prio121, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1e4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 122 Priority\n\nRead value: [`regs::Prio122ReadVal`]; Write value: [`regs::Prio122WriteVal`]"]
    #[inline(always)]
    pub fn prio122(&self) -> ureg::RegRef<crate::meta::Prio122, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1e8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 123 Priority\n\nRead value: [`regs::Prio123ReadVal`]; Write value: [`regs::Prio123WriteVal`]"]
    #[inline(always)]
    pub fn prio123(&self) -> ureg::RegRef<crate::meta::Prio123, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1ec / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 124 Priority\n\nRead value: [`regs::Prio124ReadVal`]; Write value: [`regs::Prio124WriteVal`]"]
    #[inline(always)]
    pub fn prio124(&self) -> ureg::RegRef<crate::meta::Prio124, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1f0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 125 Priority\n\nRead value: [`regs::Prio125ReadVal`]; Write value: [`regs::Prio125WriteVal`]"]
    #[inline(always)]
    pub fn prio125(&self) -> ureg::RegRef<crate::meta::Prio125, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1f4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 126 Priority\n\nRead value: [`regs::Prio126ReadVal`]; Write value: [`regs::Prio126WriteVal`]"]
    #[inline(always)]
    pub fn prio126(&self) -> ureg::RegRef<crate::meta::Prio126, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1f8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 127 Priority\n\nRead value: [`regs::Prio127ReadVal`]; Write value: [`regs::Prio127WriteVal`]"]
    #[inline(always)]
    pub fn prio127(&self) -> ureg::RegRef<crate::meta::Prio127, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1fc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 128 Priority\n\nRead value: [`regs::Prio128ReadVal`]; Write value: [`regs::Prio128WriteVal`]"]
    #[inline(always)]
    pub fn prio128(&self) -> ureg::RegRef<crate::meta::Prio128, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x200 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 129 Priority\n\nRead value: [`regs::Prio129ReadVal`]; Write value: [`regs::Prio129WriteVal`]"]
    #[inline(always)]
    pub fn prio129(&self) -> ureg::RegRef<crate::meta::Prio129, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x204 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 130 Priority\n\nRead value: [`regs::Prio130ReadVal`]; Write value: [`regs::Prio130WriteVal`]"]
    #[inline(always)]
    pub fn prio130(&self) -> ureg::RegRef<crate::meta::Prio130, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x208 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 131 Priority\n\nRead value: [`regs::Prio131ReadVal`]; Write value: [`regs::Prio131WriteVal`]"]
    #[inline(always)]
    pub fn prio131(&self) -> ureg::RegRef<crate::meta::Prio131, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 132 Priority\n\nRead value: [`regs::Prio132ReadVal`]; Write value: [`regs::Prio132WriteVal`]"]
    #[inline(always)]
    pub fn prio132(&self) -> ureg::RegRef<crate::meta::Prio132, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x210 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 133 Priority\n\nRead value: [`regs::Prio133ReadVal`]; Write value: [`regs::Prio133WriteVal`]"]
    #[inline(always)]
    pub fn prio133(&self) -> ureg::RegRef<crate::meta::Prio133, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x214 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 134 Priority\n\nRead value: [`regs::Prio134ReadVal`]; Write value: [`regs::Prio134WriteVal`]"]
    #[inline(always)]
    pub fn prio134(&self) -> ureg::RegRef<crate::meta::Prio134, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x218 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 135 Priority\n\nRead value: [`regs::Prio135ReadVal`]; Write value: [`regs::Prio135WriteVal`]"]
    #[inline(always)]
    pub fn prio135(&self) -> ureg::RegRef<crate::meta::Prio135, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x21c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 136 Priority\n\nRead value: [`regs::Prio136ReadVal`]; Write value: [`regs::Prio136WriteVal`]"]
    #[inline(always)]
    pub fn prio136(&self) -> ureg::RegRef<crate::meta::Prio136, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x220 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 137 Priority\n\nRead value: [`regs::Prio137ReadVal`]; Write value: [`regs::Prio137WriteVal`]"]
    #[inline(always)]
    pub fn prio137(&self) -> ureg::RegRef<crate::meta::Prio137, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x224 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 138 Priority\n\nRead value: [`regs::Prio138ReadVal`]; Write value: [`regs::Prio138WriteVal`]"]
    #[inline(always)]
    pub fn prio138(&self) -> ureg::RegRef<crate::meta::Prio138, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x228 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 139 Priority\n\nRead value: [`regs::Prio139ReadVal`]; Write value: [`regs::Prio139WriteVal`]"]
    #[inline(always)]
    pub fn prio139(&self) -> ureg::RegRef<crate::meta::Prio139, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x22c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 140 Priority\n\nRead value: [`regs::Prio140ReadVal`]; Write value: [`regs::Prio140WriteVal`]"]
    #[inline(always)]
    pub fn prio140(&self) -> ureg::RegRef<crate::meta::Prio140, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x230 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 141 Priority\n\nRead value: [`regs::Prio141ReadVal`]; Write value: [`regs::Prio141WriteVal`]"]
    #[inline(always)]
    pub fn prio141(&self) -> ureg::RegRef<crate::meta::Prio141, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x234 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 142 Priority\n\nRead value: [`regs::Prio142ReadVal`]; Write value: [`regs::Prio142WriteVal`]"]
    #[inline(always)]
    pub fn prio142(&self) -> ureg::RegRef<crate::meta::Prio142, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x238 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 143 Priority\n\nRead value: [`regs::Prio143ReadVal`]; Write value: [`regs::Prio143WriteVal`]"]
    #[inline(always)]
    pub fn prio143(&self) -> ureg::RegRef<crate::meta::Prio143, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x23c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 144 Priority\n\nRead value: [`regs::Prio144ReadVal`]; Write value: [`regs::Prio144WriteVal`]"]
    #[inline(always)]
    pub fn prio144(&self) -> ureg::RegRef<crate::meta::Prio144, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x240 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 145 Priority\n\nRead value: [`regs::Prio145ReadVal`]; Write value: [`regs::Prio145WriteVal`]"]
    #[inline(always)]
    pub fn prio145(&self) -> ureg::RegRef<crate::meta::Prio145, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x244 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 146 Priority\n\nRead value: [`regs::Prio146ReadVal`]; Write value: [`regs::Prio146WriteVal`]"]
    #[inline(always)]
    pub fn prio146(&self) -> ureg::RegRef<crate::meta::Prio146, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x248 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 147 Priority\n\nRead value: [`regs::Prio147ReadVal`]; Write value: [`regs::Prio147WriteVal`]"]
    #[inline(always)]
    pub fn prio147(&self) -> ureg::RegRef<crate::meta::Prio147, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 148 Priority\n\nRead value: [`regs::Prio148ReadVal`]; Write value: [`regs::Prio148WriteVal`]"]
    #[inline(always)]
    pub fn prio148(&self) -> ureg::RegRef<crate::meta::Prio148, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x250 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 149 Priority\n\nRead value: [`regs::Prio149ReadVal`]; Write value: [`regs::Prio149WriteVal`]"]
    #[inline(always)]
    pub fn prio149(&self) -> ureg::RegRef<crate::meta::Prio149, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x254 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 150 Priority\n\nRead value: [`regs::Prio150ReadVal`]; Write value: [`regs::Prio150WriteVal`]"]
    #[inline(always)]
    pub fn prio150(&self) -> ureg::RegRef<crate::meta::Prio150, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x258 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 151 Priority\n\nRead value: [`regs::Prio151ReadVal`]; Write value: [`regs::Prio151WriteVal`]"]
    #[inline(always)]
    pub fn prio151(&self) -> ureg::RegRef<crate::meta::Prio151, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x25c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 152 Priority\n\nRead value: [`regs::Prio152ReadVal`]; Write value: [`regs::Prio152WriteVal`]"]
    #[inline(always)]
    pub fn prio152(&self) -> ureg::RegRef<crate::meta::Prio152, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x260 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 153 Priority\n\nRead value: [`regs::Prio153ReadVal`]; Write value: [`regs::Prio153WriteVal`]"]
    #[inline(always)]
    pub fn prio153(&self) -> ureg::RegRef<crate::meta::Prio153, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x264 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 154 Priority\n\nRead value: [`regs::Prio154ReadVal`]; Write value: [`regs::Prio154WriteVal`]"]
    #[inline(always)]
    pub fn prio154(&self) -> ureg::RegRef<crate::meta::Prio154, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x268 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 155 Priority\n\nRead value: [`regs::Prio155ReadVal`]; Write value: [`regs::Prio155WriteVal`]"]
    #[inline(always)]
    pub fn prio155(&self) -> ureg::RegRef<crate::meta::Prio155, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x26c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 156 Priority\n\nRead value: [`regs::Prio156ReadVal`]; Write value: [`regs::Prio156WriteVal`]"]
    #[inline(always)]
    pub fn prio156(&self) -> ureg::RegRef<crate::meta::Prio156, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x270 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 157 Priority\n\nRead value: [`regs::Prio157ReadVal`]; Write value: [`regs::Prio157WriteVal`]"]
    #[inline(always)]
    pub fn prio157(&self) -> ureg::RegRef<crate::meta::Prio157, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x274 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 158 Priority\n\nRead value: [`regs::Prio158ReadVal`]; Write value: [`regs::Prio158WriteVal`]"]
    #[inline(always)]
    pub fn prio158(&self) -> ureg::RegRef<crate::meta::Prio158, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x278 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 159 Priority\n\nRead value: [`regs::Prio159ReadVal`]; Write value: [`regs::Prio159WriteVal`]"]
    #[inline(always)]
    pub fn prio159(&self) -> ureg::RegRef<crate::meta::Prio159, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x27c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 160 Priority\n\nRead value: [`regs::Prio160ReadVal`]; Write value: [`regs::Prio160WriteVal`]"]
    #[inline(always)]
    pub fn prio160(&self) -> ureg::RegRef<crate::meta::Prio160, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x280 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 161 Priority\n\nRead value: [`regs::Prio161ReadVal`]; Write value: [`regs::Prio161WriteVal`]"]
    #[inline(always)]
    pub fn prio161(&self) -> ureg::RegRef<crate::meta::Prio161, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x284 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 162 Priority\n\nRead value: [`regs::Prio162ReadVal`]; Write value: [`regs::Prio162WriteVal`]"]
    #[inline(always)]
    pub fn prio162(&self) -> ureg::RegRef<crate::meta::Prio162, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x288 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 163 Priority\n\nRead value: [`regs::Prio163ReadVal`]; Write value: [`regs::Prio163WriteVal`]"]
    #[inline(always)]
    pub fn prio163(&self) -> ureg::RegRef<crate::meta::Prio163, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 164 Priority\n\nRead value: [`regs::Prio164ReadVal`]; Write value: [`regs::Prio164WriteVal`]"]
    #[inline(always)]
    pub fn prio164(&self) -> ureg::RegRef<crate::meta::Prio164, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x290 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 165 Priority\n\nRead value: [`regs::Prio165ReadVal`]; Write value: [`regs::Prio165WriteVal`]"]
    #[inline(always)]
    pub fn prio165(&self) -> ureg::RegRef<crate::meta::Prio165, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x294 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 166 Priority\n\nRead value: [`regs::Prio166ReadVal`]; Write value: [`regs::Prio166WriteVal`]"]
    #[inline(always)]
    pub fn prio166(&self) -> ureg::RegRef<crate::meta::Prio166, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x298 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 167 Priority\n\nRead value: [`regs::Prio167ReadVal`]; Write value: [`regs::Prio167WriteVal`]"]
    #[inline(always)]
    pub fn prio167(&self) -> ureg::RegRef<crate::meta::Prio167, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x29c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 168 Priority\n\nRead value: [`regs::Prio168ReadVal`]; Write value: [`regs::Prio168WriteVal`]"]
    #[inline(always)]
    pub fn prio168(&self) -> ureg::RegRef<crate::meta::Prio168, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2a0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 169 Priority\n\nRead value: [`regs::Prio169ReadVal`]; Write value: [`regs::Prio169WriteVal`]"]
    #[inline(always)]
    pub fn prio169(&self) -> ureg::RegRef<crate::meta::Prio169, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2a4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 170 Priority\n\nRead value: [`regs::Prio170ReadVal`]; Write value: [`regs::Prio170WriteVal`]"]
    #[inline(always)]
    pub fn prio170(&self) -> ureg::RegRef<crate::meta::Prio170, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2a8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 171 Priority\n\nRead value: [`regs::Prio171ReadVal`]; Write value: [`regs::Prio171WriteVal`]"]
    #[inline(always)]
    pub fn prio171(&self) -> ureg::RegRef<crate::meta::Prio171, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2ac / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 172 Priority\n\nRead value: [`regs::Prio172ReadVal`]; Write value: [`regs::Prio172WriteVal`]"]
    #[inline(always)]
    pub fn prio172(&self) -> ureg::RegRef<crate::meta::Prio172, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2b0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 173 Priority\n\nRead value: [`regs::Prio173ReadVal`]; Write value: [`regs::Prio173WriteVal`]"]
    #[inline(always)]
    pub fn prio173(&self) -> ureg::RegRef<crate::meta::Prio173, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2b4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 174 Priority\n\nRead value: [`regs::Prio174ReadVal`]; Write value: [`regs::Prio174WriteVal`]"]
    #[inline(always)]
    pub fn prio174(&self) -> ureg::RegRef<crate::meta::Prio174, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2b8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 175 Priority\n\nRead value: [`regs::Prio175ReadVal`]; Write value: [`regs::Prio175WriteVal`]"]
    #[inline(always)]
    pub fn prio175(&self) -> ureg::RegRef<crate::meta::Prio175, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2bc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 176 Priority\n\nRead value: [`regs::Prio176ReadVal`]; Write value: [`regs::Prio176WriteVal`]"]
    #[inline(always)]
    pub fn prio176(&self) -> ureg::RegRef<crate::meta::Prio176, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 177 Priority\n\nRead value: [`regs::Prio177ReadVal`]; Write value: [`regs::Prio177WriteVal`]"]
    #[inline(always)]
    pub fn prio177(&self) -> ureg::RegRef<crate::meta::Prio177, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 178 Priority\n\nRead value: [`regs::Prio178ReadVal`]; Write value: [`regs::Prio178WriteVal`]"]
    #[inline(always)]
    pub fn prio178(&self) -> ureg::RegRef<crate::meta::Prio178, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 179 Priority\n\nRead value: [`regs::Prio179ReadVal`]; Write value: [`regs::Prio179WriteVal`]"]
    #[inline(always)]
    pub fn prio179(&self) -> ureg::RegRef<crate::meta::Prio179, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2cc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 180 Priority\n\nRead value: [`regs::Prio180ReadVal`]; Write value: [`regs::Prio180WriteVal`]"]
    #[inline(always)]
    pub fn prio180(&self) -> ureg::RegRef<crate::meta::Prio180, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2d0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 181 Priority\n\nRead value: [`regs::Prio181ReadVal`]; Write value: [`regs::Prio181WriteVal`]"]
    #[inline(always)]
    pub fn prio181(&self) -> ureg::RegRef<crate::meta::Prio181, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2d4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 182 Priority\n\nRead value: [`regs::Prio182ReadVal`]; Write value: [`regs::Prio182WriteVal`]"]
    #[inline(always)]
    pub fn prio182(&self) -> ureg::RegRef<crate::meta::Prio182, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2d8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 183 Priority\n\nRead value: [`regs::Prio183ReadVal`]; Write value: [`regs::Prio183WriteVal`]"]
    #[inline(always)]
    pub fn prio183(&self) -> ureg::RegRef<crate::meta::Prio183, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2dc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 184 Priority\n\nRead value: [`regs::Prio184ReadVal`]; Write value: [`regs::Prio184WriteVal`]"]
    #[inline(always)]
    pub fn prio184(&self) -> ureg::RegRef<crate::meta::Prio184, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2e0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Source 185 Priority\n\nRead value: [`regs::Prio185ReadVal`]; Write value: [`regs::Prio185WriteVal`]"]
    #[inline(always)]
    pub fn prio185(&self) -> ureg::RegRef<crate::meta::Prio185, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2e4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Pending\n\nRead value: [`regs::Ip0ReadVal`]; Write value: [`regs::Ip0WriteVal`]"]
    #[inline(always)]
    pub fn ip0(&self) -> ureg::RegRef<crate::meta::Ip0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1000 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Pending\n\nRead value: [`regs::Ip1ReadVal`]; Write value: [`regs::Ip1WriteVal`]"]
    #[inline(always)]
    pub fn ip1(&self) -> ureg::RegRef<crate::meta::Ip1, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1004 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Pending\n\nRead value: [`regs::Ip2ReadVal`]; Write value: [`regs::Ip2WriteVal`]"]
    #[inline(always)]
    pub fn ip2(&self) -> ureg::RegRef<crate::meta::Ip2, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1008 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Pending\n\nRead value: [`regs::Ip3ReadVal`]; Write value: [`regs::Ip3WriteVal`]"]
    #[inline(always)]
    pub fn ip3(&self) -> ureg::RegRef<crate::meta::Ip3, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x100c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Pending\n\nRead value: [`regs::Ip4ReadVal`]; Write value: [`regs::Ip4WriteVal`]"]
    #[inline(always)]
    pub fn ip4(&self) -> ureg::RegRef<crate::meta::Ip4, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1010 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Pending\n\nRead value: [`regs::Ip5ReadVal`]; Write value: [`regs::Ip5WriteVal`]"]
    #[inline(always)]
    pub fn ip5(&self) -> ureg::RegRef<crate::meta::Ip5, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1014 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Enable for Target 0\n\nRead value: [`regs::Ie0ReadVal`]; Write value: [`regs::Ie0WriteVal`]"]
    #[inline(always)]
    pub fn ie0(&self) -> ureg::RegRef<crate::meta::Ie0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2000 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Enable for Target 0\n\nRead value: [`regs::Ie1ReadVal`]; Write value: [`regs::Ie1WriteVal`]"]
    #[inline(always)]
    pub fn ie1(&self) -> ureg::RegRef<crate::meta::Ie1, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2004 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Enable for Target 0\n\nRead value: [`regs::Ie2ReadVal`]; Write value: [`regs::Ie2WriteVal`]"]
    #[inline(always)]
    pub fn ie2(&self) -> ureg::RegRef<crate::meta::Ie2, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2008 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Enable for Target 0\n\nRead value: [`regs::Ie3ReadVal`]; Write value: [`regs::Ie3WriteVal`]"]
    #[inline(always)]
    pub fn ie3(&self) -> ureg::RegRef<crate::meta::Ie3, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x200c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Enable for Target 0\n\nRead value: [`regs::Ie4ReadVal`]; Write value: [`regs::Ie4WriteVal`]"]
    #[inline(always)]
    pub fn ie4(&self) -> ureg::RegRef<crate::meta::Ie4, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2010 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Enable for Target 0\n\nRead value: [`regs::Ie5ReadVal`]; Write value: [`regs::Ie5WriteVal`]"]
    #[inline(always)]
    pub fn ie5(&self) -> ureg::RegRef<crate::meta::Ie5, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2014 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Threshold of priority for Target 0\n\nRead value: [`regs::Threshold0ReadVal`]; Write value: [`regs::Threshold0WriteVal`]"]
    #[inline(always)]
    pub fn threshold0(&self) -> ureg::RegRef<crate::meta::Threshold0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr
                    .wrapping_add(0x200000 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Claim interrupt by read, complete interrupt by write for Target 0.\nValue read/written is interrupt ID. Reading a value of 0 means no pending interrupts.\n\nRead value: [`regs::Cc0ReadVal`]; Write value: [`regs::Cc0WriteVal`]"]
    #[inline(always)]
    pub fn cc0(&self) -> ureg::RegRef<crate::meta::Cc0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr
                    .wrapping_add(0x200004 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "msip for Hart 0.\nWrite 1 to here asserts software interrupt for Hart msip_o[0], write 0 to clear.\n\nRead value: [`regs::Msip0ReadVal`]; Write value: [`regs::Msip0WriteVal`]"]
    #[inline(always)]
    pub fn msip0(&self) -> ureg::RegRef<crate::meta::Msip0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr
                    .wrapping_add(0x4000000 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Alert Test Register.\n\nRead value: [`regs::AlertTestReadVal`]; Write value: [`regs::AlertTestWriteVal`]"]
    #[inline(always)]
    pub fn alert_test(&self) -> ureg::RegRef<crate::meta::AlertTest, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr
                    .wrapping_add(0x4004000 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
}
pub mod regs {
    #![doc = r" Types that represent the values held by registers."]
    #[derive(Clone, Copy)]
    pub struct AlertTestWriteVal(u32);
    impl AlertTestWriteVal {
        #[doc = "'Write 1 to trigger one alert event of this kind.'"]
        #[inline(always)]
        pub fn fatal_fault(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for AlertTestWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AlertTestWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AlertTestWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Cc0ReadVal(u32);
    impl Cc0ReadVal {
        #[inline(always)]
        pub fn cc0(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Cc0WriteVal {
            Cc0WriteVal(self.0)
        }
    }
    impl From<u32> for Cc0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Cc0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Cc0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Cc0WriteVal(u32);
    impl Cc0WriteVal {
        #[inline(always)]
        pub fn cc0(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
    }
    impl From<u32> for Cc0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Cc0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Cc0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ie0ReadVal(u32);
    impl Ie0ReadVal {
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e12(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e13(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e14(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e15(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e16(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e17(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e18(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e19(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e20(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e21(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e22(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e23(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e24(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e25(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e26(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e27(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e28(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e29(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e30(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e31(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Ie0WriteVal {
            Ie0WriteVal(self.0)
        }
    }
    impl From<u32> for Ie0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ie0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ie0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ie0WriteVal(u32);
    impl Ie0WriteVal {
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e11(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e12(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e13(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e14(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e15(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e16(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e17(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e18(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e19(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e20(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e21(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e22(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e23(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e24(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e25(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e26(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e27(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e28(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e29(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e30(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e31(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for Ie0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ie0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Ie0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ie1ReadVal(u32);
    impl Ie1ReadVal {
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e32(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e33(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e34(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e35(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e36(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e37(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e38(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e39(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e40(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e41(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e42(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e43(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e44(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e45(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e46(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e47(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e48(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e49(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e50(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e51(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e52(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e53(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e54(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e55(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e56(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e57(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e58(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e59(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e60(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e61(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e62(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e63(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Ie1WriteVal {
            Ie1WriteVal(self.0)
        }
    }
    impl From<u32> for Ie1ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ie1ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ie1ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ie1WriteVal(u32);
    impl Ie1WriteVal {
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e32(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e33(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e34(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e35(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e36(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e37(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e38(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e39(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e40(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e41(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e42(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e43(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e44(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e45(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e46(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e47(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e48(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e49(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e50(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e51(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e52(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e53(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e54(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e55(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e56(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e57(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e58(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e59(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e60(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e61(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e62(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e63(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for Ie1WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ie1WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Ie1WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ie2ReadVal(u32);
    impl Ie2ReadVal {
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e64(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e65(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e66(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e67(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e68(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e69(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e70(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e71(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e72(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e73(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e74(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e75(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e76(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e77(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e78(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e79(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e80(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e81(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e82(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e83(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e84(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e85(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e86(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e87(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e88(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e89(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e90(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e91(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e92(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e93(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e94(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e95(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Ie2WriteVal {
            Ie2WriteVal(self.0)
        }
    }
    impl From<u32> for Ie2ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ie2ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ie2ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ie2WriteVal(u32);
    impl Ie2WriteVal {
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e64(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e65(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e66(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e67(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e68(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e69(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e70(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e71(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e72(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e73(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e74(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e75(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e76(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e77(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e78(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e79(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e80(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e81(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e82(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e83(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e84(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e85(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e86(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e87(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e88(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e89(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e90(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e91(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e92(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e93(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e94(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e95(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for Ie2WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ie2WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Ie2WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ie3ReadVal(u32);
    impl Ie3ReadVal {
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e96(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e97(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e98(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e99(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e100(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e101(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e102(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e103(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e104(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e105(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e106(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e107(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e108(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e109(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e110(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e111(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e112(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e113(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e114(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e115(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e116(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e117(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e118(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e119(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e120(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e121(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e122(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e123(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e124(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e125(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e126(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e127(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Ie3WriteVal {
            Ie3WriteVal(self.0)
        }
    }
    impl From<u32> for Ie3ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ie3ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ie3ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ie3WriteVal(u32);
    impl Ie3WriteVal {
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e96(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e97(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e98(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e99(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e100(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e101(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e102(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e103(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e104(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e105(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e106(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e107(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e108(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e109(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e110(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e111(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e112(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e113(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e114(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e115(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e116(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e117(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e118(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e119(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e120(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e121(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e122(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e123(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e124(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e125(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e126(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e127(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for Ie3WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ie3WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Ie3WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ie4ReadVal(u32);
    impl Ie4ReadVal {
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e128(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e129(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e130(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e131(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e132(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e133(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e134(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e135(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e136(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e137(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e138(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e139(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e140(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e141(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e142(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e143(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e144(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e145(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e146(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e147(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e148(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e149(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e150(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e151(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e152(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e153(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e154(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e155(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e156(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e157(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e158(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e159(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Ie4WriteVal {
            Ie4WriteVal(self.0)
        }
    }
    impl From<u32> for Ie4ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ie4ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ie4ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ie4WriteVal(u32);
    impl Ie4WriteVal {
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e128(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e129(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e130(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e131(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e132(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e133(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e134(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e135(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e136(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e137(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e138(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e139(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e140(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e141(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e142(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e143(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e144(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e145(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e146(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e147(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e148(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e149(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e150(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e151(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e152(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e153(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e154(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e155(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e156(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e157(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e158(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e159(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for Ie4WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ie4WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Ie4WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ie5ReadVal(u32);
    impl Ie5ReadVal {
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e160(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e161(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e162(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e163(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e164(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e165(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e166(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e167(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e168(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e169(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e170(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e171(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e172(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e173(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e174(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e175(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e176(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e177(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e178(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e179(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e180(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e181(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e182(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e183(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e184(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e185(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Ie5WriteVal {
            Ie5WriteVal(self.0)
        }
    }
    impl From<u32> for Ie5ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ie5ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ie5ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ie5WriteVal(u32);
    impl Ie5WriteVal {
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e160(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e161(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e162(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e163(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e164(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e165(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e166(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e167(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e168(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e169(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e170(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e171(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e172(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e173(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e174(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e175(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e176(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e177(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e178(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e179(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e180(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e181(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e182(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e183(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e184(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "Interrupt Enable of Source"]
        #[inline(always)]
        pub fn e185(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
    }
    impl From<u32> for Ie5WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ie5WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Ie5WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ip0ReadVal(u32);
    impl Ip0ReadVal {
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p12(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p13(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p14(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p15(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p16(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p17(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p18(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p19(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p20(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p21(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p22(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p23(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p24(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p25(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p26(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p27(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p28(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p29(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p30(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p31(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
    }
    impl From<u32> for Ip0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ip0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ip0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ip1ReadVal(u32);
    impl Ip1ReadVal {
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p32(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p33(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p34(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p35(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p36(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p37(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p38(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p39(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p40(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p41(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p42(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p43(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p44(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p45(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p46(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p47(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p48(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p49(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p50(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p51(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p52(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p53(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p54(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p55(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p56(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p57(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p58(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p59(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p60(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p61(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p62(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p63(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
    }
    impl From<u32> for Ip1ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ip1ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ip1ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ip2ReadVal(u32);
    impl Ip2ReadVal {
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p64(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p65(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p66(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p67(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p68(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p69(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p70(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p71(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p72(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p73(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p74(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p75(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p76(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p77(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p78(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p79(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p80(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p81(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p82(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p83(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p84(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p85(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p86(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p87(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p88(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p89(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p90(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p91(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p92(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p93(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p94(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p95(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
    }
    impl From<u32> for Ip2ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ip2ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ip2ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ip3ReadVal(u32);
    impl Ip3ReadVal {
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p96(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p97(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p98(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p99(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p100(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p101(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p102(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p103(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p104(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p105(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p106(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p107(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p108(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p109(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p110(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p111(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p112(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p113(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p114(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p115(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p116(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p117(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p118(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p119(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p120(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p121(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p122(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p123(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p124(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p125(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p126(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p127(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
    }
    impl From<u32> for Ip3ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ip3ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ip3ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ip4ReadVal(u32);
    impl Ip4ReadVal {
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p128(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p129(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p130(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p131(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p132(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p133(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p134(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p135(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p136(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p137(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p138(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p139(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p140(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p141(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p142(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p143(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p144(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p145(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p146(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p147(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p148(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p149(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p150(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p151(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p152(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p153(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p154(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p155(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p156(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p157(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p158(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p159(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
    }
    impl From<u32> for Ip4ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ip4ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ip4ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Ip5ReadVal(u32);
    impl Ip5ReadVal {
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p160(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p161(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p162(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p163(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p164(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p165(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p166(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p167(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p168(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p169(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p170(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p171(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p172(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p173(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p174(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p175(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p176(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p177(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p178(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p179(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p180(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p181(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p182(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p183(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p184(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Interrupt Pending of Source"]
        #[inline(always)]
        pub fn p185(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
    }
    impl From<u32> for Ip5ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Ip5ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Ip5ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Msip0ReadVal(u32);
    impl Msip0ReadVal {
        #[doc = "Software Interrupt Pending register"]
        #[inline(always)]
        pub fn msip0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Msip0WriteVal {
            Msip0WriteVal(self.0)
        }
    }
    impl From<u32> for Msip0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Msip0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Msip0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Msip0WriteVal(u32);
    impl Msip0WriteVal {
        #[doc = "Software Interrupt Pending register"]
        #[inline(always)]
        pub fn msip0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for Msip0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Msip0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Msip0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio0ReadVal(u32);
    impl Prio0ReadVal {
        #[inline(always)]
        pub fn prio0(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio0WriteVal {
            Prio0WriteVal(self.0)
        }
    }
    impl From<u32> for Prio0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio0WriteVal(u32);
    impl Prio0WriteVal {
        #[inline(always)]
        pub fn prio0(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio1ReadVal(u32);
    impl Prio1ReadVal {
        #[inline(always)]
        pub fn prio1(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio1WriteVal {
            Prio1WriteVal(self.0)
        }
    }
    impl From<u32> for Prio1ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio1ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio1ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio1WriteVal(u32);
    impl Prio1WriteVal {
        #[inline(always)]
        pub fn prio1(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio1WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio1WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio1WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio10ReadVal(u32);
    impl Prio10ReadVal {
        #[inline(always)]
        pub fn prio10(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio10WriteVal {
            Prio10WriteVal(self.0)
        }
    }
    impl From<u32> for Prio10ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio10ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio10ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio10WriteVal(u32);
    impl Prio10WriteVal {
        #[inline(always)]
        pub fn prio10(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio10WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio10WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio10WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio100ReadVal(u32);
    impl Prio100ReadVal {
        #[inline(always)]
        pub fn prio100(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio100WriteVal {
            Prio100WriteVal(self.0)
        }
    }
    impl From<u32> for Prio100ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio100ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio100ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio100WriteVal(u32);
    impl Prio100WriteVal {
        #[inline(always)]
        pub fn prio100(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio100WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio100WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio100WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio101ReadVal(u32);
    impl Prio101ReadVal {
        #[inline(always)]
        pub fn prio101(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio101WriteVal {
            Prio101WriteVal(self.0)
        }
    }
    impl From<u32> for Prio101ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio101ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio101ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio101WriteVal(u32);
    impl Prio101WriteVal {
        #[inline(always)]
        pub fn prio101(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio101WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio101WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio101WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio102ReadVal(u32);
    impl Prio102ReadVal {
        #[inline(always)]
        pub fn prio102(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio102WriteVal {
            Prio102WriteVal(self.0)
        }
    }
    impl From<u32> for Prio102ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio102ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio102ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio102WriteVal(u32);
    impl Prio102WriteVal {
        #[inline(always)]
        pub fn prio102(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio102WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio102WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio102WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio103ReadVal(u32);
    impl Prio103ReadVal {
        #[inline(always)]
        pub fn prio103(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio103WriteVal {
            Prio103WriteVal(self.0)
        }
    }
    impl From<u32> for Prio103ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio103ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio103ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio103WriteVal(u32);
    impl Prio103WriteVal {
        #[inline(always)]
        pub fn prio103(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio103WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio103WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio103WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio104ReadVal(u32);
    impl Prio104ReadVal {
        #[inline(always)]
        pub fn prio104(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio104WriteVal {
            Prio104WriteVal(self.0)
        }
    }
    impl From<u32> for Prio104ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio104ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio104ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio104WriteVal(u32);
    impl Prio104WriteVal {
        #[inline(always)]
        pub fn prio104(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio104WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio104WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio104WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio105ReadVal(u32);
    impl Prio105ReadVal {
        #[inline(always)]
        pub fn prio105(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio105WriteVal {
            Prio105WriteVal(self.0)
        }
    }
    impl From<u32> for Prio105ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio105ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio105ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio105WriteVal(u32);
    impl Prio105WriteVal {
        #[inline(always)]
        pub fn prio105(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio105WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio105WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio105WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio106ReadVal(u32);
    impl Prio106ReadVal {
        #[inline(always)]
        pub fn prio106(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio106WriteVal {
            Prio106WriteVal(self.0)
        }
    }
    impl From<u32> for Prio106ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio106ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio106ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio106WriteVal(u32);
    impl Prio106WriteVal {
        #[inline(always)]
        pub fn prio106(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio106WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio106WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio106WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio107ReadVal(u32);
    impl Prio107ReadVal {
        #[inline(always)]
        pub fn prio107(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio107WriteVal {
            Prio107WriteVal(self.0)
        }
    }
    impl From<u32> for Prio107ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio107ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio107ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio107WriteVal(u32);
    impl Prio107WriteVal {
        #[inline(always)]
        pub fn prio107(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio107WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio107WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio107WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio108ReadVal(u32);
    impl Prio108ReadVal {
        #[inline(always)]
        pub fn prio108(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio108WriteVal {
            Prio108WriteVal(self.0)
        }
    }
    impl From<u32> for Prio108ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio108ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio108ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio108WriteVal(u32);
    impl Prio108WriteVal {
        #[inline(always)]
        pub fn prio108(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio108WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio108WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio108WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio109ReadVal(u32);
    impl Prio109ReadVal {
        #[inline(always)]
        pub fn prio109(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio109WriteVal {
            Prio109WriteVal(self.0)
        }
    }
    impl From<u32> for Prio109ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio109ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio109ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio109WriteVal(u32);
    impl Prio109WriteVal {
        #[inline(always)]
        pub fn prio109(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio109WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio109WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio109WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio11ReadVal(u32);
    impl Prio11ReadVal {
        #[inline(always)]
        pub fn prio11(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio11WriteVal {
            Prio11WriteVal(self.0)
        }
    }
    impl From<u32> for Prio11ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio11ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio11ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio11WriteVal(u32);
    impl Prio11WriteVal {
        #[inline(always)]
        pub fn prio11(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio11WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio11WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio11WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio110ReadVal(u32);
    impl Prio110ReadVal {
        #[inline(always)]
        pub fn prio110(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio110WriteVal {
            Prio110WriteVal(self.0)
        }
    }
    impl From<u32> for Prio110ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio110ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio110ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio110WriteVal(u32);
    impl Prio110WriteVal {
        #[inline(always)]
        pub fn prio110(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio110WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio110WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio110WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio111ReadVal(u32);
    impl Prio111ReadVal {
        #[inline(always)]
        pub fn prio111(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio111WriteVal {
            Prio111WriteVal(self.0)
        }
    }
    impl From<u32> for Prio111ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio111ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio111ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio111WriteVal(u32);
    impl Prio111WriteVal {
        #[inline(always)]
        pub fn prio111(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio111WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio111WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio111WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio112ReadVal(u32);
    impl Prio112ReadVal {
        #[inline(always)]
        pub fn prio112(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio112WriteVal {
            Prio112WriteVal(self.0)
        }
    }
    impl From<u32> for Prio112ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio112ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio112ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio112WriteVal(u32);
    impl Prio112WriteVal {
        #[inline(always)]
        pub fn prio112(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio112WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio112WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio112WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio113ReadVal(u32);
    impl Prio113ReadVal {
        #[inline(always)]
        pub fn prio113(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio113WriteVal {
            Prio113WriteVal(self.0)
        }
    }
    impl From<u32> for Prio113ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio113ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio113ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio113WriteVal(u32);
    impl Prio113WriteVal {
        #[inline(always)]
        pub fn prio113(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio113WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio113WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio113WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio114ReadVal(u32);
    impl Prio114ReadVal {
        #[inline(always)]
        pub fn prio114(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio114WriteVal {
            Prio114WriteVal(self.0)
        }
    }
    impl From<u32> for Prio114ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio114ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio114ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio114WriteVal(u32);
    impl Prio114WriteVal {
        #[inline(always)]
        pub fn prio114(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio114WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio114WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio114WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio115ReadVal(u32);
    impl Prio115ReadVal {
        #[inline(always)]
        pub fn prio115(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio115WriteVal {
            Prio115WriteVal(self.0)
        }
    }
    impl From<u32> for Prio115ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio115ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio115ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio115WriteVal(u32);
    impl Prio115WriteVal {
        #[inline(always)]
        pub fn prio115(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio115WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio115WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio115WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio116ReadVal(u32);
    impl Prio116ReadVal {
        #[inline(always)]
        pub fn prio116(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio116WriteVal {
            Prio116WriteVal(self.0)
        }
    }
    impl From<u32> for Prio116ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio116ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio116ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio116WriteVal(u32);
    impl Prio116WriteVal {
        #[inline(always)]
        pub fn prio116(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio116WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio116WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio116WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio117ReadVal(u32);
    impl Prio117ReadVal {
        #[inline(always)]
        pub fn prio117(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio117WriteVal {
            Prio117WriteVal(self.0)
        }
    }
    impl From<u32> for Prio117ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio117ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio117ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio117WriteVal(u32);
    impl Prio117WriteVal {
        #[inline(always)]
        pub fn prio117(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio117WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio117WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio117WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio118ReadVal(u32);
    impl Prio118ReadVal {
        #[inline(always)]
        pub fn prio118(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio118WriteVal {
            Prio118WriteVal(self.0)
        }
    }
    impl From<u32> for Prio118ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio118ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio118ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio118WriteVal(u32);
    impl Prio118WriteVal {
        #[inline(always)]
        pub fn prio118(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio118WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio118WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio118WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio119ReadVal(u32);
    impl Prio119ReadVal {
        #[inline(always)]
        pub fn prio119(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio119WriteVal {
            Prio119WriteVal(self.0)
        }
    }
    impl From<u32> for Prio119ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio119ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio119ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio119WriteVal(u32);
    impl Prio119WriteVal {
        #[inline(always)]
        pub fn prio119(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio119WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio119WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio119WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio12ReadVal(u32);
    impl Prio12ReadVal {
        #[inline(always)]
        pub fn prio12(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio12WriteVal {
            Prio12WriteVal(self.0)
        }
    }
    impl From<u32> for Prio12ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio12ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio12ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio12WriteVal(u32);
    impl Prio12WriteVal {
        #[inline(always)]
        pub fn prio12(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio12WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio12WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio12WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio120ReadVal(u32);
    impl Prio120ReadVal {
        #[inline(always)]
        pub fn prio120(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio120WriteVal {
            Prio120WriteVal(self.0)
        }
    }
    impl From<u32> for Prio120ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio120ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio120ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio120WriteVal(u32);
    impl Prio120WriteVal {
        #[inline(always)]
        pub fn prio120(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio120WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio120WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio120WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio121ReadVal(u32);
    impl Prio121ReadVal {
        #[inline(always)]
        pub fn prio121(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio121WriteVal {
            Prio121WriteVal(self.0)
        }
    }
    impl From<u32> for Prio121ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio121ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio121ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio121WriteVal(u32);
    impl Prio121WriteVal {
        #[inline(always)]
        pub fn prio121(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio121WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio121WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio121WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio122ReadVal(u32);
    impl Prio122ReadVal {
        #[inline(always)]
        pub fn prio122(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio122WriteVal {
            Prio122WriteVal(self.0)
        }
    }
    impl From<u32> for Prio122ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio122ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio122ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio122WriteVal(u32);
    impl Prio122WriteVal {
        #[inline(always)]
        pub fn prio122(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio122WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio122WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio122WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio123ReadVal(u32);
    impl Prio123ReadVal {
        #[inline(always)]
        pub fn prio123(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio123WriteVal {
            Prio123WriteVal(self.0)
        }
    }
    impl From<u32> for Prio123ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio123ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio123ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio123WriteVal(u32);
    impl Prio123WriteVal {
        #[inline(always)]
        pub fn prio123(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio123WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio123WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio123WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio124ReadVal(u32);
    impl Prio124ReadVal {
        #[inline(always)]
        pub fn prio124(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio124WriteVal {
            Prio124WriteVal(self.0)
        }
    }
    impl From<u32> for Prio124ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio124ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio124ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio124WriteVal(u32);
    impl Prio124WriteVal {
        #[inline(always)]
        pub fn prio124(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio124WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio124WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio124WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio125ReadVal(u32);
    impl Prio125ReadVal {
        #[inline(always)]
        pub fn prio125(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio125WriteVal {
            Prio125WriteVal(self.0)
        }
    }
    impl From<u32> for Prio125ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio125ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio125ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio125WriteVal(u32);
    impl Prio125WriteVal {
        #[inline(always)]
        pub fn prio125(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio125WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio125WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio125WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio126ReadVal(u32);
    impl Prio126ReadVal {
        #[inline(always)]
        pub fn prio126(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio126WriteVal {
            Prio126WriteVal(self.0)
        }
    }
    impl From<u32> for Prio126ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio126ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio126ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio126WriteVal(u32);
    impl Prio126WriteVal {
        #[inline(always)]
        pub fn prio126(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio126WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio126WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio126WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio127ReadVal(u32);
    impl Prio127ReadVal {
        #[inline(always)]
        pub fn prio127(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio127WriteVal {
            Prio127WriteVal(self.0)
        }
    }
    impl From<u32> for Prio127ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio127ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio127ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio127WriteVal(u32);
    impl Prio127WriteVal {
        #[inline(always)]
        pub fn prio127(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio127WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio127WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio127WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio128ReadVal(u32);
    impl Prio128ReadVal {
        #[inline(always)]
        pub fn prio128(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio128WriteVal {
            Prio128WriteVal(self.0)
        }
    }
    impl From<u32> for Prio128ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio128ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio128ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio128WriteVal(u32);
    impl Prio128WriteVal {
        #[inline(always)]
        pub fn prio128(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio128WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio128WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio128WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio129ReadVal(u32);
    impl Prio129ReadVal {
        #[inline(always)]
        pub fn prio129(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio129WriteVal {
            Prio129WriteVal(self.0)
        }
    }
    impl From<u32> for Prio129ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio129ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio129ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio129WriteVal(u32);
    impl Prio129WriteVal {
        #[inline(always)]
        pub fn prio129(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio129WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio129WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio129WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio13ReadVal(u32);
    impl Prio13ReadVal {
        #[inline(always)]
        pub fn prio13(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio13WriteVal {
            Prio13WriteVal(self.0)
        }
    }
    impl From<u32> for Prio13ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio13ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio13ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio13WriteVal(u32);
    impl Prio13WriteVal {
        #[inline(always)]
        pub fn prio13(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio13WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio13WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio13WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio130ReadVal(u32);
    impl Prio130ReadVal {
        #[inline(always)]
        pub fn prio130(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio130WriteVal {
            Prio130WriteVal(self.0)
        }
    }
    impl From<u32> for Prio130ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio130ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio130ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio130WriteVal(u32);
    impl Prio130WriteVal {
        #[inline(always)]
        pub fn prio130(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio130WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio130WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio130WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio131ReadVal(u32);
    impl Prio131ReadVal {
        #[inline(always)]
        pub fn prio131(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio131WriteVal {
            Prio131WriteVal(self.0)
        }
    }
    impl From<u32> for Prio131ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio131ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio131ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio131WriteVal(u32);
    impl Prio131WriteVal {
        #[inline(always)]
        pub fn prio131(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio131WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio131WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio131WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio132ReadVal(u32);
    impl Prio132ReadVal {
        #[inline(always)]
        pub fn prio132(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio132WriteVal {
            Prio132WriteVal(self.0)
        }
    }
    impl From<u32> for Prio132ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio132ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio132ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio132WriteVal(u32);
    impl Prio132WriteVal {
        #[inline(always)]
        pub fn prio132(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio132WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio132WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio132WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio133ReadVal(u32);
    impl Prio133ReadVal {
        #[inline(always)]
        pub fn prio133(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio133WriteVal {
            Prio133WriteVal(self.0)
        }
    }
    impl From<u32> for Prio133ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio133ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio133ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio133WriteVal(u32);
    impl Prio133WriteVal {
        #[inline(always)]
        pub fn prio133(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio133WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio133WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio133WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio134ReadVal(u32);
    impl Prio134ReadVal {
        #[inline(always)]
        pub fn prio134(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio134WriteVal {
            Prio134WriteVal(self.0)
        }
    }
    impl From<u32> for Prio134ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio134ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio134ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio134WriteVal(u32);
    impl Prio134WriteVal {
        #[inline(always)]
        pub fn prio134(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio134WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio134WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio134WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio135ReadVal(u32);
    impl Prio135ReadVal {
        #[inline(always)]
        pub fn prio135(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio135WriteVal {
            Prio135WriteVal(self.0)
        }
    }
    impl From<u32> for Prio135ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio135ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio135ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio135WriteVal(u32);
    impl Prio135WriteVal {
        #[inline(always)]
        pub fn prio135(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio135WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio135WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio135WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio136ReadVal(u32);
    impl Prio136ReadVal {
        #[inline(always)]
        pub fn prio136(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio136WriteVal {
            Prio136WriteVal(self.0)
        }
    }
    impl From<u32> for Prio136ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio136ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio136ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio136WriteVal(u32);
    impl Prio136WriteVal {
        #[inline(always)]
        pub fn prio136(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio136WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio136WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio136WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio137ReadVal(u32);
    impl Prio137ReadVal {
        #[inline(always)]
        pub fn prio137(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio137WriteVal {
            Prio137WriteVal(self.0)
        }
    }
    impl From<u32> for Prio137ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio137ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio137ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio137WriteVal(u32);
    impl Prio137WriteVal {
        #[inline(always)]
        pub fn prio137(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio137WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio137WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio137WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio138ReadVal(u32);
    impl Prio138ReadVal {
        #[inline(always)]
        pub fn prio138(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio138WriteVal {
            Prio138WriteVal(self.0)
        }
    }
    impl From<u32> for Prio138ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio138ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio138ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio138WriteVal(u32);
    impl Prio138WriteVal {
        #[inline(always)]
        pub fn prio138(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio138WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio138WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio138WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio139ReadVal(u32);
    impl Prio139ReadVal {
        #[inline(always)]
        pub fn prio139(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio139WriteVal {
            Prio139WriteVal(self.0)
        }
    }
    impl From<u32> for Prio139ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio139ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio139ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio139WriteVal(u32);
    impl Prio139WriteVal {
        #[inline(always)]
        pub fn prio139(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio139WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio139WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio139WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio14ReadVal(u32);
    impl Prio14ReadVal {
        #[inline(always)]
        pub fn prio14(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio14WriteVal {
            Prio14WriteVal(self.0)
        }
    }
    impl From<u32> for Prio14ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio14ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio14ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio14WriteVal(u32);
    impl Prio14WriteVal {
        #[inline(always)]
        pub fn prio14(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio14WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio14WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio14WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio140ReadVal(u32);
    impl Prio140ReadVal {
        #[inline(always)]
        pub fn prio140(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio140WriteVal {
            Prio140WriteVal(self.0)
        }
    }
    impl From<u32> for Prio140ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio140ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio140ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio140WriteVal(u32);
    impl Prio140WriteVal {
        #[inline(always)]
        pub fn prio140(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio140WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio140WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio140WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio141ReadVal(u32);
    impl Prio141ReadVal {
        #[inline(always)]
        pub fn prio141(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio141WriteVal {
            Prio141WriteVal(self.0)
        }
    }
    impl From<u32> for Prio141ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio141ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio141ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio141WriteVal(u32);
    impl Prio141WriteVal {
        #[inline(always)]
        pub fn prio141(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio141WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio141WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio141WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio142ReadVal(u32);
    impl Prio142ReadVal {
        #[inline(always)]
        pub fn prio142(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio142WriteVal {
            Prio142WriteVal(self.0)
        }
    }
    impl From<u32> for Prio142ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio142ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio142ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio142WriteVal(u32);
    impl Prio142WriteVal {
        #[inline(always)]
        pub fn prio142(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio142WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio142WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio142WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio143ReadVal(u32);
    impl Prio143ReadVal {
        #[inline(always)]
        pub fn prio143(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio143WriteVal {
            Prio143WriteVal(self.0)
        }
    }
    impl From<u32> for Prio143ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio143ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio143ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio143WriteVal(u32);
    impl Prio143WriteVal {
        #[inline(always)]
        pub fn prio143(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio143WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio143WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio143WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio144ReadVal(u32);
    impl Prio144ReadVal {
        #[inline(always)]
        pub fn prio144(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio144WriteVal {
            Prio144WriteVal(self.0)
        }
    }
    impl From<u32> for Prio144ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio144ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio144ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio144WriteVal(u32);
    impl Prio144WriteVal {
        #[inline(always)]
        pub fn prio144(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio144WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio144WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio144WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio145ReadVal(u32);
    impl Prio145ReadVal {
        #[inline(always)]
        pub fn prio145(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio145WriteVal {
            Prio145WriteVal(self.0)
        }
    }
    impl From<u32> for Prio145ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio145ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio145ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio145WriteVal(u32);
    impl Prio145WriteVal {
        #[inline(always)]
        pub fn prio145(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio145WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio145WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio145WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio146ReadVal(u32);
    impl Prio146ReadVal {
        #[inline(always)]
        pub fn prio146(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio146WriteVal {
            Prio146WriteVal(self.0)
        }
    }
    impl From<u32> for Prio146ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio146ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio146ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio146WriteVal(u32);
    impl Prio146WriteVal {
        #[inline(always)]
        pub fn prio146(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio146WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio146WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio146WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio147ReadVal(u32);
    impl Prio147ReadVal {
        #[inline(always)]
        pub fn prio147(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio147WriteVal {
            Prio147WriteVal(self.0)
        }
    }
    impl From<u32> for Prio147ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio147ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio147ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio147WriteVal(u32);
    impl Prio147WriteVal {
        #[inline(always)]
        pub fn prio147(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio147WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio147WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio147WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio148ReadVal(u32);
    impl Prio148ReadVal {
        #[inline(always)]
        pub fn prio148(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio148WriteVal {
            Prio148WriteVal(self.0)
        }
    }
    impl From<u32> for Prio148ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio148ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio148ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio148WriteVal(u32);
    impl Prio148WriteVal {
        #[inline(always)]
        pub fn prio148(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio148WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio148WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio148WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio149ReadVal(u32);
    impl Prio149ReadVal {
        #[inline(always)]
        pub fn prio149(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio149WriteVal {
            Prio149WriteVal(self.0)
        }
    }
    impl From<u32> for Prio149ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio149ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio149ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio149WriteVal(u32);
    impl Prio149WriteVal {
        #[inline(always)]
        pub fn prio149(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio149WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio149WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio149WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio15ReadVal(u32);
    impl Prio15ReadVal {
        #[inline(always)]
        pub fn prio15(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio15WriteVal {
            Prio15WriteVal(self.0)
        }
    }
    impl From<u32> for Prio15ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio15ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio15ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio15WriteVal(u32);
    impl Prio15WriteVal {
        #[inline(always)]
        pub fn prio15(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio15WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio15WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio15WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio150ReadVal(u32);
    impl Prio150ReadVal {
        #[inline(always)]
        pub fn prio150(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio150WriteVal {
            Prio150WriteVal(self.0)
        }
    }
    impl From<u32> for Prio150ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio150ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio150ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio150WriteVal(u32);
    impl Prio150WriteVal {
        #[inline(always)]
        pub fn prio150(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio150WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio150WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio150WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio151ReadVal(u32);
    impl Prio151ReadVal {
        #[inline(always)]
        pub fn prio151(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio151WriteVal {
            Prio151WriteVal(self.0)
        }
    }
    impl From<u32> for Prio151ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio151ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio151ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio151WriteVal(u32);
    impl Prio151WriteVal {
        #[inline(always)]
        pub fn prio151(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio151WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio151WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio151WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio152ReadVal(u32);
    impl Prio152ReadVal {
        #[inline(always)]
        pub fn prio152(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio152WriteVal {
            Prio152WriteVal(self.0)
        }
    }
    impl From<u32> for Prio152ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio152ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio152ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio152WriteVal(u32);
    impl Prio152WriteVal {
        #[inline(always)]
        pub fn prio152(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio152WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio152WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio152WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio153ReadVal(u32);
    impl Prio153ReadVal {
        #[inline(always)]
        pub fn prio153(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio153WriteVal {
            Prio153WriteVal(self.0)
        }
    }
    impl From<u32> for Prio153ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio153ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio153ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio153WriteVal(u32);
    impl Prio153WriteVal {
        #[inline(always)]
        pub fn prio153(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio153WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio153WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio153WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio154ReadVal(u32);
    impl Prio154ReadVal {
        #[inline(always)]
        pub fn prio154(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio154WriteVal {
            Prio154WriteVal(self.0)
        }
    }
    impl From<u32> for Prio154ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio154ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio154ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio154WriteVal(u32);
    impl Prio154WriteVal {
        #[inline(always)]
        pub fn prio154(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio154WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio154WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio154WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio155ReadVal(u32);
    impl Prio155ReadVal {
        #[inline(always)]
        pub fn prio155(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio155WriteVal {
            Prio155WriteVal(self.0)
        }
    }
    impl From<u32> for Prio155ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio155ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio155ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio155WriteVal(u32);
    impl Prio155WriteVal {
        #[inline(always)]
        pub fn prio155(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio155WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio155WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio155WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio156ReadVal(u32);
    impl Prio156ReadVal {
        #[inline(always)]
        pub fn prio156(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio156WriteVal {
            Prio156WriteVal(self.0)
        }
    }
    impl From<u32> for Prio156ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio156ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio156ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio156WriteVal(u32);
    impl Prio156WriteVal {
        #[inline(always)]
        pub fn prio156(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio156WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio156WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio156WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio157ReadVal(u32);
    impl Prio157ReadVal {
        #[inline(always)]
        pub fn prio157(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio157WriteVal {
            Prio157WriteVal(self.0)
        }
    }
    impl From<u32> for Prio157ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio157ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio157ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio157WriteVal(u32);
    impl Prio157WriteVal {
        #[inline(always)]
        pub fn prio157(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio157WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio157WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio157WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio158ReadVal(u32);
    impl Prio158ReadVal {
        #[inline(always)]
        pub fn prio158(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio158WriteVal {
            Prio158WriteVal(self.0)
        }
    }
    impl From<u32> for Prio158ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio158ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio158ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio158WriteVal(u32);
    impl Prio158WriteVal {
        #[inline(always)]
        pub fn prio158(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio158WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio158WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio158WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio159ReadVal(u32);
    impl Prio159ReadVal {
        #[inline(always)]
        pub fn prio159(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio159WriteVal {
            Prio159WriteVal(self.0)
        }
    }
    impl From<u32> for Prio159ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio159ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio159ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio159WriteVal(u32);
    impl Prio159WriteVal {
        #[inline(always)]
        pub fn prio159(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio159WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio159WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio159WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio16ReadVal(u32);
    impl Prio16ReadVal {
        #[inline(always)]
        pub fn prio16(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio16WriteVal {
            Prio16WriteVal(self.0)
        }
    }
    impl From<u32> for Prio16ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio16ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio16ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio16WriteVal(u32);
    impl Prio16WriteVal {
        #[inline(always)]
        pub fn prio16(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio16WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio16WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio16WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio160ReadVal(u32);
    impl Prio160ReadVal {
        #[inline(always)]
        pub fn prio160(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio160WriteVal {
            Prio160WriteVal(self.0)
        }
    }
    impl From<u32> for Prio160ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio160ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio160ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio160WriteVal(u32);
    impl Prio160WriteVal {
        #[inline(always)]
        pub fn prio160(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio160WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio160WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio160WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio161ReadVal(u32);
    impl Prio161ReadVal {
        #[inline(always)]
        pub fn prio161(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio161WriteVal {
            Prio161WriteVal(self.0)
        }
    }
    impl From<u32> for Prio161ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio161ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio161ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio161WriteVal(u32);
    impl Prio161WriteVal {
        #[inline(always)]
        pub fn prio161(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio161WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio161WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio161WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio162ReadVal(u32);
    impl Prio162ReadVal {
        #[inline(always)]
        pub fn prio162(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio162WriteVal {
            Prio162WriteVal(self.0)
        }
    }
    impl From<u32> for Prio162ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio162ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio162ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio162WriteVal(u32);
    impl Prio162WriteVal {
        #[inline(always)]
        pub fn prio162(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio162WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio162WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio162WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio163ReadVal(u32);
    impl Prio163ReadVal {
        #[inline(always)]
        pub fn prio163(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio163WriteVal {
            Prio163WriteVal(self.0)
        }
    }
    impl From<u32> for Prio163ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio163ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio163ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio163WriteVal(u32);
    impl Prio163WriteVal {
        #[inline(always)]
        pub fn prio163(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio163WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio163WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio163WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio164ReadVal(u32);
    impl Prio164ReadVal {
        #[inline(always)]
        pub fn prio164(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio164WriteVal {
            Prio164WriteVal(self.0)
        }
    }
    impl From<u32> for Prio164ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio164ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio164ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio164WriteVal(u32);
    impl Prio164WriteVal {
        #[inline(always)]
        pub fn prio164(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio164WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio164WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio164WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio165ReadVal(u32);
    impl Prio165ReadVal {
        #[inline(always)]
        pub fn prio165(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio165WriteVal {
            Prio165WriteVal(self.0)
        }
    }
    impl From<u32> for Prio165ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio165ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio165ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio165WriteVal(u32);
    impl Prio165WriteVal {
        #[inline(always)]
        pub fn prio165(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio165WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio165WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio165WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio166ReadVal(u32);
    impl Prio166ReadVal {
        #[inline(always)]
        pub fn prio166(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio166WriteVal {
            Prio166WriteVal(self.0)
        }
    }
    impl From<u32> for Prio166ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio166ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio166ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio166WriteVal(u32);
    impl Prio166WriteVal {
        #[inline(always)]
        pub fn prio166(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio166WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio166WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio166WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio167ReadVal(u32);
    impl Prio167ReadVal {
        #[inline(always)]
        pub fn prio167(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio167WriteVal {
            Prio167WriteVal(self.0)
        }
    }
    impl From<u32> for Prio167ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio167ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio167ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio167WriteVal(u32);
    impl Prio167WriteVal {
        #[inline(always)]
        pub fn prio167(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio167WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio167WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio167WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio168ReadVal(u32);
    impl Prio168ReadVal {
        #[inline(always)]
        pub fn prio168(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio168WriteVal {
            Prio168WriteVal(self.0)
        }
    }
    impl From<u32> for Prio168ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio168ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio168ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio168WriteVal(u32);
    impl Prio168WriteVal {
        #[inline(always)]
        pub fn prio168(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio168WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio168WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio168WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio169ReadVal(u32);
    impl Prio169ReadVal {
        #[inline(always)]
        pub fn prio169(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio169WriteVal {
            Prio169WriteVal(self.0)
        }
    }
    impl From<u32> for Prio169ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio169ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio169ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio169WriteVal(u32);
    impl Prio169WriteVal {
        #[inline(always)]
        pub fn prio169(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio169WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio169WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio169WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio17ReadVal(u32);
    impl Prio17ReadVal {
        #[inline(always)]
        pub fn prio17(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio17WriteVal {
            Prio17WriteVal(self.0)
        }
    }
    impl From<u32> for Prio17ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio17ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio17ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio17WriteVal(u32);
    impl Prio17WriteVal {
        #[inline(always)]
        pub fn prio17(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio17WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio17WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio17WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio170ReadVal(u32);
    impl Prio170ReadVal {
        #[inline(always)]
        pub fn prio170(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio170WriteVal {
            Prio170WriteVal(self.0)
        }
    }
    impl From<u32> for Prio170ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio170ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio170ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio170WriteVal(u32);
    impl Prio170WriteVal {
        #[inline(always)]
        pub fn prio170(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio170WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio170WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio170WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio171ReadVal(u32);
    impl Prio171ReadVal {
        #[inline(always)]
        pub fn prio171(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio171WriteVal {
            Prio171WriteVal(self.0)
        }
    }
    impl From<u32> for Prio171ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio171ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio171ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio171WriteVal(u32);
    impl Prio171WriteVal {
        #[inline(always)]
        pub fn prio171(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio171WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio171WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio171WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio172ReadVal(u32);
    impl Prio172ReadVal {
        #[inline(always)]
        pub fn prio172(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio172WriteVal {
            Prio172WriteVal(self.0)
        }
    }
    impl From<u32> for Prio172ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio172ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio172ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio172WriteVal(u32);
    impl Prio172WriteVal {
        #[inline(always)]
        pub fn prio172(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio172WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio172WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio172WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio173ReadVal(u32);
    impl Prio173ReadVal {
        #[inline(always)]
        pub fn prio173(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio173WriteVal {
            Prio173WriteVal(self.0)
        }
    }
    impl From<u32> for Prio173ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio173ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio173ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio173WriteVal(u32);
    impl Prio173WriteVal {
        #[inline(always)]
        pub fn prio173(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio173WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio173WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio173WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio174ReadVal(u32);
    impl Prio174ReadVal {
        #[inline(always)]
        pub fn prio174(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio174WriteVal {
            Prio174WriteVal(self.0)
        }
    }
    impl From<u32> for Prio174ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio174ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio174ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio174WriteVal(u32);
    impl Prio174WriteVal {
        #[inline(always)]
        pub fn prio174(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio174WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio174WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio174WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio175ReadVal(u32);
    impl Prio175ReadVal {
        #[inline(always)]
        pub fn prio175(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio175WriteVal {
            Prio175WriteVal(self.0)
        }
    }
    impl From<u32> for Prio175ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio175ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio175ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio175WriteVal(u32);
    impl Prio175WriteVal {
        #[inline(always)]
        pub fn prio175(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio175WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio175WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio175WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio176ReadVal(u32);
    impl Prio176ReadVal {
        #[inline(always)]
        pub fn prio176(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio176WriteVal {
            Prio176WriteVal(self.0)
        }
    }
    impl From<u32> for Prio176ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio176ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio176ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio176WriteVal(u32);
    impl Prio176WriteVal {
        #[inline(always)]
        pub fn prio176(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio176WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio176WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio176WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio177ReadVal(u32);
    impl Prio177ReadVal {
        #[inline(always)]
        pub fn prio177(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio177WriteVal {
            Prio177WriteVal(self.0)
        }
    }
    impl From<u32> for Prio177ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio177ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio177ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio177WriteVal(u32);
    impl Prio177WriteVal {
        #[inline(always)]
        pub fn prio177(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio177WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio177WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio177WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio178ReadVal(u32);
    impl Prio178ReadVal {
        #[inline(always)]
        pub fn prio178(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio178WriteVal {
            Prio178WriteVal(self.0)
        }
    }
    impl From<u32> for Prio178ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio178ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio178ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio178WriteVal(u32);
    impl Prio178WriteVal {
        #[inline(always)]
        pub fn prio178(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio178WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio178WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio178WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio179ReadVal(u32);
    impl Prio179ReadVal {
        #[inline(always)]
        pub fn prio179(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio179WriteVal {
            Prio179WriteVal(self.0)
        }
    }
    impl From<u32> for Prio179ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio179ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio179ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio179WriteVal(u32);
    impl Prio179WriteVal {
        #[inline(always)]
        pub fn prio179(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio179WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio179WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio179WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio18ReadVal(u32);
    impl Prio18ReadVal {
        #[inline(always)]
        pub fn prio18(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio18WriteVal {
            Prio18WriteVal(self.0)
        }
    }
    impl From<u32> for Prio18ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio18ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio18ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio18WriteVal(u32);
    impl Prio18WriteVal {
        #[inline(always)]
        pub fn prio18(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio18WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio18WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio18WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio180ReadVal(u32);
    impl Prio180ReadVal {
        #[inline(always)]
        pub fn prio180(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio180WriteVal {
            Prio180WriteVal(self.0)
        }
    }
    impl From<u32> for Prio180ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio180ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio180ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio180WriteVal(u32);
    impl Prio180WriteVal {
        #[inline(always)]
        pub fn prio180(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio180WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio180WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio180WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio181ReadVal(u32);
    impl Prio181ReadVal {
        #[inline(always)]
        pub fn prio181(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio181WriteVal {
            Prio181WriteVal(self.0)
        }
    }
    impl From<u32> for Prio181ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio181ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio181ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio181WriteVal(u32);
    impl Prio181WriteVal {
        #[inline(always)]
        pub fn prio181(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio181WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio181WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio181WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio182ReadVal(u32);
    impl Prio182ReadVal {
        #[inline(always)]
        pub fn prio182(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio182WriteVal {
            Prio182WriteVal(self.0)
        }
    }
    impl From<u32> for Prio182ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio182ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio182ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio182WriteVal(u32);
    impl Prio182WriteVal {
        #[inline(always)]
        pub fn prio182(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio182WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio182WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio182WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio183ReadVal(u32);
    impl Prio183ReadVal {
        #[inline(always)]
        pub fn prio183(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio183WriteVal {
            Prio183WriteVal(self.0)
        }
    }
    impl From<u32> for Prio183ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio183ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio183ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio183WriteVal(u32);
    impl Prio183WriteVal {
        #[inline(always)]
        pub fn prio183(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio183WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio183WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio183WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio184ReadVal(u32);
    impl Prio184ReadVal {
        #[inline(always)]
        pub fn prio184(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio184WriteVal {
            Prio184WriteVal(self.0)
        }
    }
    impl From<u32> for Prio184ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio184ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio184ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio184WriteVal(u32);
    impl Prio184WriteVal {
        #[inline(always)]
        pub fn prio184(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio184WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio184WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio184WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio185ReadVal(u32);
    impl Prio185ReadVal {
        #[inline(always)]
        pub fn prio185(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio185WriteVal {
            Prio185WriteVal(self.0)
        }
    }
    impl From<u32> for Prio185ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio185ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio185ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio185WriteVal(u32);
    impl Prio185WriteVal {
        #[inline(always)]
        pub fn prio185(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio185WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio185WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio185WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio19ReadVal(u32);
    impl Prio19ReadVal {
        #[inline(always)]
        pub fn prio19(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio19WriteVal {
            Prio19WriteVal(self.0)
        }
    }
    impl From<u32> for Prio19ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio19ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio19ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio19WriteVal(u32);
    impl Prio19WriteVal {
        #[inline(always)]
        pub fn prio19(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio19WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio19WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio19WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio2ReadVal(u32);
    impl Prio2ReadVal {
        #[inline(always)]
        pub fn prio2(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio2WriteVal {
            Prio2WriteVal(self.0)
        }
    }
    impl From<u32> for Prio2ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio2ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio2ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio2WriteVal(u32);
    impl Prio2WriteVal {
        #[inline(always)]
        pub fn prio2(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio2WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio2WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio2WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio20ReadVal(u32);
    impl Prio20ReadVal {
        #[inline(always)]
        pub fn prio20(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio20WriteVal {
            Prio20WriteVal(self.0)
        }
    }
    impl From<u32> for Prio20ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio20ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio20ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio20WriteVal(u32);
    impl Prio20WriteVal {
        #[inline(always)]
        pub fn prio20(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio20WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio20WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio20WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio21ReadVal(u32);
    impl Prio21ReadVal {
        #[inline(always)]
        pub fn prio21(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio21WriteVal {
            Prio21WriteVal(self.0)
        }
    }
    impl From<u32> for Prio21ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio21ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio21ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio21WriteVal(u32);
    impl Prio21WriteVal {
        #[inline(always)]
        pub fn prio21(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio21WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio21WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio21WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio22ReadVal(u32);
    impl Prio22ReadVal {
        #[inline(always)]
        pub fn prio22(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio22WriteVal {
            Prio22WriteVal(self.0)
        }
    }
    impl From<u32> for Prio22ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio22ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio22ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio22WriteVal(u32);
    impl Prio22WriteVal {
        #[inline(always)]
        pub fn prio22(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio22WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio22WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio22WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio23ReadVal(u32);
    impl Prio23ReadVal {
        #[inline(always)]
        pub fn prio23(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio23WriteVal {
            Prio23WriteVal(self.0)
        }
    }
    impl From<u32> for Prio23ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio23ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio23ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio23WriteVal(u32);
    impl Prio23WriteVal {
        #[inline(always)]
        pub fn prio23(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio23WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio23WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio23WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio24ReadVal(u32);
    impl Prio24ReadVal {
        #[inline(always)]
        pub fn prio24(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio24WriteVal {
            Prio24WriteVal(self.0)
        }
    }
    impl From<u32> for Prio24ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio24ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio24ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio24WriteVal(u32);
    impl Prio24WriteVal {
        #[inline(always)]
        pub fn prio24(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio24WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio24WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio24WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio25ReadVal(u32);
    impl Prio25ReadVal {
        #[inline(always)]
        pub fn prio25(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio25WriteVal {
            Prio25WriteVal(self.0)
        }
    }
    impl From<u32> for Prio25ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio25ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio25ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio25WriteVal(u32);
    impl Prio25WriteVal {
        #[inline(always)]
        pub fn prio25(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio25WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio25WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio25WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio26ReadVal(u32);
    impl Prio26ReadVal {
        #[inline(always)]
        pub fn prio26(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio26WriteVal {
            Prio26WriteVal(self.0)
        }
    }
    impl From<u32> for Prio26ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio26ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio26ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio26WriteVal(u32);
    impl Prio26WriteVal {
        #[inline(always)]
        pub fn prio26(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio26WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio26WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio26WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio27ReadVal(u32);
    impl Prio27ReadVal {
        #[inline(always)]
        pub fn prio27(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio27WriteVal {
            Prio27WriteVal(self.0)
        }
    }
    impl From<u32> for Prio27ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio27ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio27ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio27WriteVal(u32);
    impl Prio27WriteVal {
        #[inline(always)]
        pub fn prio27(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio27WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio27WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio27WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio28ReadVal(u32);
    impl Prio28ReadVal {
        #[inline(always)]
        pub fn prio28(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio28WriteVal {
            Prio28WriteVal(self.0)
        }
    }
    impl From<u32> for Prio28ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio28ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio28ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio28WriteVal(u32);
    impl Prio28WriteVal {
        #[inline(always)]
        pub fn prio28(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio28WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio28WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio28WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio29ReadVal(u32);
    impl Prio29ReadVal {
        #[inline(always)]
        pub fn prio29(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio29WriteVal {
            Prio29WriteVal(self.0)
        }
    }
    impl From<u32> for Prio29ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio29ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio29ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio29WriteVal(u32);
    impl Prio29WriteVal {
        #[inline(always)]
        pub fn prio29(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio29WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio29WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio29WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio3ReadVal(u32);
    impl Prio3ReadVal {
        #[inline(always)]
        pub fn prio3(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio3WriteVal {
            Prio3WriteVal(self.0)
        }
    }
    impl From<u32> for Prio3ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio3ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio3ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio3WriteVal(u32);
    impl Prio3WriteVal {
        #[inline(always)]
        pub fn prio3(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio3WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio3WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio3WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio30ReadVal(u32);
    impl Prio30ReadVal {
        #[inline(always)]
        pub fn prio30(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio30WriteVal {
            Prio30WriteVal(self.0)
        }
    }
    impl From<u32> for Prio30ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio30ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio30ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio30WriteVal(u32);
    impl Prio30WriteVal {
        #[inline(always)]
        pub fn prio30(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio30WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio30WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio30WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio31ReadVal(u32);
    impl Prio31ReadVal {
        #[inline(always)]
        pub fn prio31(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio31WriteVal {
            Prio31WriteVal(self.0)
        }
    }
    impl From<u32> for Prio31ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio31ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio31ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio31WriteVal(u32);
    impl Prio31WriteVal {
        #[inline(always)]
        pub fn prio31(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio31WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio31WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio31WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio32ReadVal(u32);
    impl Prio32ReadVal {
        #[inline(always)]
        pub fn prio32(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio32WriteVal {
            Prio32WriteVal(self.0)
        }
    }
    impl From<u32> for Prio32ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio32ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio32ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio32WriteVal(u32);
    impl Prio32WriteVal {
        #[inline(always)]
        pub fn prio32(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio32WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio32WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio32WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio33ReadVal(u32);
    impl Prio33ReadVal {
        #[inline(always)]
        pub fn prio33(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio33WriteVal {
            Prio33WriteVal(self.0)
        }
    }
    impl From<u32> for Prio33ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio33ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio33ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio33WriteVal(u32);
    impl Prio33WriteVal {
        #[inline(always)]
        pub fn prio33(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio33WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio33WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio33WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio34ReadVal(u32);
    impl Prio34ReadVal {
        #[inline(always)]
        pub fn prio34(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio34WriteVal {
            Prio34WriteVal(self.0)
        }
    }
    impl From<u32> for Prio34ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio34ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio34ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio34WriteVal(u32);
    impl Prio34WriteVal {
        #[inline(always)]
        pub fn prio34(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio34WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio34WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio34WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio35ReadVal(u32);
    impl Prio35ReadVal {
        #[inline(always)]
        pub fn prio35(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio35WriteVal {
            Prio35WriteVal(self.0)
        }
    }
    impl From<u32> for Prio35ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio35ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio35ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio35WriteVal(u32);
    impl Prio35WriteVal {
        #[inline(always)]
        pub fn prio35(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio35WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio35WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio35WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio36ReadVal(u32);
    impl Prio36ReadVal {
        #[inline(always)]
        pub fn prio36(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio36WriteVal {
            Prio36WriteVal(self.0)
        }
    }
    impl From<u32> for Prio36ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio36ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio36ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio36WriteVal(u32);
    impl Prio36WriteVal {
        #[inline(always)]
        pub fn prio36(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio36WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio36WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio36WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio37ReadVal(u32);
    impl Prio37ReadVal {
        #[inline(always)]
        pub fn prio37(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio37WriteVal {
            Prio37WriteVal(self.0)
        }
    }
    impl From<u32> for Prio37ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio37ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio37ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio37WriteVal(u32);
    impl Prio37WriteVal {
        #[inline(always)]
        pub fn prio37(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio37WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio37WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio37WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio38ReadVal(u32);
    impl Prio38ReadVal {
        #[inline(always)]
        pub fn prio38(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio38WriteVal {
            Prio38WriteVal(self.0)
        }
    }
    impl From<u32> for Prio38ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio38ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio38ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio38WriteVal(u32);
    impl Prio38WriteVal {
        #[inline(always)]
        pub fn prio38(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio38WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio38WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio38WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio39ReadVal(u32);
    impl Prio39ReadVal {
        #[inline(always)]
        pub fn prio39(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio39WriteVal {
            Prio39WriteVal(self.0)
        }
    }
    impl From<u32> for Prio39ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio39ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio39ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio39WriteVal(u32);
    impl Prio39WriteVal {
        #[inline(always)]
        pub fn prio39(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio39WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio39WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio39WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio4ReadVal(u32);
    impl Prio4ReadVal {
        #[inline(always)]
        pub fn prio4(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio4WriteVal {
            Prio4WriteVal(self.0)
        }
    }
    impl From<u32> for Prio4ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio4ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio4ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio4WriteVal(u32);
    impl Prio4WriteVal {
        #[inline(always)]
        pub fn prio4(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio4WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio4WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio4WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio40ReadVal(u32);
    impl Prio40ReadVal {
        #[inline(always)]
        pub fn prio40(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio40WriteVal {
            Prio40WriteVal(self.0)
        }
    }
    impl From<u32> for Prio40ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio40ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio40ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio40WriteVal(u32);
    impl Prio40WriteVal {
        #[inline(always)]
        pub fn prio40(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio40WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio40WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio40WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio41ReadVal(u32);
    impl Prio41ReadVal {
        #[inline(always)]
        pub fn prio41(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio41WriteVal {
            Prio41WriteVal(self.0)
        }
    }
    impl From<u32> for Prio41ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio41ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio41ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio41WriteVal(u32);
    impl Prio41WriteVal {
        #[inline(always)]
        pub fn prio41(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio41WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio41WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio41WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio42ReadVal(u32);
    impl Prio42ReadVal {
        #[inline(always)]
        pub fn prio42(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio42WriteVal {
            Prio42WriteVal(self.0)
        }
    }
    impl From<u32> for Prio42ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio42ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio42ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio42WriteVal(u32);
    impl Prio42WriteVal {
        #[inline(always)]
        pub fn prio42(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio42WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio42WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio42WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio43ReadVal(u32);
    impl Prio43ReadVal {
        #[inline(always)]
        pub fn prio43(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio43WriteVal {
            Prio43WriteVal(self.0)
        }
    }
    impl From<u32> for Prio43ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio43ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio43ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio43WriteVal(u32);
    impl Prio43WriteVal {
        #[inline(always)]
        pub fn prio43(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio43WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio43WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio43WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio44ReadVal(u32);
    impl Prio44ReadVal {
        #[inline(always)]
        pub fn prio44(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio44WriteVal {
            Prio44WriteVal(self.0)
        }
    }
    impl From<u32> for Prio44ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio44ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio44ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio44WriteVal(u32);
    impl Prio44WriteVal {
        #[inline(always)]
        pub fn prio44(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio44WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio44WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio44WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio45ReadVal(u32);
    impl Prio45ReadVal {
        #[inline(always)]
        pub fn prio45(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio45WriteVal {
            Prio45WriteVal(self.0)
        }
    }
    impl From<u32> for Prio45ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio45ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio45ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio45WriteVal(u32);
    impl Prio45WriteVal {
        #[inline(always)]
        pub fn prio45(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio45WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio45WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio45WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio46ReadVal(u32);
    impl Prio46ReadVal {
        #[inline(always)]
        pub fn prio46(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio46WriteVal {
            Prio46WriteVal(self.0)
        }
    }
    impl From<u32> for Prio46ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio46ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio46ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio46WriteVal(u32);
    impl Prio46WriteVal {
        #[inline(always)]
        pub fn prio46(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio46WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio46WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio46WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio47ReadVal(u32);
    impl Prio47ReadVal {
        #[inline(always)]
        pub fn prio47(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio47WriteVal {
            Prio47WriteVal(self.0)
        }
    }
    impl From<u32> for Prio47ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio47ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio47ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio47WriteVal(u32);
    impl Prio47WriteVal {
        #[inline(always)]
        pub fn prio47(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio47WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio47WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio47WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio48ReadVal(u32);
    impl Prio48ReadVal {
        #[inline(always)]
        pub fn prio48(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio48WriteVal {
            Prio48WriteVal(self.0)
        }
    }
    impl From<u32> for Prio48ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio48ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio48ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio48WriteVal(u32);
    impl Prio48WriteVal {
        #[inline(always)]
        pub fn prio48(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio48WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio48WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio48WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio49ReadVal(u32);
    impl Prio49ReadVal {
        #[inline(always)]
        pub fn prio49(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio49WriteVal {
            Prio49WriteVal(self.0)
        }
    }
    impl From<u32> for Prio49ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio49ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio49ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio49WriteVal(u32);
    impl Prio49WriteVal {
        #[inline(always)]
        pub fn prio49(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio49WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio49WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio49WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio5ReadVal(u32);
    impl Prio5ReadVal {
        #[inline(always)]
        pub fn prio5(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio5WriteVal {
            Prio5WriteVal(self.0)
        }
    }
    impl From<u32> for Prio5ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio5ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio5ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio5WriteVal(u32);
    impl Prio5WriteVal {
        #[inline(always)]
        pub fn prio5(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio5WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio5WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio5WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio50ReadVal(u32);
    impl Prio50ReadVal {
        #[inline(always)]
        pub fn prio50(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio50WriteVal {
            Prio50WriteVal(self.0)
        }
    }
    impl From<u32> for Prio50ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio50ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio50ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio50WriteVal(u32);
    impl Prio50WriteVal {
        #[inline(always)]
        pub fn prio50(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio50WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio50WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio50WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio51ReadVal(u32);
    impl Prio51ReadVal {
        #[inline(always)]
        pub fn prio51(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio51WriteVal {
            Prio51WriteVal(self.0)
        }
    }
    impl From<u32> for Prio51ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio51ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio51ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio51WriteVal(u32);
    impl Prio51WriteVal {
        #[inline(always)]
        pub fn prio51(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio51WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio51WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio51WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio52ReadVal(u32);
    impl Prio52ReadVal {
        #[inline(always)]
        pub fn prio52(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio52WriteVal {
            Prio52WriteVal(self.0)
        }
    }
    impl From<u32> for Prio52ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio52ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio52ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio52WriteVal(u32);
    impl Prio52WriteVal {
        #[inline(always)]
        pub fn prio52(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio52WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio52WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio52WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio53ReadVal(u32);
    impl Prio53ReadVal {
        #[inline(always)]
        pub fn prio53(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio53WriteVal {
            Prio53WriteVal(self.0)
        }
    }
    impl From<u32> for Prio53ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio53ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio53ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio53WriteVal(u32);
    impl Prio53WriteVal {
        #[inline(always)]
        pub fn prio53(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio53WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio53WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio53WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio54ReadVal(u32);
    impl Prio54ReadVal {
        #[inline(always)]
        pub fn prio54(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio54WriteVal {
            Prio54WriteVal(self.0)
        }
    }
    impl From<u32> for Prio54ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio54ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio54ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio54WriteVal(u32);
    impl Prio54WriteVal {
        #[inline(always)]
        pub fn prio54(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio54WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio54WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio54WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio55ReadVal(u32);
    impl Prio55ReadVal {
        #[inline(always)]
        pub fn prio55(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio55WriteVal {
            Prio55WriteVal(self.0)
        }
    }
    impl From<u32> for Prio55ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio55ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio55ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio55WriteVal(u32);
    impl Prio55WriteVal {
        #[inline(always)]
        pub fn prio55(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio55WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio55WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio55WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio56ReadVal(u32);
    impl Prio56ReadVal {
        #[inline(always)]
        pub fn prio56(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio56WriteVal {
            Prio56WriteVal(self.0)
        }
    }
    impl From<u32> for Prio56ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio56ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio56ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio56WriteVal(u32);
    impl Prio56WriteVal {
        #[inline(always)]
        pub fn prio56(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio56WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio56WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio56WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio57ReadVal(u32);
    impl Prio57ReadVal {
        #[inline(always)]
        pub fn prio57(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio57WriteVal {
            Prio57WriteVal(self.0)
        }
    }
    impl From<u32> for Prio57ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio57ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio57ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio57WriteVal(u32);
    impl Prio57WriteVal {
        #[inline(always)]
        pub fn prio57(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio57WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio57WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio57WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio58ReadVal(u32);
    impl Prio58ReadVal {
        #[inline(always)]
        pub fn prio58(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio58WriteVal {
            Prio58WriteVal(self.0)
        }
    }
    impl From<u32> for Prio58ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio58ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio58ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio58WriteVal(u32);
    impl Prio58WriteVal {
        #[inline(always)]
        pub fn prio58(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio58WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio58WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio58WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio59ReadVal(u32);
    impl Prio59ReadVal {
        #[inline(always)]
        pub fn prio59(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio59WriteVal {
            Prio59WriteVal(self.0)
        }
    }
    impl From<u32> for Prio59ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio59ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio59ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio59WriteVal(u32);
    impl Prio59WriteVal {
        #[inline(always)]
        pub fn prio59(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio59WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio59WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio59WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio6ReadVal(u32);
    impl Prio6ReadVal {
        #[inline(always)]
        pub fn prio6(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio6WriteVal {
            Prio6WriteVal(self.0)
        }
    }
    impl From<u32> for Prio6ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio6ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio6ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio6WriteVal(u32);
    impl Prio6WriteVal {
        #[inline(always)]
        pub fn prio6(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio6WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio6WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio6WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio60ReadVal(u32);
    impl Prio60ReadVal {
        #[inline(always)]
        pub fn prio60(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio60WriteVal {
            Prio60WriteVal(self.0)
        }
    }
    impl From<u32> for Prio60ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio60ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio60ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio60WriteVal(u32);
    impl Prio60WriteVal {
        #[inline(always)]
        pub fn prio60(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio60WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio60WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio60WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio61ReadVal(u32);
    impl Prio61ReadVal {
        #[inline(always)]
        pub fn prio61(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio61WriteVal {
            Prio61WriteVal(self.0)
        }
    }
    impl From<u32> for Prio61ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio61ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio61ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio61WriteVal(u32);
    impl Prio61WriteVal {
        #[inline(always)]
        pub fn prio61(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio61WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio61WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio61WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio62ReadVal(u32);
    impl Prio62ReadVal {
        #[inline(always)]
        pub fn prio62(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio62WriteVal {
            Prio62WriteVal(self.0)
        }
    }
    impl From<u32> for Prio62ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio62ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio62ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio62WriteVal(u32);
    impl Prio62WriteVal {
        #[inline(always)]
        pub fn prio62(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio62WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio62WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio62WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio63ReadVal(u32);
    impl Prio63ReadVal {
        #[inline(always)]
        pub fn prio63(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio63WriteVal {
            Prio63WriteVal(self.0)
        }
    }
    impl From<u32> for Prio63ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio63ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio63ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio63WriteVal(u32);
    impl Prio63WriteVal {
        #[inline(always)]
        pub fn prio63(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio63WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio63WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio63WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio64ReadVal(u32);
    impl Prio64ReadVal {
        #[inline(always)]
        pub fn prio64(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio64WriteVal {
            Prio64WriteVal(self.0)
        }
    }
    impl From<u32> for Prio64ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio64ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio64ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio64WriteVal(u32);
    impl Prio64WriteVal {
        #[inline(always)]
        pub fn prio64(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio64WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio64WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio64WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio65ReadVal(u32);
    impl Prio65ReadVal {
        #[inline(always)]
        pub fn prio65(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio65WriteVal {
            Prio65WriteVal(self.0)
        }
    }
    impl From<u32> for Prio65ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio65ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio65ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio65WriteVal(u32);
    impl Prio65WriteVal {
        #[inline(always)]
        pub fn prio65(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio65WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio65WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio65WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio66ReadVal(u32);
    impl Prio66ReadVal {
        #[inline(always)]
        pub fn prio66(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio66WriteVal {
            Prio66WriteVal(self.0)
        }
    }
    impl From<u32> for Prio66ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio66ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio66ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio66WriteVal(u32);
    impl Prio66WriteVal {
        #[inline(always)]
        pub fn prio66(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio66WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio66WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio66WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio67ReadVal(u32);
    impl Prio67ReadVal {
        #[inline(always)]
        pub fn prio67(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio67WriteVal {
            Prio67WriteVal(self.0)
        }
    }
    impl From<u32> for Prio67ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio67ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio67ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio67WriteVal(u32);
    impl Prio67WriteVal {
        #[inline(always)]
        pub fn prio67(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio67WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio67WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio67WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio68ReadVal(u32);
    impl Prio68ReadVal {
        #[inline(always)]
        pub fn prio68(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio68WriteVal {
            Prio68WriteVal(self.0)
        }
    }
    impl From<u32> for Prio68ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio68ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio68ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio68WriteVal(u32);
    impl Prio68WriteVal {
        #[inline(always)]
        pub fn prio68(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio68WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio68WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio68WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio69ReadVal(u32);
    impl Prio69ReadVal {
        #[inline(always)]
        pub fn prio69(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio69WriteVal {
            Prio69WriteVal(self.0)
        }
    }
    impl From<u32> for Prio69ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio69ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio69ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio69WriteVal(u32);
    impl Prio69WriteVal {
        #[inline(always)]
        pub fn prio69(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio69WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio69WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio69WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio7ReadVal(u32);
    impl Prio7ReadVal {
        #[inline(always)]
        pub fn prio7(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio7WriteVal {
            Prio7WriteVal(self.0)
        }
    }
    impl From<u32> for Prio7ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio7ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio7ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio7WriteVal(u32);
    impl Prio7WriteVal {
        #[inline(always)]
        pub fn prio7(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio7WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio7WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio7WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio70ReadVal(u32);
    impl Prio70ReadVal {
        #[inline(always)]
        pub fn prio70(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio70WriteVal {
            Prio70WriteVal(self.0)
        }
    }
    impl From<u32> for Prio70ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio70ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio70ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio70WriteVal(u32);
    impl Prio70WriteVal {
        #[inline(always)]
        pub fn prio70(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio70WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio70WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio70WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio71ReadVal(u32);
    impl Prio71ReadVal {
        #[inline(always)]
        pub fn prio71(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio71WriteVal {
            Prio71WriteVal(self.0)
        }
    }
    impl From<u32> for Prio71ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio71ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio71ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio71WriteVal(u32);
    impl Prio71WriteVal {
        #[inline(always)]
        pub fn prio71(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio71WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio71WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio71WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio72ReadVal(u32);
    impl Prio72ReadVal {
        #[inline(always)]
        pub fn prio72(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio72WriteVal {
            Prio72WriteVal(self.0)
        }
    }
    impl From<u32> for Prio72ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio72ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio72ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio72WriteVal(u32);
    impl Prio72WriteVal {
        #[inline(always)]
        pub fn prio72(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio72WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio72WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio72WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio73ReadVal(u32);
    impl Prio73ReadVal {
        #[inline(always)]
        pub fn prio73(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio73WriteVal {
            Prio73WriteVal(self.0)
        }
    }
    impl From<u32> for Prio73ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio73ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio73ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio73WriteVal(u32);
    impl Prio73WriteVal {
        #[inline(always)]
        pub fn prio73(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio73WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio73WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio73WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio74ReadVal(u32);
    impl Prio74ReadVal {
        #[inline(always)]
        pub fn prio74(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio74WriteVal {
            Prio74WriteVal(self.0)
        }
    }
    impl From<u32> for Prio74ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio74ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio74ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio74WriteVal(u32);
    impl Prio74WriteVal {
        #[inline(always)]
        pub fn prio74(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio74WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio74WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio74WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio75ReadVal(u32);
    impl Prio75ReadVal {
        #[inline(always)]
        pub fn prio75(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio75WriteVal {
            Prio75WriteVal(self.0)
        }
    }
    impl From<u32> for Prio75ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio75ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio75ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio75WriteVal(u32);
    impl Prio75WriteVal {
        #[inline(always)]
        pub fn prio75(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio75WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio75WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio75WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio76ReadVal(u32);
    impl Prio76ReadVal {
        #[inline(always)]
        pub fn prio76(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio76WriteVal {
            Prio76WriteVal(self.0)
        }
    }
    impl From<u32> for Prio76ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio76ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio76ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio76WriteVal(u32);
    impl Prio76WriteVal {
        #[inline(always)]
        pub fn prio76(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio76WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio76WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio76WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio77ReadVal(u32);
    impl Prio77ReadVal {
        #[inline(always)]
        pub fn prio77(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio77WriteVal {
            Prio77WriteVal(self.0)
        }
    }
    impl From<u32> for Prio77ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio77ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio77ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio77WriteVal(u32);
    impl Prio77WriteVal {
        #[inline(always)]
        pub fn prio77(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio77WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio77WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio77WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio78ReadVal(u32);
    impl Prio78ReadVal {
        #[inline(always)]
        pub fn prio78(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio78WriteVal {
            Prio78WriteVal(self.0)
        }
    }
    impl From<u32> for Prio78ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio78ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio78ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio78WriteVal(u32);
    impl Prio78WriteVal {
        #[inline(always)]
        pub fn prio78(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio78WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio78WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio78WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio79ReadVal(u32);
    impl Prio79ReadVal {
        #[inline(always)]
        pub fn prio79(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio79WriteVal {
            Prio79WriteVal(self.0)
        }
    }
    impl From<u32> for Prio79ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio79ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio79ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio79WriteVal(u32);
    impl Prio79WriteVal {
        #[inline(always)]
        pub fn prio79(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio79WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio79WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio79WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio8ReadVal(u32);
    impl Prio8ReadVal {
        #[inline(always)]
        pub fn prio8(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio8WriteVal {
            Prio8WriteVal(self.0)
        }
    }
    impl From<u32> for Prio8ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio8ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio8ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio8WriteVal(u32);
    impl Prio8WriteVal {
        #[inline(always)]
        pub fn prio8(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio8WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio8WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio8WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio80ReadVal(u32);
    impl Prio80ReadVal {
        #[inline(always)]
        pub fn prio80(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio80WriteVal {
            Prio80WriteVal(self.0)
        }
    }
    impl From<u32> for Prio80ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio80ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio80ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio80WriteVal(u32);
    impl Prio80WriteVal {
        #[inline(always)]
        pub fn prio80(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio80WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio80WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio80WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio81ReadVal(u32);
    impl Prio81ReadVal {
        #[inline(always)]
        pub fn prio81(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio81WriteVal {
            Prio81WriteVal(self.0)
        }
    }
    impl From<u32> for Prio81ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio81ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio81ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio81WriteVal(u32);
    impl Prio81WriteVal {
        #[inline(always)]
        pub fn prio81(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio81WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio81WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio81WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio82ReadVal(u32);
    impl Prio82ReadVal {
        #[inline(always)]
        pub fn prio82(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio82WriteVal {
            Prio82WriteVal(self.0)
        }
    }
    impl From<u32> for Prio82ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio82ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio82ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio82WriteVal(u32);
    impl Prio82WriteVal {
        #[inline(always)]
        pub fn prio82(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio82WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio82WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio82WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio83ReadVal(u32);
    impl Prio83ReadVal {
        #[inline(always)]
        pub fn prio83(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio83WriteVal {
            Prio83WriteVal(self.0)
        }
    }
    impl From<u32> for Prio83ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio83ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio83ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio83WriteVal(u32);
    impl Prio83WriteVal {
        #[inline(always)]
        pub fn prio83(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio83WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio83WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio83WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio84ReadVal(u32);
    impl Prio84ReadVal {
        #[inline(always)]
        pub fn prio84(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio84WriteVal {
            Prio84WriteVal(self.0)
        }
    }
    impl From<u32> for Prio84ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio84ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio84ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio84WriteVal(u32);
    impl Prio84WriteVal {
        #[inline(always)]
        pub fn prio84(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio84WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio84WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio84WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio85ReadVal(u32);
    impl Prio85ReadVal {
        #[inline(always)]
        pub fn prio85(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio85WriteVal {
            Prio85WriteVal(self.0)
        }
    }
    impl From<u32> for Prio85ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio85ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio85ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio85WriteVal(u32);
    impl Prio85WriteVal {
        #[inline(always)]
        pub fn prio85(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio85WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio85WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio85WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio86ReadVal(u32);
    impl Prio86ReadVal {
        #[inline(always)]
        pub fn prio86(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio86WriteVal {
            Prio86WriteVal(self.0)
        }
    }
    impl From<u32> for Prio86ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio86ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio86ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio86WriteVal(u32);
    impl Prio86WriteVal {
        #[inline(always)]
        pub fn prio86(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio86WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio86WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio86WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio87ReadVal(u32);
    impl Prio87ReadVal {
        #[inline(always)]
        pub fn prio87(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio87WriteVal {
            Prio87WriteVal(self.0)
        }
    }
    impl From<u32> for Prio87ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio87ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio87ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio87WriteVal(u32);
    impl Prio87WriteVal {
        #[inline(always)]
        pub fn prio87(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio87WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio87WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio87WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio88ReadVal(u32);
    impl Prio88ReadVal {
        #[inline(always)]
        pub fn prio88(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio88WriteVal {
            Prio88WriteVal(self.0)
        }
    }
    impl From<u32> for Prio88ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio88ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio88ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio88WriteVal(u32);
    impl Prio88WriteVal {
        #[inline(always)]
        pub fn prio88(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio88WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio88WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio88WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio89ReadVal(u32);
    impl Prio89ReadVal {
        #[inline(always)]
        pub fn prio89(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio89WriteVal {
            Prio89WriteVal(self.0)
        }
    }
    impl From<u32> for Prio89ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio89ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio89ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio89WriteVal(u32);
    impl Prio89WriteVal {
        #[inline(always)]
        pub fn prio89(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio89WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio89WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio89WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio9ReadVal(u32);
    impl Prio9ReadVal {
        #[inline(always)]
        pub fn prio9(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio9WriteVal {
            Prio9WriteVal(self.0)
        }
    }
    impl From<u32> for Prio9ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio9ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio9ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio9WriteVal(u32);
    impl Prio9WriteVal {
        #[inline(always)]
        pub fn prio9(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio9WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio9WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio9WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio90ReadVal(u32);
    impl Prio90ReadVal {
        #[inline(always)]
        pub fn prio90(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio90WriteVal {
            Prio90WriteVal(self.0)
        }
    }
    impl From<u32> for Prio90ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio90ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio90ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio90WriteVal(u32);
    impl Prio90WriteVal {
        #[inline(always)]
        pub fn prio90(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio90WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio90WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio90WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio91ReadVal(u32);
    impl Prio91ReadVal {
        #[inline(always)]
        pub fn prio91(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio91WriteVal {
            Prio91WriteVal(self.0)
        }
    }
    impl From<u32> for Prio91ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio91ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio91ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio91WriteVal(u32);
    impl Prio91WriteVal {
        #[inline(always)]
        pub fn prio91(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio91WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio91WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio91WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio92ReadVal(u32);
    impl Prio92ReadVal {
        #[inline(always)]
        pub fn prio92(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio92WriteVal {
            Prio92WriteVal(self.0)
        }
    }
    impl From<u32> for Prio92ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio92ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio92ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio92WriteVal(u32);
    impl Prio92WriteVal {
        #[inline(always)]
        pub fn prio92(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio92WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio92WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio92WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio93ReadVal(u32);
    impl Prio93ReadVal {
        #[inline(always)]
        pub fn prio93(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio93WriteVal {
            Prio93WriteVal(self.0)
        }
    }
    impl From<u32> for Prio93ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio93ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio93ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio93WriteVal(u32);
    impl Prio93WriteVal {
        #[inline(always)]
        pub fn prio93(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio93WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio93WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio93WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio94ReadVal(u32);
    impl Prio94ReadVal {
        #[inline(always)]
        pub fn prio94(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio94WriteVal {
            Prio94WriteVal(self.0)
        }
    }
    impl From<u32> for Prio94ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio94ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio94ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio94WriteVal(u32);
    impl Prio94WriteVal {
        #[inline(always)]
        pub fn prio94(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio94WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio94WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio94WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio95ReadVal(u32);
    impl Prio95ReadVal {
        #[inline(always)]
        pub fn prio95(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio95WriteVal {
            Prio95WriteVal(self.0)
        }
    }
    impl From<u32> for Prio95ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio95ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio95ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio95WriteVal(u32);
    impl Prio95WriteVal {
        #[inline(always)]
        pub fn prio95(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio95WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio95WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio95WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio96ReadVal(u32);
    impl Prio96ReadVal {
        #[inline(always)]
        pub fn prio96(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio96WriteVal {
            Prio96WriteVal(self.0)
        }
    }
    impl From<u32> for Prio96ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio96ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio96ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio96WriteVal(u32);
    impl Prio96WriteVal {
        #[inline(always)]
        pub fn prio96(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio96WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio96WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio96WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio97ReadVal(u32);
    impl Prio97ReadVal {
        #[inline(always)]
        pub fn prio97(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio97WriteVal {
            Prio97WriteVal(self.0)
        }
    }
    impl From<u32> for Prio97ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio97ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio97ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio97WriteVal(u32);
    impl Prio97WriteVal {
        #[inline(always)]
        pub fn prio97(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio97WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio97WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio97WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio98ReadVal(u32);
    impl Prio98ReadVal {
        #[inline(always)]
        pub fn prio98(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio98WriteVal {
            Prio98WriteVal(self.0)
        }
    }
    impl From<u32> for Prio98ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio98ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio98ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio98WriteVal(u32);
    impl Prio98WriteVal {
        #[inline(always)]
        pub fn prio98(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio98WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio98WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio98WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio99ReadVal(u32);
    impl Prio99ReadVal {
        #[inline(always)]
        pub fn prio99(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Prio99WriteVal {
            Prio99WriteVal(self.0)
        }
    }
    impl From<u32> for Prio99ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio99ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Prio99ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Prio99WriteVal(u32);
    impl Prio99WriteVal {
        #[inline(always)]
        pub fn prio99(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Prio99WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Prio99WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Prio99WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Threshold0ReadVal(u32);
    impl Threshold0ReadVal {
        #[inline(always)]
        pub fn threshold0(&self) -> u32 {
            (self.0 >> 0) & 3
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> Threshold0WriteVal {
            Threshold0WriteVal(self.0)
        }
    }
    impl From<u32> for Threshold0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Threshold0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: Threshold0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct Threshold0WriteVal(u32);
    impl Threshold0WriteVal {
        #[inline(always)]
        pub fn threshold0(self, val: u32) -> Self {
            Self((self.0 & !(3 << 0)) | ((val & 3) << 0))
        }
    }
    impl From<u32> for Threshold0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<Threshold0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: Threshold0WriteVal) -> u32 {
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
    pub type Prio0 = ureg::ReadWriteReg32<0, crate::regs::Prio0ReadVal, crate::regs::Prio0WriteVal>;
    pub type Prio1 = ureg::ReadWriteReg32<0, crate::regs::Prio1ReadVal, crate::regs::Prio1WriteVal>;
    pub type Prio2 = ureg::ReadWriteReg32<0, crate::regs::Prio2ReadVal, crate::regs::Prio2WriteVal>;
    pub type Prio3 = ureg::ReadWriteReg32<0, crate::regs::Prio3ReadVal, crate::regs::Prio3WriteVal>;
    pub type Prio4 = ureg::ReadWriteReg32<0, crate::regs::Prio4ReadVal, crate::regs::Prio4WriteVal>;
    pub type Prio5 = ureg::ReadWriteReg32<0, crate::regs::Prio5ReadVal, crate::regs::Prio5WriteVal>;
    pub type Prio6 = ureg::ReadWriteReg32<0, crate::regs::Prio6ReadVal, crate::regs::Prio6WriteVal>;
    pub type Prio7 = ureg::ReadWriteReg32<0, crate::regs::Prio7ReadVal, crate::regs::Prio7WriteVal>;
    pub type Prio8 = ureg::ReadWriteReg32<0, crate::regs::Prio8ReadVal, crate::regs::Prio8WriteVal>;
    pub type Prio9 = ureg::ReadWriteReg32<0, crate::regs::Prio9ReadVal, crate::regs::Prio9WriteVal>;
    pub type Prio10 =
        ureg::ReadWriteReg32<0, crate::regs::Prio10ReadVal, crate::regs::Prio10WriteVal>;
    pub type Prio11 =
        ureg::ReadWriteReg32<0, crate::regs::Prio11ReadVal, crate::regs::Prio11WriteVal>;
    pub type Prio12 =
        ureg::ReadWriteReg32<0, crate::regs::Prio12ReadVal, crate::regs::Prio12WriteVal>;
    pub type Prio13 =
        ureg::ReadWriteReg32<0, crate::regs::Prio13ReadVal, crate::regs::Prio13WriteVal>;
    pub type Prio14 =
        ureg::ReadWriteReg32<0, crate::regs::Prio14ReadVal, crate::regs::Prio14WriteVal>;
    pub type Prio15 =
        ureg::ReadWriteReg32<0, crate::regs::Prio15ReadVal, crate::regs::Prio15WriteVal>;
    pub type Prio16 =
        ureg::ReadWriteReg32<0, crate::regs::Prio16ReadVal, crate::regs::Prio16WriteVal>;
    pub type Prio17 =
        ureg::ReadWriteReg32<0, crate::regs::Prio17ReadVal, crate::regs::Prio17WriteVal>;
    pub type Prio18 =
        ureg::ReadWriteReg32<0, crate::regs::Prio18ReadVal, crate::regs::Prio18WriteVal>;
    pub type Prio19 =
        ureg::ReadWriteReg32<0, crate::regs::Prio19ReadVal, crate::regs::Prio19WriteVal>;
    pub type Prio20 =
        ureg::ReadWriteReg32<0, crate::regs::Prio20ReadVal, crate::regs::Prio20WriteVal>;
    pub type Prio21 =
        ureg::ReadWriteReg32<0, crate::regs::Prio21ReadVal, crate::regs::Prio21WriteVal>;
    pub type Prio22 =
        ureg::ReadWriteReg32<0, crate::regs::Prio22ReadVal, crate::regs::Prio22WriteVal>;
    pub type Prio23 =
        ureg::ReadWriteReg32<0, crate::regs::Prio23ReadVal, crate::regs::Prio23WriteVal>;
    pub type Prio24 =
        ureg::ReadWriteReg32<0, crate::regs::Prio24ReadVal, crate::regs::Prio24WriteVal>;
    pub type Prio25 =
        ureg::ReadWriteReg32<0, crate::regs::Prio25ReadVal, crate::regs::Prio25WriteVal>;
    pub type Prio26 =
        ureg::ReadWriteReg32<0, crate::regs::Prio26ReadVal, crate::regs::Prio26WriteVal>;
    pub type Prio27 =
        ureg::ReadWriteReg32<0, crate::regs::Prio27ReadVal, crate::regs::Prio27WriteVal>;
    pub type Prio28 =
        ureg::ReadWriteReg32<0, crate::regs::Prio28ReadVal, crate::regs::Prio28WriteVal>;
    pub type Prio29 =
        ureg::ReadWriteReg32<0, crate::regs::Prio29ReadVal, crate::regs::Prio29WriteVal>;
    pub type Prio30 =
        ureg::ReadWriteReg32<0, crate::regs::Prio30ReadVal, crate::regs::Prio30WriteVal>;
    pub type Prio31 =
        ureg::ReadWriteReg32<0, crate::regs::Prio31ReadVal, crate::regs::Prio31WriteVal>;
    pub type Prio32 =
        ureg::ReadWriteReg32<0, crate::regs::Prio32ReadVal, crate::regs::Prio32WriteVal>;
    pub type Prio33 =
        ureg::ReadWriteReg32<0, crate::regs::Prio33ReadVal, crate::regs::Prio33WriteVal>;
    pub type Prio34 =
        ureg::ReadWriteReg32<0, crate::regs::Prio34ReadVal, crate::regs::Prio34WriteVal>;
    pub type Prio35 =
        ureg::ReadWriteReg32<0, crate::regs::Prio35ReadVal, crate::regs::Prio35WriteVal>;
    pub type Prio36 =
        ureg::ReadWriteReg32<0, crate::regs::Prio36ReadVal, crate::regs::Prio36WriteVal>;
    pub type Prio37 =
        ureg::ReadWriteReg32<0, crate::regs::Prio37ReadVal, crate::regs::Prio37WriteVal>;
    pub type Prio38 =
        ureg::ReadWriteReg32<0, crate::regs::Prio38ReadVal, crate::regs::Prio38WriteVal>;
    pub type Prio39 =
        ureg::ReadWriteReg32<0, crate::regs::Prio39ReadVal, crate::regs::Prio39WriteVal>;
    pub type Prio40 =
        ureg::ReadWriteReg32<0, crate::regs::Prio40ReadVal, crate::regs::Prio40WriteVal>;
    pub type Prio41 =
        ureg::ReadWriteReg32<0, crate::regs::Prio41ReadVal, crate::regs::Prio41WriteVal>;
    pub type Prio42 =
        ureg::ReadWriteReg32<0, crate::regs::Prio42ReadVal, crate::regs::Prio42WriteVal>;
    pub type Prio43 =
        ureg::ReadWriteReg32<0, crate::regs::Prio43ReadVal, crate::regs::Prio43WriteVal>;
    pub type Prio44 =
        ureg::ReadWriteReg32<0, crate::regs::Prio44ReadVal, crate::regs::Prio44WriteVal>;
    pub type Prio45 =
        ureg::ReadWriteReg32<0, crate::regs::Prio45ReadVal, crate::regs::Prio45WriteVal>;
    pub type Prio46 =
        ureg::ReadWriteReg32<0, crate::regs::Prio46ReadVal, crate::regs::Prio46WriteVal>;
    pub type Prio47 =
        ureg::ReadWriteReg32<0, crate::regs::Prio47ReadVal, crate::regs::Prio47WriteVal>;
    pub type Prio48 =
        ureg::ReadWriteReg32<0, crate::regs::Prio48ReadVal, crate::regs::Prio48WriteVal>;
    pub type Prio49 =
        ureg::ReadWriteReg32<0, crate::regs::Prio49ReadVal, crate::regs::Prio49WriteVal>;
    pub type Prio50 =
        ureg::ReadWriteReg32<0, crate::regs::Prio50ReadVal, crate::regs::Prio50WriteVal>;
    pub type Prio51 =
        ureg::ReadWriteReg32<0, crate::regs::Prio51ReadVal, crate::regs::Prio51WriteVal>;
    pub type Prio52 =
        ureg::ReadWriteReg32<0, crate::regs::Prio52ReadVal, crate::regs::Prio52WriteVal>;
    pub type Prio53 =
        ureg::ReadWriteReg32<0, crate::regs::Prio53ReadVal, crate::regs::Prio53WriteVal>;
    pub type Prio54 =
        ureg::ReadWriteReg32<0, crate::regs::Prio54ReadVal, crate::regs::Prio54WriteVal>;
    pub type Prio55 =
        ureg::ReadWriteReg32<0, crate::regs::Prio55ReadVal, crate::regs::Prio55WriteVal>;
    pub type Prio56 =
        ureg::ReadWriteReg32<0, crate::regs::Prio56ReadVal, crate::regs::Prio56WriteVal>;
    pub type Prio57 =
        ureg::ReadWriteReg32<0, crate::regs::Prio57ReadVal, crate::regs::Prio57WriteVal>;
    pub type Prio58 =
        ureg::ReadWriteReg32<0, crate::regs::Prio58ReadVal, crate::regs::Prio58WriteVal>;
    pub type Prio59 =
        ureg::ReadWriteReg32<0, crate::regs::Prio59ReadVal, crate::regs::Prio59WriteVal>;
    pub type Prio60 =
        ureg::ReadWriteReg32<0, crate::regs::Prio60ReadVal, crate::regs::Prio60WriteVal>;
    pub type Prio61 =
        ureg::ReadWriteReg32<0, crate::regs::Prio61ReadVal, crate::regs::Prio61WriteVal>;
    pub type Prio62 =
        ureg::ReadWriteReg32<0, crate::regs::Prio62ReadVal, crate::regs::Prio62WriteVal>;
    pub type Prio63 =
        ureg::ReadWriteReg32<0, crate::regs::Prio63ReadVal, crate::regs::Prio63WriteVal>;
    pub type Prio64 =
        ureg::ReadWriteReg32<0, crate::regs::Prio64ReadVal, crate::regs::Prio64WriteVal>;
    pub type Prio65 =
        ureg::ReadWriteReg32<0, crate::regs::Prio65ReadVal, crate::regs::Prio65WriteVal>;
    pub type Prio66 =
        ureg::ReadWriteReg32<0, crate::regs::Prio66ReadVal, crate::regs::Prio66WriteVal>;
    pub type Prio67 =
        ureg::ReadWriteReg32<0, crate::regs::Prio67ReadVal, crate::regs::Prio67WriteVal>;
    pub type Prio68 =
        ureg::ReadWriteReg32<0, crate::regs::Prio68ReadVal, crate::regs::Prio68WriteVal>;
    pub type Prio69 =
        ureg::ReadWriteReg32<0, crate::regs::Prio69ReadVal, crate::regs::Prio69WriteVal>;
    pub type Prio70 =
        ureg::ReadWriteReg32<0, crate::regs::Prio70ReadVal, crate::regs::Prio70WriteVal>;
    pub type Prio71 =
        ureg::ReadWriteReg32<0, crate::regs::Prio71ReadVal, crate::regs::Prio71WriteVal>;
    pub type Prio72 =
        ureg::ReadWriteReg32<0, crate::regs::Prio72ReadVal, crate::regs::Prio72WriteVal>;
    pub type Prio73 =
        ureg::ReadWriteReg32<0, crate::regs::Prio73ReadVal, crate::regs::Prio73WriteVal>;
    pub type Prio74 =
        ureg::ReadWriteReg32<0, crate::regs::Prio74ReadVal, crate::regs::Prio74WriteVal>;
    pub type Prio75 =
        ureg::ReadWriteReg32<0, crate::regs::Prio75ReadVal, crate::regs::Prio75WriteVal>;
    pub type Prio76 =
        ureg::ReadWriteReg32<0, crate::regs::Prio76ReadVal, crate::regs::Prio76WriteVal>;
    pub type Prio77 =
        ureg::ReadWriteReg32<0, crate::regs::Prio77ReadVal, crate::regs::Prio77WriteVal>;
    pub type Prio78 =
        ureg::ReadWriteReg32<0, crate::regs::Prio78ReadVal, crate::regs::Prio78WriteVal>;
    pub type Prio79 =
        ureg::ReadWriteReg32<0, crate::regs::Prio79ReadVal, crate::regs::Prio79WriteVal>;
    pub type Prio80 =
        ureg::ReadWriteReg32<0, crate::regs::Prio80ReadVal, crate::regs::Prio80WriteVal>;
    pub type Prio81 =
        ureg::ReadWriteReg32<0, crate::regs::Prio81ReadVal, crate::regs::Prio81WriteVal>;
    pub type Prio82 =
        ureg::ReadWriteReg32<0, crate::regs::Prio82ReadVal, crate::regs::Prio82WriteVal>;
    pub type Prio83 =
        ureg::ReadWriteReg32<0, crate::regs::Prio83ReadVal, crate::regs::Prio83WriteVal>;
    pub type Prio84 =
        ureg::ReadWriteReg32<0, crate::regs::Prio84ReadVal, crate::regs::Prio84WriteVal>;
    pub type Prio85 =
        ureg::ReadWriteReg32<0, crate::regs::Prio85ReadVal, crate::regs::Prio85WriteVal>;
    pub type Prio86 =
        ureg::ReadWriteReg32<0, crate::regs::Prio86ReadVal, crate::regs::Prio86WriteVal>;
    pub type Prio87 =
        ureg::ReadWriteReg32<0, crate::regs::Prio87ReadVal, crate::regs::Prio87WriteVal>;
    pub type Prio88 =
        ureg::ReadWriteReg32<0, crate::regs::Prio88ReadVal, crate::regs::Prio88WriteVal>;
    pub type Prio89 =
        ureg::ReadWriteReg32<0, crate::regs::Prio89ReadVal, crate::regs::Prio89WriteVal>;
    pub type Prio90 =
        ureg::ReadWriteReg32<0, crate::regs::Prio90ReadVal, crate::regs::Prio90WriteVal>;
    pub type Prio91 =
        ureg::ReadWriteReg32<0, crate::regs::Prio91ReadVal, crate::regs::Prio91WriteVal>;
    pub type Prio92 =
        ureg::ReadWriteReg32<0, crate::regs::Prio92ReadVal, crate::regs::Prio92WriteVal>;
    pub type Prio93 =
        ureg::ReadWriteReg32<0, crate::regs::Prio93ReadVal, crate::regs::Prio93WriteVal>;
    pub type Prio94 =
        ureg::ReadWriteReg32<0, crate::regs::Prio94ReadVal, crate::regs::Prio94WriteVal>;
    pub type Prio95 =
        ureg::ReadWriteReg32<0, crate::regs::Prio95ReadVal, crate::regs::Prio95WriteVal>;
    pub type Prio96 =
        ureg::ReadWriteReg32<0, crate::regs::Prio96ReadVal, crate::regs::Prio96WriteVal>;
    pub type Prio97 =
        ureg::ReadWriteReg32<0, crate::regs::Prio97ReadVal, crate::regs::Prio97WriteVal>;
    pub type Prio98 =
        ureg::ReadWriteReg32<0, crate::regs::Prio98ReadVal, crate::regs::Prio98WriteVal>;
    pub type Prio99 =
        ureg::ReadWriteReg32<0, crate::regs::Prio99ReadVal, crate::regs::Prio99WriteVal>;
    pub type Prio100 =
        ureg::ReadWriteReg32<0, crate::regs::Prio100ReadVal, crate::regs::Prio100WriteVal>;
    pub type Prio101 =
        ureg::ReadWriteReg32<0, crate::regs::Prio101ReadVal, crate::regs::Prio101WriteVal>;
    pub type Prio102 =
        ureg::ReadWriteReg32<0, crate::regs::Prio102ReadVal, crate::regs::Prio102WriteVal>;
    pub type Prio103 =
        ureg::ReadWriteReg32<0, crate::regs::Prio103ReadVal, crate::regs::Prio103WriteVal>;
    pub type Prio104 =
        ureg::ReadWriteReg32<0, crate::regs::Prio104ReadVal, crate::regs::Prio104WriteVal>;
    pub type Prio105 =
        ureg::ReadWriteReg32<0, crate::regs::Prio105ReadVal, crate::regs::Prio105WriteVal>;
    pub type Prio106 =
        ureg::ReadWriteReg32<0, crate::regs::Prio106ReadVal, crate::regs::Prio106WriteVal>;
    pub type Prio107 =
        ureg::ReadWriteReg32<0, crate::regs::Prio107ReadVal, crate::regs::Prio107WriteVal>;
    pub type Prio108 =
        ureg::ReadWriteReg32<0, crate::regs::Prio108ReadVal, crate::regs::Prio108WriteVal>;
    pub type Prio109 =
        ureg::ReadWriteReg32<0, crate::regs::Prio109ReadVal, crate::regs::Prio109WriteVal>;
    pub type Prio110 =
        ureg::ReadWriteReg32<0, crate::regs::Prio110ReadVal, crate::regs::Prio110WriteVal>;
    pub type Prio111 =
        ureg::ReadWriteReg32<0, crate::regs::Prio111ReadVal, crate::regs::Prio111WriteVal>;
    pub type Prio112 =
        ureg::ReadWriteReg32<0, crate::regs::Prio112ReadVal, crate::regs::Prio112WriteVal>;
    pub type Prio113 =
        ureg::ReadWriteReg32<0, crate::regs::Prio113ReadVal, crate::regs::Prio113WriteVal>;
    pub type Prio114 =
        ureg::ReadWriteReg32<0, crate::regs::Prio114ReadVal, crate::regs::Prio114WriteVal>;
    pub type Prio115 =
        ureg::ReadWriteReg32<0, crate::regs::Prio115ReadVal, crate::regs::Prio115WriteVal>;
    pub type Prio116 =
        ureg::ReadWriteReg32<0, crate::regs::Prio116ReadVal, crate::regs::Prio116WriteVal>;
    pub type Prio117 =
        ureg::ReadWriteReg32<0, crate::regs::Prio117ReadVal, crate::regs::Prio117WriteVal>;
    pub type Prio118 =
        ureg::ReadWriteReg32<0, crate::regs::Prio118ReadVal, crate::regs::Prio118WriteVal>;
    pub type Prio119 =
        ureg::ReadWriteReg32<0, crate::regs::Prio119ReadVal, crate::regs::Prio119WriteVal>;
    pub type Prio120 =
        ureg::ReadWriteReg32<0, crate::regs::Prio120ReadVal, crate::regs::Prio120WriteVal>;
    pub type Prio121 =
        ureg::ReadWriteReg32<0, crate::regs::Prio121ReadVal, crate::regs::Prio121WriteVal>;
    pub type Prio122 =
        ureg::ReadWriteReg32<0, crate::regs::Prio122ReadVal, crate::regs::Prio122WriteVal>;
    pub type Prio123 =
        ureg::ReadWriteReg32<0, crate::regs::Prio123ReadVal, crate::regs::Prio123WriteVal>;
    pub type Prio124 =
        ureg::ReadWriteReg32<0, crate::regs::Prio124ReadVal, crate::regs::Prio124WriteVal>;
    pub type Prio125 =
        ureg::ReadWriteReg32<0, crate::regs::Prio125ReadVal, crate::regs::Prio125WriteVal>;
    pub type Prio126 =
        ureg::ReadWriteReg32<0, crate::regs::Prio126ReadVal, crate::regs::Prio126WriteVal>;
    pub type Prio127 =
        ureg::ReadWriteReg32<0, crate::regs::Prio127ReadVal, crate::regs::Prio127WriteVal>;
    pub type Prio128 =
        ureg::ReadWriteReg32<0, crate::regs::Prio128ReadVal, crate::regs::Prio128WriteVal>;
    pub type Prio129 =
        ureg::ReadWriteReg32<0, crate::regs::Prio129ReadVal, crate::regs::Prio129WriteVal>;
    pub type Prio130 =
        ureg::ReadWriteReg32<0, crate::regs::Prio130ReadVal, crate::regs::Prio130WriteVal>;
    pub type Prio131 =
        ureg::ReadWriteReg32<0, crate::regs::Prio131ReadVal, crate::regs::Prio131WriteVal>;
    pub type Prio132 =
        ureg::ReadWriteReg32<0, crate::regs::Prio132ReadVal, crate::regs::Prio132WriteVal>;
    pub type Prio133 =
        ureg::ReadWriteReg32<0, crate::regs::Prio133ReadVal, crate::regs::Prio133WriteVal>;
    pub type Prio134 =
        ureg::ReadWriteReg32<0, crate::regs::Prio134ReadVal, crate::regs::Prio134WriteVal>;
    pub type Prio135 =
        ureg::ReadWriteReg32<0, crate::regs::Prio135ReadVal, crate::regs::Prio135WriteVal>;
    pub type Prio136 =
        ureg::ReadWriteReg32<0, crate::regs::Prio136ReadVal, crate::regs::Prio136WriteVal>;
    pub type Prio137 =
        ureg::ReadWriteReg32<0, crate::regs::Prio137ReadVal, crate::regs::Prio137WriteVal>;
    pub type Prio138 =
        ureg::ReadWriteReg32<0, crate::regs::Prio138ReadVal, crate::regs::Prio138WriteVal>;
    pub type Prio139 =
        ureg::ReadWriteReg32<0, crate::regs::Prio139ReadVal, crate::regs::Prio139WriteVal>;
    pub type Prio140 =
        ureg::ReadWriteReg32<0, crate::regs::Prio140ReadVal, crate::regs::Prio140WriteVal>;
    pub type Prio141 =
        ureg::ReadWriteReg32<0, crate::regs::Prio141ReadVal, crate::regs::Prio141WriteVal>;
    pub type Prio142 =
        ureg::ReadWriteReg32<0, crate::regs::Prio142ReadVal, crate::regs::Prio142WriteVal>;
    pub type Prio143 =
        ureg::ReadWriteReg32<0, crate::regs::Prio143ReadVal, crate::regs::Prio143WriteVal>;
    pub type Prio144 =
        ureg::ReadWriteReg32<0, crate::regs::Prio144ReadVal, crate::regs::Prio144WriteVal>;
    pub type Prio145 =
        ureg::ReadWriteReg32<0, crate::regs::Prio145ReadVal, crate::regs::Prio145WriteVal>;
    pub type Prio146 =
        ureg::ReadWriteReg32<0, crate::regs::Prio146ReadVal, crate::regs::Prio146WriteVal>;
    pub type Prio147 =
        ureg::ReadWriteReg32<0, crate::regs::Prio147ReadVal, crate::regs::Prio147WriteVal>;
    pub type Prio148 =
        ureg::ReadWriteReg32<0, crate::regs::Prio148ReadVal, crate::regs::Prio148WriteVal>;
    pub type Prio149 =
        ureg::ReadWriteReg32<0, crate::regs::Prio149ReadVal, crate::regs::Prio149WriteVal>;
    pub type Prio150 =
        ureg::ReadWriteReg32<0, crate::regs::Prio150ReadVal, crate::regs::Prio150WriteVal>;
    pub type Prio151 =
        ureg::ReadWriteReg32<0, crate::regs::Prio151ReadVal, crate::regs::Prio151WriteVal>;
    pub type Prio152 =
        ureg::ReadWriteReg32<0, crate::regs::Prio152ReadVal, crate::regs::Prio152WriteVal>;
    pub type Prio153 =
        ureg::ReadWriteReg32<0, crate::regs::Prio153ReadVal, crate::regs::Prio153WriteVal>;
    pub type Prio154 =
        ureg::ReadWriteReg32<0, crate::regs::Prio154ReadVal, crate::regs::Prio154WriteVal>;
    pub type Prio155 =
        ureg::ReadWriteReg32<0, crate::regs::Prio155ReadVal, crate::regs::Prio155WriteVal>;
    pub type Prio156 =
        ureg::ReadWriteReg32<0, crate::regs::Prio156ReadVal, crate::regs::Prio156WriteVal>;
    pub type Prio157 =
        ureg::ReadWriteReg32<0, crate::regs::Prio157ReadVal, crate::regs::Prio157WriteVal>;
    pub type Prio158 =
        ureg::ReadWriteReg32<0, crate::regs::Prio158ReadVal, crate::regs::Prio158WriteVal>;
    pub type Prio159 =
        ureg::ReadWriteReg32<0, crate::regs::Prio159ReadVal, crate::regs::Prio159WriteVal>;
    pub type Prio160 =
        ureg::ReadWriteReg32<0, crate::regs::Prio160ReadVal, crate::regs::Prio160WriteVal>;
    pub type Prio161 =
        ureg::ReadWriteReg32<0, crate::regs::Prio161ReadVal, crate::regs::Prio161WriteVal>;
    pub type Prio162 =
        ureg::ReadWriteReg32<0, crate::regs::Prio162ReadVal, crate::regs::Prio162WriteVal>;
    pub type Prio163 =
        ureg::ReadWriteReg32<0, crate::regs::Prio163ReadVal, crate::regs::Prio163WriteVal>;
    pub type Prio164 =
        ureg::ReadWriteReg32<0, crate::regs::Prio164ReadVal, crate::regs::Prio164WriteVal>;
    pub type Prio165 =
        ureg::ReadWriteReg32<0, crate::regs::Prio165ReadVal, crate::regs::Prio165WriteVal>;
    pub type Prio166 =
        ureg::ReadWriteReg32<0, crate::regs::Prio166ReadVal, crate::regs::Prio166WriteVal>;
    pub type Prio167 =
        ureg::ReadWriteReg32<0, crate::regs::Prio167ReadVal, crate::regs::Prio167WriteVal>;
    pub type Prio168 =
        ureg::ReadWriteReg32<0, crate::regs::Prio168ReadVal, crate::regs::Prio168WriteVal>;
    pub type Prio169 =
        ureg::ReadWriteReg32<0, crate::regs::Prio169ReadVal, crate::regs::Prio169WriteVal>;
    pub type Prio170 =
        ureg::ReadWriteReg32<0, crate::regs::Prio170ReadVal, crate::regs::Prio170WriteVal>;
    pub type Prio171 =
        ureg::ReadWriteReg32<0, crate::regs::Prio171ReadVal, crate::regs::Prio171WriteVal>;
    pub type Prio172 =
        ureg::ReadWriteReg32<0, crate::regs::Prio172ReadVal, crate::regs::Prio172WriteVal>;
    pub type Prio173 =
        ureg::ReadWriteReg32<0, crate::regs::Prio173ReadVal, crate::regs::Prio173WriteVal>;
    pub type Prio174 =
        ureg::ReadWriteReg32<0, crate::regs::Prio174ReadVal, crate::regs::Prio174WriteVal>;
    pub type Prio175 =
        ureg::ReadWriteReg32<0, crate::regs::Prio175ReadVal, crate::regs::Prio175WriteVal>;
    pub type Prio176 =
        ureg::ReadWriteReg32<0, crate::regs::Prio176ReadVal, crate::regs::Prio176WriteVal>;
    pub type Prio177 =
        ureg::ReadWriteReg32<0, crate::regs::Prio177ReadVal, crate::regs::Prio177WriteVal>;
    pub type Prio178 =
        ureg::ReadWriteReg32<0, crate::regs::Prio178ReadVal, crate::regs::Prio178WriteVal>;
    pub type Prio179 =
        ureg::ReadWriteReg32<0, crate::regs::Prio179ReadVal, crate::regs::Prio179WriteVal>;
    pub type Prio180 =
        ureg::ReadWriteReg32<0, crate::regs::Prio180ReadVal, crate::regs::Prio180WriteVal>;
    pub type Prio181 =
        ureg::ReadWriteReg32<0, crate::regs::Prio181ReadVal, crate::regs::Prio181WriteVal>;
    pub type Prio182 =
        ureg::ReadWriteReg32<0, crate::regs::Prio182ReadVal, crate::regs::Prio182WriteVal>;
    pub type Prio183 =
        ureg::ReadWriteReg32<0, crate::regs::Prio183ReadVal, crate::regs::Prio183WriteVal>;
    pub type Prio184 =
        ureg::ReadWriteReg32<0, crate::regs::Prio184ReadVal, crate::regs::Prio184WriteVal>;
    pub type Prio185 =
        ureg::ReadWriteReg32<0, crate::regs::Prio185ReadVal, crate::regs::Prio185WriteVal>;
    pub type Ip0 = ureg::ReadOnlyReg32<crate::regs::Ip0ReadVal>;
    pub type Ip1 = ureg::ReadOnlyReg32<crate::regs::Ip1ReadVal>;
    pub type Ip2 = ureg::ReadOnlyReg32<crate::regs::Ip2ReadVal>;
    pub type Ip3 = ureg::ReadOnlyReg32<crate::regs::Ip3ReadVal>;
    pub type Ip4 = ureg::ReadOnlyReg32<crate::regs::Ip4ReadVal>;
    pub type Ip5 = ureg::ReadOnlyReg32<crate::regs::Ip5ReadVal>;
    pub type Ie0 = ureg::ReadWriteReg32<0, crate::regs::Ie0ReadVal, crate::regs::Ie0WriteVal>;
    pub type Ie1 = ureg::ReadWriteReg32<0, crate::regs::Ie1ReadVal, crate::regs::Ie1WriteVal>;
    pub type Ie2 = ureg::ReadWriteReg32<0, crate::regs::Ie2ReadVal, crate::regs::Ie2WriteVal>;
    pub type Ie3 = ureg::ReadWriteReg32<0, crate::regs::Ie3ReadVal, crate::regs::Ie3WriteVal>;
    pub type Ie4 = ureg::ReadWriteReg32<0, crate::regs::Ie4ReadVal, crate::regs::Ie4WriteVal>;
    pub type Ie5 = ureg::ReadWriteReg32<0, crate::regs::Ie5ReadVal, crate::regs::Ie5WriteVal>;
    pub type Threshold0 =
        ureg::ReadWriteReg32<0, crate::regs::Threshold0ReadVal, crate::regs::Threshold0WriteVal>;
    pub type Cc0 = ureg::ReadWriteReg32<0, crate::regs::Cc0ReadVal, crate::regs::Cc0WriteVal>;
    pub type Msip0 = ureg::ReadWriteReg32<0, crate::regs::Msip0ReadVal, crate::regs::Msip0WriteVal>;
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
}

#![no_std]
#![allow(clippy::erasing_op)]
#![allow(clippy::identity_op)]
#[doc = r" A zero-sized type that represents ownership of this"]
#[doc = r" peripheral, used to get access to a Register lock. Most"]
#[doc = r" programs create one of these in unsafe code near the top of"]
#[doc = r" main(), and pass it to the driver responsible for managing"]
#[doc = r" all access to the hardware."]
pub struct SpiDevice {
    _priv: (),
}
impl SpiDevice {
    pub const PTR: *mut u32 = 0x40050000 as *mut u32;
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
    #[doc = "Interrupt State Register\n\nRead value: [`regs::IntrStateReadVal`]; Write value: [`regs::IntrStateWriteVal`]"]
    #[inline(always)]
    pub fn intr_state(&self) -> ureg::RegRef<crate::meta::IntrState, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Enable Register\n\nRead value: [`regs::IntrEnableReadVal`]; Write value: [`regs::IntrEnableWriteVal`]"]
    #[inline(always)]
    pub fn intr_enable(&self) -> ureg::RegRef<crate::meta::IntrEnable, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Interrupt Test Register\n\nRead value: [`regs::IntrTestReadVal`]; Write value: [`regs::IntrTestWriteVal`]"]
    #[inline(always)]
    pub fn intr_test(&self) -> ureg::RegRef<crate::meta::IntrTest, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Alert Test Register\n\nRead value: [`regs::AlertTestReadVal`]; Write value: [`regs::AlertTestWriteVal`]"]
    #[inline(always)]
    pub fn alert_test(&self) -> ureg::RegRef<crate::meta::AlertTest, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Control register\n\nRead value: [`regs::ControlReadVal`]; Write value: [`regs::ControlWriteVal`]"]
    #[inline(always)]
    pub fn control(&self) -> ureg::RegRef<crate::meta::Control, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x10 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Configuration Register\n\nRead value: [`regs::CfgReadVal`]; Write value: [`regs::CfgWriteVal`]"]
    #[inline(always)]
    pub fn cfg(&self) -> ureg::RegRef<crate::meta::Cfg, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x14 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "SPI Device status register\n\nRead value: [`regs::StatusReadVal`]; Write value: [`regs::StatusWriteVal`]"]
    #[inline(always)]
    pub fn status(&self) -> ureg::RegRef<crate::meta::Status, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x18 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Intercept Passthrough datapath.\n\n\nRead value: [`regs::InterceptEnReadVal`]; Write value: [`regs::InterceptEnWriteVal`]"]
    #[inline(always)]
    pub fn intercept_en(&self) -> ureg::RegRef<crate::meta::InterceptEn, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x1c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Flash address mode configuration\n\nThis register shows the current address mode and pending changes.\nIt is updated by the HW when the command phase completes.\n\nRead value: [`regs::AddrModeReadVal`]; Write value: [`regs::AddrModeWriteVal`]"]
    #[inline(always)]
    pub fn addr_mode(&self) -> ureg::RegRef<crate::meta::AddrMode, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x20 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Last Read Address\n\nThis register shows the last address accessed by the host system.\nIt is updated by the HW when CSb is de-asserted.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn last_read_addr(&self) -> ureg::RegRef<crate::meta::LastReadAddr, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x24 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "SPI Flash Status register.\n\nThis register emulates the SPI Flash Status 3, 2, 1 registers.\nbit [7:0] is for Status register, bit [15:8] is for Status-2 register,\nand bit [23:16] is for Status-3 register. It is SW responsibility to\nmaintain this register value up to date.\n\nThe HW latches the value when SPI Flash transaction begins. Any updates\nduring the transaction will be updated after the transaction is\ncompleted.\n\nRead value: [`regs::FlashStatusReadVal`]; Write value: [`regs::FlashStatusWriteVal`]"]
    #[inline(always)]
    pub fn flash_status(&self) -> ureg::RegRef<crate::meta::FlashStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x28 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "JEDEC Continuation Code configuration register.\n\nRead JEDEC ID must return the continuation code if the manufacturer ID\nis not shown in the first page of JEDEC table. This register controls\nthe Continuation Code.\n\nRead value: [`regs::JedecCcReadVal`]; Write value: [`regs::JedecCcWriteVal`]"]
    #[inline(always)]
    pub fn jedec_cc(&self) -> ureg::RegRef<crate::meta::JedecCc, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x2c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "JEDEC ID register.\n\nRead value: [`regs::JedecIdReadVal`]; Write value: [`regs::JedecIdWriteVal`]"]
    #[inline(always)]
    pub fn jedec_id(&self) -> ureg::RegRef<crate::meta::JedecId, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x30 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Read Buffer threshold register.\n\n\nRead value: [`regs::ReadThresholdReadVal`]; Write value: [`regs::ReadThresholdWriteVal`]"]
    #[inline(always)]
    pub fn read_threshold(&self) -> ureg::RegRef<crate::meta::ReadThreshold, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x34 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Mailbox Base address register.\n\nThe mailbox size is fixed. In this version of IP, the size is 1kB.\nLower 10 bits of the Mailbox address is tied to 0.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn mailbox_addr(&self) -> ureg::RegRef<crate::meta::MailboxAddr, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x38 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Upload module status register.\n\nRead value: [`regs::UploadStatusReadVal`]; Write value: [`regs::UploadStatusWriteVal`]"]
    #[inline(always)]
    pub fn upload_status(&self) -> ureg::RegRef<crate::meta::UploadStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x3c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Upload module status 2 register.\n\nThis register contains payload related status. payload_depth indicates\nthe payload size (from 0 to 256 bytes).\n\npayload_start_idx indicates the start of the 256B. This stays 0\nusually. However, when the SPI host system issues more than 256B of\npayload in a command, this field may not be 0. For example, if the\nsystem issues 258B payload, the payload_depth is 256 (as the IP only\nholds 256B of payload), the payload_start_idx is 2. SW should read from\n2 to 255 then 0 and 1.\n\nRead value: [`regs::UploadStatus2ReadVal`]; Write value: [`regs::UploadStatus2WriteVal`]"]
    #[inline(always)]
    pub fn upload_status2(&self) -> ureg::RegRef<crate::meta::UploadStatus2, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x40 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command Fifo Read Port.\n\nRead value: [`regs::UploadCmdfifoReadVal`]; Write value: [`regs::UploadCmdfifoWriteVal`]"]
    #[inline(always)]
    pub fn upload_cmdfifo(&self) -> ureg::RegRef<crate::meta::UploadCmdfifo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x44 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Address Fifo Read Port.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn upload_addrfifo(&self) -> ureg::RegRef<crate::meta::UploadAddrfifo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x48 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command Filter\n\nIf a bit in this CSR is 1, then corresponding SPI command w.r.t the\nbit position among 256 bit is dropped in SPI Passthrough mode.\n\nRead value: [`regs::CmdFilter0ReadVal`]; Write value: [`regs::CmdFilter0WriteVal`]"]
    #[inline(always)]
    pub fn cmd_filter0(&self) -> ureg::RegRef<crate::meta::CmdFilter0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x4c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command Filter\n\nIf a bit in this CSR is 1, then corresponding SPI command w.r.t the\nbit position among 256 bit is dropped in SPI Passthrough mode.\n\nRead value: [`regs::CmdFilter1ReadVal`]; Write value: [`regs::CmdFilter1WriteVal`]"]
    #[inline(always)]
    pub fn cmd_filter1(&self) -> ureg::RegRef<crate::meta::CmdFilter1, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x50 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command Filter\n\nIf a bit in this CSR is 1, then corresponding SPI command w.r.t the\nbit position among 256 bit is dropped in SPI Passthrough mode.\n\nRead value: [`regs::CmdFilter2ReadVal`]; Write value: [`regs::CmdFilter2WriteVal`]"]
    #[inline(always)]
    pub fn cmd_filter2(&self) -> ureg::RegRef<crate::meta::CmdFilter2, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x54 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command Filter\n\nIf a bit in this CSR is 1, then corresponding SPI command w.r.t the\nbit position among 256 bit is dropped in SPI Passthrough mode.\n\nRead value: [`regs::CmdFilter3ReadVal`]; Write value: [`regs::CmdFilter3WriteVal`]"]
    #[inline(always)]
    pub fn cmd_filter3(&self) -> ureg::RegRef<crate::meta::CmdFilter3, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x58 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command Filter\n\nIf a bit in this CSR is 1, then corresponding SPI command w.r.t the\nbit position among 256 bit is dropped in SPI Passthrough mode.\n\nRead value: [`regs::CmdFilter4ReadVal`]; Write value: [`regs::CmdFilter4WriteVal`]"]
    #[inline(always)]
    pub fn cmd_filter4(&self) -> ureg::RegRef<crate::meta::CmdFilter4, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x5c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command Filter\n\nIf a bit in this CSR is 1, then corresponding SPI command w.r.t the\nbit position among 256 bit is dropped in SPI Passthrough mode.\n\nRead value: [`regs::CmdFilter5ReadVal`]; Write value: [`regs::CmdFilter5WriteVal`]"]
    #[inline(always)]
    pub fn cmd_filter5(&self) -> ureg::RegRef<crate::meta::CmdFilter5, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x60 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command Filter\n\nIf a bit in this CSR is 1, then corresponding SPI command w.r.t the\nbit position among 256 bit is dropped in SPI Passthrough mode.\n\nRead value: [`regs::CmdFilter6ReadVal`]; Write value: [`regs::CmdFilter6WriteVal`]"]
    #[inline(always)]
    pub fn cmd_filter6(&self) -> ureg::RegRef<crate::meta::CmdFilter6, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x64 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command Filter\n\nIf a bit in this CSR is 1, then corresponding SPI command w.r.t the\nbit position among 256 bit is dropped in SPI Passthrough mode.\n\nRead value: [`regs::CmdFilter7ReadVal`]; Write value: [`regs::CmdFilter7WriteVal`]"]
    #[inline(always)]
    pub fn cmd_filter7(&self) -> ureg::RegRef<crate::meta::CmdFilter7, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x68 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Address Swap Mask register.\n\nThis register is used in the SPI passthrough mode. If any of bits in\nthis register is set, the corresponding address bit in the SPI Read\ncommands is replaced with the data from !!ADDR_SWAP_DATA.\n\nIf 3B address mode is active, upper 8bit [31:24] is ignored.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn addr_swap_mask(&self) -> ureg::RegRef<crate::meta::AddrSwapMask, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x6c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "The address value for the address swap feature.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn addr_swap_data(&self) -> ureg::RegRef<crate::meta::AddrSwapData, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x70 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Write Data Swap in the passthrough mode.\n\nPAYLOAD_SWAP_MASK CSR provides the SW to change certain bits in the\nfirst 4 bytes of the write payload in the passthrough mode.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn payload_swap_mask(&self) -> ureg::RegRef<crate::meta::PayloadSwapMask, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x74 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Write Data Swap in the passthrough mode.\n\nPAYLOAD_SWAP_DATA combined with PAYLOAD_SWAP_MASK provides the SW to\nchange certain bits in the first 4 bytes of the write payload in the\npassthrough mode.\n\nThe register should be written in Little-Endian order. [7:0] bits are\nprocessed in the first received payload byte. [31:24] bits for the 4th\nbyte.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn payload_swap_data(&self) -> ureg::RegRef<crate::meta::PayloadSwapData, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x78 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Command Info register.\n\n\nRead value: [`regs::CmdInfoReadVal`]; Write value: [`regs::CmdInfoWriteVal`]"]
    #[inline(always)]
    pub fn cmd_info(&self) -> ureg::Array<24, ureg::RegRef<crate::meta::CmdInfo, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x7c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Opcode for EN4B.\n\nIf the register is active, it affects in flash / passthrough modes.\n\nRead value: [`regs::CmdInfoEn4bReadVal`]; Write value: [`regs::CmdInfoEn4bWriteVal`]"]
    #[inline(always)]
    pub fn cmd_info_en4_b(&self) -> ureg::RegRef<crate::meta::CmdInfoEn4b, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xdc / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Opcode for EX4B\n\nRead value: [`regs::CmdInfoEx4bReadVal`]; Write value: [`regs::CmdInfoEx4bWriteVal`]"]
    #[inline(always)]
    pub fn cmd_info_ex4_b(&self) -> ureg::RegRef<crate::meta::CmdInfoEx4b, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xe0 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Opcode for Write Enable (WREN)\n\nRead value: [`regs::CmdInfoWrenReadVal`]; Write value: [`regs::CmdInfoWrenWriteVal`]"]
    #[inline(always)]
    pub fn cmd_info_wren(&self) -> ureg::RegRef<crate::meta::CmdInfoWren, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xe4 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "Opcode for Write Disable (WRDI)\n\nRead value: [`regs::CmdInfoWrdiReadVal`]; Write value: [`regs::CmdInfoWrdiWriteVal`]"]
    #[inline(always)]
    pub fn cmd_info_wrdi(&self) -> ureg::RegRef<crate::meta::CmdInfoWrdi, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0xe8 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM HWIP Capability register.\n\nThis register shows the features the current TPM HWIP supports.\n\nRead value: [`regs::TpmCapReadVal`]; Write value: [`regs::TpmCapWriteVal`]"]
    #[inline(always)]
    pub fn tpm_cap(&self) -> ureg::RegRef<crate::meta::TpmCap, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x800 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM Configuration register.\n\nRead value: [`regs::TpmCfgReadVal`]; Write value: [`regs::TpmCfgWriteVal`]"]
    #[inline(always)]
    pub fn tpm_cfg(&self) -> ureg::RegRef<crate::meta::TpmCfg, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x804 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM submodule state register.\n\nThe TPM_STATUS CSR provides the current TPM status, mostly the buffer and FIFO status.\n\nRead value: [`regs::TpmStatusReadVal`]; Write value: [`regs::TpmStatusWriteVal`]"]
    #[inline(always)]
    pub fn tpm_status(&self) -> ureg::RegRef<crate::meta::TpmStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x808 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM_ACCESS_x register.\n\nRead value: [`regs::TpmAccess0ReadVal`]; Write value: [`regs::TpmAccess0WriteVal`]"]
    #[inline(always)]
    pub fn tpm_access0(&self) -> ureg::RegRef<crate::meta::TpmAccess0, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x80c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM_ACCESS_x register.\n\nRead value: [`regs::TpmAccess1ReadVal`]; Write value: [`regs::TpmAccess1WriteVal`]"]
    #[inline(always)]
    pub fn tpm_access1(&self) -> ureg::RegRef<crate::meta::TpmAccess1, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x810 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM_STS_x register.\n\nThe register is mirrored to all Localities.\nThe value is returned to the host system only when the activeLocality\nin the TPM_ACCESS_x is matched to the current received Locality.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn tpm_sts(&self) -> ureg::RegRef<crate::meta::TpmSts, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x814 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM_INTF_CAPABILITY\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn tpm_intf_capability(&self) -> ureg::RegRef<crate::meta::TpmIntfCapability, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x818 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM_INT_ENABLE\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn tpm_int_enable(&self) -> ureg::RegRef<crate::meta::TpmIntEnable, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x81c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM_INT_VECTOR\n\nRead value: [`regs::TpmIntVectorReadVal`]; Write value: [`regs::TpmIntVectorWriteVal`]"]
    #[inline(always)]
    pub fn tpm_int_vector(&self) -> ureg::RegRef<crate::meta::TpmIntVector, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x820 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM_INT_STATUS\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn tpm_int_status(&self) -> ureg::RegRef<crate::meta::TpmIntStatus, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x824 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM_DID/ TPM_VID register\n\nRead value: [`regs::TpmDidVidReadVal`]; Write value: [`regs::TpmDidVidWriteVal`]"]
    #[inline(always)]
    pub fn tpm_did_vid(&self) -> ureg::RegRef<crate::meta::TpmDidVid, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x828 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM_RID\n\nRead value: [`regs::TpmRidReadVal`]; Write value: [`regs::TpmRidWriteVal`]"]
    #[inline(always)]
    pub fn tpm_rid(&self) -> ureg::RegRef<crate::meta::TpmRid, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x82c / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM Command and Address buffer\n\nThe SW may get the received TPM command and address by readin gthis CSR.\n\nRead value: [`regs::TpmCmdAddrReadVal`]; Write value: [`regs::TpmCmdAddrWriteVal`]"]
    #[inline(always)]
    pub fn tpm_cmd_addr(&self) -> ureg::RegRef<crate::meta::TpmCmdAddr, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x830 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "TPM Read command return data FIFO.\n\nThe write port of the read command FIFO.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn tpm_read_fifo(&self) -> ureg::RegRef<crate::meta::TpmReadFifo, &TMmio> {
        unsafe {
            ureg::RegRef::new_with_mmio(
                self.ptr.wrapping_add(0x834 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "SPI internal egress buffer.\n\nThe lower 2 kB is for Read content emulating eFlash.\nThe next 1 kB is for the Mailbox buffer.\nThen the next 256 B is for the SFDP buffer.\nFinally, the buffer spaces end with a 64 B TPM Read FIFO.\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn egress_buffer(
        &self,
    ) -> ureg::Array<848, ureg::RegRef<crate::meta::EgressBuffer, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x1000 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
    #[doc = "SPI internal ingress buffer.\n\nThe layout is as follows (starting from offset 0):\n- 256 B SFDP buffer\n- 32 B CmdFIFO\n- 32 B AddrFIFO\n- 256 B payload FIFO\n- 64 B TPM Write FIFO\n\nRead value: [`u32`]; Write value: [`u32`]"]
    #[inline(always)]
    pub fn ingress_buffer(
        &self,
    ) -> ureg::Array<112, ureg::RegRef<crate::meta::IngressBuffer, &TMmio>> {
        unsafe {
            ureg::Array::new_with_mmio(
                self.ptr.wrapping_add(0x1e00 / core::mem::size_of::<u32>()),
                core::borrow::Borrow::borrow(&self.mmio),
            )
        }
    }
}
pub mod regs {
    #![doc = r" Types that represent the values held by registers."]
    #[derive(Clone, Copy)]
    pub struct AddrModeReadVal(u32);
    impl AddrModeReadVal {
        #[doc = "4B Address Mode enable.\n\nThis field configures the internal module to receive 32 bits of the SPI commands.\nThe affected commands are the SPI read commands except QPI, and program commands.\nIt is expected for SW to configure this field at the configuration stage and release control to HW until the next reset.\n\nEven though Read SFDP command has address fields, the SFDP command is not affected by this field.\nThe command always parse 24 bits on the SPI line 0 following the SPI command as the address field.\n\nThis field has noteworthy read behavior.\nIf a software-initiated change is still `pending` the sync to the SPI domain, this bit will reflect the value to be sent.\nOtherwise, this field will reflect the current value observed in the SPI domain."]
        #[inline(always)]
        pub fn addr_4b_en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "SW-originated change is pending.\n\nThis bit is 1 whenever the current value of addr_4b_en has yet to sync with the SPI domain.\nIf an EN4B or EX4B command arrives next, the current value in `addr_4b_en` will be ignored,\nand the SPI flash command will take priority, with an update to `addr_4b_en` to match the command's result."]
        #[inline(always)]
        pub fn pending(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> AddrModeWriteVal {
            AddrModeWriteVal(self.0)
        }
    }
    impl From<u32> for AddrModeReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AddrModeReadVal> for u32 {
        #[inline(always)]
        fn from(val: AddrModeReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AddrModeWriteVal(u32);
    impl AddrModeWriteVal {
        #[doc = "4B Address Mode enable.\n\nThis field configures the internal module to receive 32 bits of the SPI commands.\nThe affected commands are the SPI read commands except QPI, and program commands.\nIt is expected for SW to configure this field at the configuration stage and release control to HW until the next reset.\n\nEven though Read SFDP command has address fields, the SFDP command is not affected by this field.\nThe command always parse 24 bits on the SPI line 0 following the SPI command as the address field.\n\nThis field has noteworthy read behavior.\nIf a software-initiated change is still `pending` the sync to the SPI domain, this bit will reflect the value to be sent.\nOtherwise, this field will reflect the current value observed in the SPI domain."]
        #[inline(always)]
        pub fn addr_4b_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
    }
    impl From<u32> for AddrModeWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<AddrModeWriteVal> for u32 {
        #[inline(always)]
        fn from(val: AddrModeWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct AlertTestWriteVal(u32);
    impl AlertTestWriteVal {
        #[doc = "Write 1 to trigger one alert event of this kind."]
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
    pub struct CfgReadVal(u32);
    impl CfgReadVal {
        #[doc = "TX bit order on SDO. 0 for MSB to LSB, 1 for LSB to MSB"]
        #[inline(always)]
        pub fn tx_order(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "RX bit order on SDI. Module stores bitstream from MSB to LSB if value is 0."]
        #[inline(always)]
        pub fn rx_order(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Mailbox enable.\n\nIf 1, in the flash and passthrough mode, the IP checks the incoming\naddress and return from the internal Mailbox buffer if the address\nfalls into the MAILBOX range\n(MAILBOX_ADDR:MAILBOX_ADDR+MAILBOX_SIZE)}."]
        #[inline(always)]
        pub fn mailbox_en(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CfgWriteVal {
            CfgWriteVal(self.0)
        }
    }
    impl From<u32> for CfgReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgReadVal> for u32 {
        #[inline(always)]
        fn from(val: CfgReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CfgWriteVal(u32);
    impl CfgWriteVal {
        #[doc = "TX bit order on SDO. 0 for MSB to LSB, 1 for LSB to MSB"]
        #[inline(always)]
        pub fn tx_order(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "RX bit order on SDI. Module stores bitstream from MSB to LSB if value is 0."]
        #[inline(always)]
        pub fn rx_order(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Mailbox enable.\n\nIf 1, in the flash and passthrough mode, the IP checks the incoming\naddress and return from the internal Mailbox buffer if the address\nfalls into the MAILBOX range\n(MAILBOX_ADDR:MAILBOX_ADDR+MAILBOX_SIZE)}."]
        #[inline(always)]
        pub fn mailbox_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
    }
    impl From<u32> for CfgWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CfgWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CfgWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter0ReadVal(u32);
    impl CmdFilter0ReadVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter0(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter1(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter2(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter3(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter4(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter5(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter6(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter7(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter8(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter9(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter10(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter11(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter12(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter13(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter14(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter15(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter16(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter17(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter18(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter19(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter20(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter21(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter22(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter23(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter24(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter25(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter26(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter27(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter28(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter29(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter30(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter31(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdFilter0WriteVal {
            CmdFilter0WriteVal(self.0)
        }
    }
    impl From<u32> for CmdFilter0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter0WriteVal(u32);
    impl CmdFilter0WriteVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter0(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter1(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter2(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter3(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter4(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter5(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter6(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter7(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter8(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter9(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter10(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter11(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter12(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter13(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter14(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter15(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter16(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter17(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter18(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter19(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter20(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter21(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter22(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter23(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter24(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter25(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter26(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter27(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter28(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter29(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter30(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter31(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdFilter0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter1ReadVal(u32);
    impl CmdFilter1ReadVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter32(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter33(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter34(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter35(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter36(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter37(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter38(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter39(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter40(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter41(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter42(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter43(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter44(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter45(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter46(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter47(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter48(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter49(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter50(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter51(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter52(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter53(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter54(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter55(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter56(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter57(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter58(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter59(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter60(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter61(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter62(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter63(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdFilter1WriteVal {
            CmdFilter1WriteVal(self.0)
        }
    }
    impl From<u32> for CmdFilter1ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter1ReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter1ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter1WriteVal(u32);
    impl CmdFilter1WriteVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter32(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter33(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter34(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter35(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter36(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter37(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter38(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter39(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter40(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter41(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter42(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter43(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter44(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter45(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter46(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter47(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter48(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter49(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter50(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter51(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter52(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter53(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter54(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter55(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter56(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter57(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter58(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter59(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter60(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter61(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter62(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter63(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdFilter1WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter1WriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter1WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter2ReadVal(u32);
    impl CmdFilter2ReadVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter64(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter65(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter66(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter67(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter68(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter69(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter70(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter71(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter72(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter73(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter74(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter75(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter76(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter77(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter78(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter79(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter80(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter81(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter82(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter83(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter84(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter85(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter86(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter87(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter88(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter89(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter90(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter91(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter92(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter93(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter94(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter95(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdFilter2WriteVal {
            CmdFilter2WriteVal(self.0)
        }
    }
    impl From<u32> for CmdFilter2ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter2ReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter2ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter2WriteVal(u32);
    impl CmdFilter2WriteVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter64(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter65(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter66(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter67(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter68(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter69(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter70(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter71(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter72(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter73(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter74(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter75(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter76(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter77(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter78(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter79(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter80(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter81(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter82(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter83(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter84(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter85(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter86(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter87(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter88(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter89(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter90(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter91(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter92(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter93(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter94(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter95(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdFilter2WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter2WriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter2WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter3ReadVal(u32);
    impl CmdFilter3ReadVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter96(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter97(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter98(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter99(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter100(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter101(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter102(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter103(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter104(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter105(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter106(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter107(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter108(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter109(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter110(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter111(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter112(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter113(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter114(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter115(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter116(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter117(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter118(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter119(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter120(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter121(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter122(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter123(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter124(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter125(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter126(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter127(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdFilter3WriteVal {
            CmdFilter3WriteVal(self.0)
        }
    }
    impl From<u32> for CmdFilter3ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter3ReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter3ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter3WriteVal(u32);
    impl CmdFilter3WriteVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter96(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter97(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter98(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter99(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter100(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter101(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter102(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter103(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter104(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter105(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter106(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter107(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter108(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter109(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter110(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter111(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter112(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter113(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter114(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter115(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter116(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter117(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter118(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter119(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter120(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter121(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter122(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter123(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter124(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter125(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter126(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter127(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdFilter3WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter3WriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter3WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter4ReadVal(u32);
    impl CmdFilter4ReadVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter128(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter129(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter130(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter131(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter132(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter133(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter134(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter135(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter136(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter137(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter138(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter139(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter140(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter141(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter142(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter143(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter144(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter145(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter146(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter147(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter148(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter149(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter150(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter151(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter152(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter153(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter154(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter155(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter156(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter157(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter158(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter159(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdFilter4WriteVal {
            CmdFilter4WriteVal(self.0)
        }
    }
    impl From<u32> for CmdFilter4ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter4ReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter4ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter4WriteVal(u32);
    impl CmdFilter4WriteVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter128(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter129(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter130(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter131(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter132(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter133(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter134(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter135(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter136(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter137(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter138(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter139(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter140(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter141(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter142(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter143(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter144(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter145(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter146(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter147(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter148(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter149(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter150(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter151(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter152(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter153(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter154(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter155(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter156(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter157(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter158(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter159(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdFilter4WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter4WriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter4WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter5ReadVal(u32);
    impl CmdFilter5ReadVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter160(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter161(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter162(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter163(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter164(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter165(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter166(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter167(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter168(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter169(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter170(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter171(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter172(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter173(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter174(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter175(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter176(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter177(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter178(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter179(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter180(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter181(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter182(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter183(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter184(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter185(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter186(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter187(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter188(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter189(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter190(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter191(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdFilter5WriteVal {
            CmdFilter5WriteVal(self.0)
        }
    }
    impl From<u32> for CmdFilter5ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter5ReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter5ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter5WriteVal(u32);
    impl CmdFilter5WriteVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter160(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter161(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter162(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter163(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter164(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter165(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter166(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter167(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter168(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter169(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter170(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter171(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter172(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter173(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter174(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter175(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter176(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter177(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter178(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter179(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter180(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter181(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter182(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter183(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter184(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter185(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter186(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter187(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter188(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter189(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter190(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter191(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdFilter5WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter5WriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter5WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter6ReadVal(u32);
    impl CmdFilter6ReadVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter192(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter193(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter194(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter195(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter196(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter197(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter198(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter199(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter200(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter201(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter202(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter203(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter204(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter205(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter206(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter207(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter208(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter209(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter210(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter211(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter212(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter213(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter214(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter215(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter216(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter217(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter218(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter219(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter220(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter221(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter222(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter223(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdFilter6WriteVal {
            CmdFilter6WriteVal(self.0)
        }
    }
    impl From<u32> for CmdFilter6ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter6ReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter6ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter6WriteVal(u32);
    impl CmdFilter6WriteVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter192(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter193(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter194(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter195(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter196(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter197(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter198(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter199(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter200(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter201(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter202(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter203(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter204(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter205(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter206(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter207(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter208(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter209(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter210(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter211(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter212(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter213(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter214(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter215(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter216(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter217(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter218(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter219(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter220(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter221(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter222(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter223(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdFilter6WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter6WriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter6WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter7ReadVal(u32);
    impl CmdFilter7ReadVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter224(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter225(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter226(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter227(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter228(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter229(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter230(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter231(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter232(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter233(&self) -> bool {
            ((self.0 >> 9) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter234(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter235(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter236(&self) -> bool {
            ((self.0 >> 12) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter237(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter238(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter239(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter240(&self) -> bool {
            ((self.0 >> 16) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter241(&self) -> bool {
            ((self.0 >> 17) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter242(&self) -> bool {
            ((self.0 >> 18) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter243(&self) -> bool {
            ((self.0 >> 19) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter244(&self) -> bool {
            ((self.0 >> 20) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter245(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter246(&self) -> bool {
            ((self.0 >> 22) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter247(&self) -> bool {
            ((self.0 >> 23) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter248(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter249(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter250(&self) -> bool {
            ((self.0 >> 26) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter251(&self) -> bool {
            ((self.0 >> 27) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter252(&self) -> bool {
            ((self.0 >> 28) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter253(&self) -> bool {
            ((self.0 >> 29) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter254(&self) -> bool {
            ((self.0 >> 30) & 1) != 0
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter255(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdFilter7WriteVal {
            CmdFilter7WriteVal(self.0)
        }
    }
    impl From<u32> for CmdFilter7ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter7ReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter7ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdFilter7WriteVal(u32);
    impl CmdFilter7WriteVal {
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter224(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter225(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter226(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter227(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter228(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter229(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter230(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter231(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter232(self, val: bool) -> Self {
            Self((self.0 & !(1 << 8)) | (u32::from(val) << 8))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter233(self, val: bool) -> Self {
            Self((self.0 & !(1 << 9)) | (u32::from(val) << 9))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter234(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter235(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter236(self, val: bool) -> Self {
            Self((self.0 & !(1 << 12)) | (u32::from(val) << 12))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter237(self, val: bool) -> Self {
            Self((self.0 & !(1 << 13)) | (u32::from(val) << 13))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter238(self, val: bool) -> Self {
            Self((self.0 & !(1 << 14)) | (u32::from(val) << 14))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter239(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter240(self, val: bool) -> Self {
            Self((self.0 & !(1 << 16)) | (u32::from(val) << 16))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter241(self, val: bool) -> Self {
            Self((self.0 & !(1 << 17)) | (u32::from(val) << 17))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter242(self, val: bool) -> Self {
            Self((self.0 & !(1 << 18)) | (u32::from(val) << 18))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter243(self, val: bool) -> Self {
            Self((self.0 & !(1 << 19)) | (u32::from(val) << 19))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter244(self, val: bool) -> Self {
            Self((self.0 & !(1 << 20)) | (u32::from(val) << 20))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter245(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter246(self, val: bool) -> Self {
            Self((self.0 & !(1 << 22)) | (u32::from(val) << 22))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter247(self, val: bool) -> Self {
            Self((self.0 & !(1 << 23)) | (u32::from(val) << 23))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter248(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter249(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter250(self, val: bool) -> Self {
            Self((self.0 & !(1 << 26)) | (u32::from(val) << 26))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter251(self, val: bool) -> Self {
            Self((self.0 & !(1 << 27)) | (u32::from(val) << 27))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter252(self, val: bool) -> Self {
            Self((self.0 & !(1 << 28)) | (u32::from(val) << 28))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter253(self, val: bool) -> Self {
            Self((self.0 & !(1 << 29)) | (u32::from(val) << 29))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter254(self, val: bool) -> Self {
            Self((self.0 & !(1 << 30)) | (u32::from(val) << 30))
        }
        #[doc = "If 1, command will be filtered"]
        #[inline(always)]
        pub fn filter255(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdFilter7WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdFilter7WriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdFilter7WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdInfoReadVal(u32);
    impl CmdInfoReadVal {
        #[doc = "Command Opcode"]
        #[inline(always)]
        pub fn opcode(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "Command address mode\n\nA command can have four modes:\n\n- 0: Command does not have an address field\n- 1: CFG.addr_4b_en decides the address size (3B/4B)\n- 2: Address size is always 3B regardless of CFG.addr_4b_en\n- 3: Address size is always 4B regardless of CFG.addr_4b_en"]
        #[inline(always)]
        pub fn addr_mode(&self) -> super::enums::AddrMode {
            super::enums::AddrMode::try_from((self.0 >> 8) & 3).unwrap()
        }
        #[doc = "This field is used in the passthrough logic.\nIf this field is set to 1, the address in the passthrough command\nis replaced to the preconfigured value."]
        #[inline(always)]
        pub fn addr_swap_en(&self) -> bool {
            ((self.0 >> 10) & 1) != 0
        }
        #[doc = "If 1, the command has a MByte field following the\naddress field. This is set to 1 for DualIO, QuadIO commands."]
        #[inline(always)]
        pub fn mbyte_en(&self) -> bool {
            ((self.0 >> 11) & 1) != 0
        }
        #[doc = "The number of dummy cycles -1 for the command"]
        #[inline(always)]
        pub fn dummy_size(&self) -> u32 {
            (self.0 >> 12) & 7
        }
        #[doc = "Set to 1 if the command has a dummy cycle following the address field."]
        #[inline(always)]
        pub fn dummy_en(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
        #[doc = "Payload Enable per SPI lane.\n\nSet to non-zero if the command has payload at the end of the\nprotocol. This field has four bits. Each bit represents the SPI\nline. If a command is a Single IO command and returns data to the\nhost system, the data is returned on the MISO line (IO[1]). In\nthis case, SW sets payload_en to 4'b 0010."]
        #[inline(always)]
        pub fn payload_en(&self) -> u32 {
            (self.0 >> 16) & 0xf
        }
        #[doc = "Set to 1 if the command returns data. If 0, the payload\nsends to the downstream Flash device."]
        #[inline(always)]
        pub fn payload_dir(&self) -> super::enums::PayloadDir {
            super::enums::PayloadDir::try_from((self.0 >> 20) & 1).unwrap()
        }
        #[doc = "Swap the first byte of the write payload.\n\nIf `payload_swap_en` is set, the passthrough logic swaps the first byte of the write payload with DATA_SWAP CSR.\n\n`payload_swap_en` only works with write data and SingleIO mode. `payload_en` must be 4'b 0001 and `paylod_dir` to be PayloadIn."]
        #[inline(always)]
        pub fn payload_swap_en(&self) -> bool {
            ((self.0 >> 21) & 1) != 0
        }
        #[doc = "Add 2-stage pipeline to read payload.\n\nIf `read_pipeline_mode` is not set to `zero_stages`, the read logic adds a 2-stage pipeline to the read data for this command.\nThis read pipeline enables higher throughput for certain read commands in passthrough mode.\n\n`payload_dir` must be set to PayloadOut: `payload_pipeline_en` only works with read data.\nIt may be used with any IO mode, but general host compatibility is likely limited to Quad Read.\nIf this pipeline is used for passthrough, the internal SFDP should report 2 additional dummy cycles compared to the downstream flash.\nSFDP read commands should be processed internally, and `dummy_size` should still reflect the downstream device's dummy cycle count."]
        #[inline(always)]
        pub fn read_pipeline_mode(&self) -> super::enums::ReadPipelineMode {
            super::enums::ReadPipelineMode::try_from((self.0 >> 22) & 3).unwrap()
        }
        #[doc = "Set to 1 to upload the command.\n\nIf upload field in the command info entry is set, the cmdparse\nactivates the upload submodule when the opcode is received.\n`addr_en`, `addr_4B_affected`, and `addr_4b_forced` (TBD) affect\nthe upload functionality. The three address related configs\ndefines the command address field size.\n\nThe logic assumes the following SPI input stream as payload,\nwhich max size is 256B. If the command exceeds the maximum\npayload size 256B, the logic wraps the payload and overwrites."]
        #[inline(always)]
        pub fn upload(&self) -> bool {
            ((self.0 >> 24) & 1) != 0
        }
        #[doc = "Set to 1 to set the BUSY bit in the FLASH_STATUS when the\ncommand is received.  This bit is active only when `upload` bit is\nset."]
        #[inline(always)]
        pub fn busy(&self) -> bool {
            ((self.0 >> 25) & 1) != 0
        }
        #[doc = "Set to 1 if the config in the register is valid"]
        #[inline(always)]
        pub fn valid(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdInfoWriteVal {
            CmdInfoWriteVal(self.0)
        }
    }
    impl From<u32> for CmdInfoReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdInfoReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdInfoReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdInfoWriteVal(u32);
    impl CmdInfoWriteVal {
        #[doc = "Command Opcode"]
        #[inline(always)]
        pub fn opcode(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "Command address mode\n\nA command can have four modes:\n\n- 0: Command does not have an address field\n- 1: CFG.addr_4b_en decides the address size (3B/4B)\n- 2: Address size is always 3B regardless of CFG.addr_4b_en\n- 3: Address size is always 4B regardless of CFG.addr_4b_en"]
        #[inline(always)]
        pub fn addr_mode(
            self,
            f: impl FnOnce(super::enums::selector::AddrModeSelector) -> super::enums::AddrMode,
        ) -> Self {
            Self(
                (self.0 & !(3 << 8))
                    | (u32::from(f(super::enums::selector::AddrModeSelector())) << 8),
            )
        }
        #[doc = "This field is used in the passthrough logic.\nIf this field is set to 1, the address in the passthrough command\nis replaced to the preconfigured value."]
        #[inline(always)]
        pub fn addr_swap_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 10)) | (u32::from(val) << 10))
        }
        #[doc = "If 1, the command has a MByte field following the\naddress field. This is set to 1 for DualIO, QuadIO commands."]
        #[inline(always)]
        pub fn mbyte_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 11)) | (u32::from(val) << 11))
        }
        #[doc = "The number of dummy cycles -1 for the command"]
        #[inline(always)]
        pub fn dummy_size(self, val: u32) -> Self {
            Self((self.0 & !(7 << 12)) | ((val & 7) << 12))
        }
        #[doc = "Set to 1 if the command has a dummy cycle following the address field."]
        #[inline(always)]
        pub fn dummy_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 15)) | (u32::from(val) << 15))
        }
        #[doc = "Payload Enable per SPI lane.\n\nSet to non-zero if the command has payload at the end of the\nprotocol. This field has four bits. Each bit represents the SPI\nline. If a command is a Single IO command and returns data to the\nhost system, the data is returned on the MISO line (IO[1]). In\nthis case, SW sets payload_en to 4'b 0010."]
        #[inline(always)]
        pub fn payload_en(self, val: u32) -> Self {
            Self((self.0 & !(0xf << 16)) | ((val & 0xf) << 16))
        }
        #[doc = "Set to 1 if the command returns data. If 0, the payload\nsends to the downstream Flash device."]
        #[inline(always)]
        pub fn payload_dir(
            self,
            f: impl FnOnce(super::enums::selector::PayloadDirSelector) -> super::enums::PayloadDir,
        ) -> Self {
            Self(
                (self.0 & !(1 << 20))
                    | (u32::from(f(super::enums::selector::PayloadDirSelector())) << 20),
            )
        }
        #[doc = "Swap the first byte of the write payload.\n\nIf `payload_swap_en` is set, the passthrough logic swaps the first byte of the write payload with DATA_SWAP CSR.\n\n`payload_swap_en` only works with write data and SingleIO mode. `payload_en` must be 4'b 0001 and `paylod_dir` to be PayloadIn."]
        #[inline(always)]
        pub fn payload_swap_en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 21)) | (u32::from(val) << 21))
        }
        #[doc = "Add 2-stage pipeline to read payload.\n\nIf `read_pipeline_mode` is not set to `zero_stages`, the read logic adds a 2-stage pipeline to the read data for this command.\nThis read pipeline enables higher throughput for certain read commands in passthrough mode.\n\n`payload_dir` must be set to PayloadOut: `payload_pipeline_en` only works with read data.\nIt may be used with any IO mode, but general host compatibility is likely limited to Quad Read.\nIf this pipeline is used for passthrough, the internal SFDP should report 2 additional dummy cycles compared to the downstream flash.\nSFDP read commands should be processed internally, and `dummy_size` should still reflect the downstream device's dummy cycle count."]
        #[inline(always)]
        pub fn read_pipeline_mode(
            self,
            f: impl FnOnce(
                super::enums::selector::ReadPipelineModeSelector,
            ) -> super::enums::ReadPipelineMode,
        ) -> Self {
            Self(
                (self.0 & !(3 << 22))
                    | (u32::from(f(super::enums::selector::ReadPipelineModeSelector())) << 22),
            )
        }
        #[doc = "Set to 1 to upload the command.\n\nIf upload field in the command info entry is set, the cmdparse\nactivates the upload submodule when the opcode is received.\n`addr_en`, `addr_4B_affected`, and `addr_4b_forced` (TBD) affect\nthe upload functionality. The three address related configs\ndefines the command address field size.\n\nThe logic assumes the following SPI input stream as payload,\nwhich max size is 256B. If the command exceeds the maximum\npayload size 256B, the logic wraps the payload and overwrites."]
        #[inline(always)]
        pub fn upload(self, val: bool) -> Self {
            Self((self.0 & !(1 << 24)) | (u32::from(val) << 24))
        }
        #[doc = "Set to 1 to set the BUSY bit in the FLASH_STATUS when the\ncommand is received.  This bit is active only when `upload` bit is\nset."]
        #[inline(always)]
        pub fn busy(self, val: bool) -> Self {
            Self((self.0 & !(1 << 25)) | (u32::from(val) << 25))
        }
        #[doc = "Set to 1 if the config in the register is valid"]
        #[inline(always)]
        pub fn valid(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdInfoWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdInfoWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdInfoWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdInfoEn4bReadVal(u32);
    impl CmdInfoEn4bReadVal {
        #[doc = "EN4B opcode"]
        #[inline(always)]
        pub fn opcode(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "If 1, Opcode affects"]
        #[inline(always)]
        pub fn valid(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdInfoEn4bWriteVal {
            CmdInfoEn4bWriteVal(self.0)
        }
    }
    impl From<u32> for CmdInfoEn4bReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdInfoEn4bReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdInfoEn4bReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdInfoEn4bWriteVal(u32);
    impl CmdInfoEn4bWriteVal {
        #[doc = "EN4B opcode"]
        #[inline(always)]
        pub fn opcode(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "If 1, Opcode affects"]
        #[inline(always)]
        pub fn valid(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdInfoEn4bWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdInfoEn4bWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdInfoEn4bWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdInfoEx4bReadVal(u32);
    impl CmdInfoEx4bReadVal {
        #[doc = "EX4B opcode"]
        #[inline(always)]
        pub fn opcode(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "If 1, Opcode affects"]
        #[inline(always)]
        pub fn valid(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdInfoEx4bWriteVal {
            CmdInfoEx4bWriteVal(self.0)
        }
    }
    impl From<u32> for CmdInfoEx4bReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdInfoEx4bReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdInfoEx4bReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdInfoEx4bWriteVal(u32);
    impl CmdInfoEx4bWriteVal {
        #[doc = "EX4B opcode"]
        #[inline(always)]
        pub fn opcode(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "If 1, Opcode affects"]
        #[inline(always)]
        pub fn valid(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdInfoEx4bWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdInfoEx4bWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdInfoEx4bWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdInfoWrdiReadVal(u32);
    impl CmdInfoWrdiReadVal {
        #[doc = "WRDI opcode"]
        #[inline(always)]
        pub fn opcode(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "If 1, opcode affects"]
        #[inline(always)]
        pub fn valid(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdInfoWrdiWriteVal {
            CmdInfoWrdiWriteVal(self.0)
        }
    }
    impl From<u32> for CmdInfoWrdiReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdInfoWrdiReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdInfoWrdiReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdInfoWrdiWriteVal(u32);
    impl CmdInfoWrdiWriteVal {
        #[doc = "WRDI opcode"]
        #[inline(always)]
        pub fn opcode(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "If 1, opcode affects"]
        #[inline(always)]
        pub fn valid(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdInfoWrdiWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdInfoWrdiWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdInfoWrdiWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdInfoWrenReadVal(u32);
    impl CmdInfoWrenReadVal {
        #[doc = "WREN opcode"]
        #[inline(always)]
        pub fn opcode(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "If 1, opcode affects"]
        #[inline(always)]
        pub fn valid(&self) -> bool {
            ((self.0 >> 31) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> CmdInfoWrenWriteVal {
            CmdInfoWrenWriteVal(self.0)
        }
    }
    impl From<u32> for CmdInfoWrenReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdInfoWrenReadVal> for u32 {
        #[inline(always)]
        fn from(val: CmdInfoWrenReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct CmdInfoWrenWriteVal(u32);
    impl CmdInfoWrenWriteVal {
        #[doc = "WREN opcode"]
        #[inline(always)]
        pub fn opcode(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "If 1, opcode affects"]
        #[inline(always)]
        pub fn valid(self, val: bool) -> Self {
            Self((self.0 & !(1 << 31)) | (u32::from(val) << 31))
        }
    }
    impl From<u32> for CmdInfoWrenWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<CmdInfoWrenWriteVal> for u32 {
        #[inline(always)]
        fn from(val: CmdInfoWrenWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ControlReadVal(u32);
    impl ControlReadVal {
        #[doc = "Set to clear the flash status FIFO.\n\nWhen set to 1, resets the flash status FIFO used for synchronizing changes from firmware.\nThe reset should only be used when the upstream SPI host is known to be inactive.\nThis function is intended to allow restoring initial values when the upstream SPI host is reset.\n\nThis CSR automatically resets to 0."]
        #[inline(always)]
        pub fn flash_status_fifo_clr(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Set to clear the read buffer state.\n\nWhen set to 1, resets the flash read buffer state that tracks the host read address.\nThe reset should only be used when the upstream SPI host is known to be inactive.\nThis function is intended to allow restoring initial values when the upstream SPI host is reset.\n\nThis CSR automatically resets to 0."]
        #[inline(always)]
        pub fn flash_read_buffer_clr(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "SPI Device flash operation mode."]
        #[inline(always)]
        pub fn mode(&self) -> super::enums::Mode {
            super::enums::Mode::try_from((self.0 >> 4) & 3).unwrap()
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ControlWriteVal {
            ControlWriteVal(self.0)
        }
    }
    impl From<u32> for ControlReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ControlReadVal> for u32 {
        #[inline(always)]
        fn from(val: ControlReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ControlWriteVal(u32);
    impl ControlWriteVal {
        #[doc = "Set to clear the flash status FIFO.\n\nWhen set to 1, resets the flash status FIFO used for synchronizing changes from firmware.\nThe reset should only be used when the upstream SPI host is known to be inactive.\nThis function is intended to allow restoring initial values when the upstream SPI host is reset.\n\nThis CSR automatically resets to 0."]
        #[inline(always)]
        pub fn flash_status_fifo_clr_set(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Set to clear the read buffer state.\n\nWhen set to 1, resets the flash read buffer state that tracks the host read address.\nThe reset should only be used when the upstream SPI host is known to be inactive.\nThis function is intended to allow restoring initial values when the upstream SPI host is reset.\n\nThis CSR automatically resets to 0."]
        #[inline(always)]
        pub fn flash_read_buffer_clr_set(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "SPI Device flash operation mode."]
        #[inline(always)]
        pub fn mode(
            self,
            f: impl FnOnce(super::enums::selector::ModeSelector) -> super::enums::Mode,
        ) -> Self {
            Self((self.0 & !(3 << 4)) | (u32::from(f(super::enums::selector::ModeSelector())) << 4))
        }
    }
    impl From<u32> for ControlWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ControlWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ControlWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FlashStatusReadVal(u32);
    impl FlashStatusReadVal {
        #[doc = "BUSY signal is cleared when CSb is high. SW should read\nback the register to confirm the value is cleared."]
        #[inline(always)]
        pub fn busy(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "WEL signal is cleared when CSb is high. SW should read\nback the register to confirm the value is cleared.\n\nBit 1 (WEL) is a SW modifiable and HW modifiable field.\nHW updates the WEL field when `WRDI` or `WREN` command is received."]
        #[inline(always)]
        pub fn wel(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Rest of the status register.\n\nFields other than the bit 0 (BUSY) and bit 1 (WEL) fields are\nSW-maintained fields. HW just reads and returns to the host system.\n\n- [ 2]\\: BP0\n- [ 3]\\: BP1\n- [ 4]\\: BP2\n- [ 5]\\: TB\n- [ 6]\\: SEC\n- [ 7]\\: SRP0\n- [ 8]\\: SRP1\n- [ 9]\\: QE\n- [11]\\: LB1\n- [12]\\: LB2\n- [13]\\: LB3\n- [14]\\: CMP\n- [15]\\: SUS\n- [18]\\: WPS\n- [21]\\: DRV0\n- [22]\\: DRV1\n- [23]\\: HOLD /RST"]
        #[inline(always)]
        pub fn status(&self) -> u32 {
            (self.0 >> 2) & 0x3fffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> FlashStatusWriteVal {
            FlashStatusWriteVal(self.0)
        }
    }
    impl From<u32> for FlashStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FlashStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: FlashStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct FlashStatusWriteVal(u32);
    impl FlashStatusWriteVal {
        #[doc = "BUSY signal is cleared when CSb is high. SW should read\nback the register to confirm the value is cleared."]
        #[inline(always)]
        pub fn busy_clear(self) -> Self {
            Self(self.0 & !(1 << 0))
        }
        #[doc = "WEL signal is cleared when CSb is high. SW should read\nback the register to confirm the value is cleared.\n\nBit 1 (WEL) is a SW modifiable and HW modifiable field.\nHW updates the WEL field when `WRDI` or `WREN` command is received."]
        #[inline(always)]
        pub fn wel_clear(self) -> Self {
            Self(self.0 & !(1 << 1))
        }
        #[doc = "Rest of the status register.\n\nFields other than the bit 0 (BUSY) and bit 1 (WEL) fields are\nSW-maintained fields. HW just reads and returns to the host system.\n\n- [ 2]\\: BP0\n- [ 3]\\: BP1\n- [ 4]\\: BP2\n- [ 5]\\: TB\n- [ 6]\\: SEC\n- [ 7]\\: SRP0\n- [ 8]\\: SRP1\n- [ 9]\\: QE\n- [11]\\: LB1\n- [12]\\: LB2\n- [13]\\: LB3\n- [14]\\: CMP\n- [15]\\: SUS\n- [18]\\: WPS\n- [21]\\: DRV0\n- [22]\\: DRV1\n- [23]\\: HOLD /RST"]
        #[inline(always)]
        pub fn status(self, val: u32) -> Self {
            Self((self.0 & !(0x3fffff << 2)) | ((val & 0x3fffff) << 2))
        }
    }
    impl From<u32> for FlashStatusWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<FlashStatusWriteVal> for u32 {
        #[inline(always)]
        fn from(val: FlashStatusWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct InterceptEnReadVal(u32);
    impl InterceptEnReadVal {
        #[doc = "If set, Read Status is processed internally."]
        #[inline(always)]
        pub fn status(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If set, Read JEDEC ID is processed internally."]
        #[inline(always)]
        pub fn jedec(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If set, Read SFDP is processed internally."]
        #[inline(always)]
        pub fn sfdp(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If set, Read Command to Mailbox region is processed internally."]
        #[inline(always)]
        pub fn mbx(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> InterceptEnWriteVal {
            InterceptEnWriteVal(self.0)
        }
    }
    impl From<u32> for InterceptEnReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<InterceptEnReadVal> for u32 {
        #[inline(always)]
        fn from(val: InterceptEnReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct InterceptEnWriteVal(u32);
    impl InterceptEnWriteVal {
        #[doc = "If set, Read Status is processed internally."]
        #[inline(always)]
        pub fn status(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "If set, Read JEDEC ID is processed internally."]
        #[inline(always)]
        pub fn jedec(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If set, Read SFDP is processed internally."]
        #[inline(always)]
        pub fn sfdp(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If set, Read Command to Mailbox region is processed internally."]
        #[inline(always)]
        pub fn mbx(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
    }
    impl From<u32> for InterceptEnWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<InterceptEnWriteVal> for u32 {
        #[inline(always)]
        fn from(val: InterceptEnWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableReadVal(u32);
    impl IntrEnableReadVal {
        #[doc = "Enable interrupt when !!INTR_STATE.upload_cmdfifo_not_empty is set."]
        #[inline(always)]
        pub fn upload_cmdfifo_not_empty(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.upload_payload_not_empty is set."]
        #[inline(always)]
        pub fn upload_payload_not_empty(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.upload_payload_overflow is set."]
        #[inline(always)]
        pub fn upload_payload_overflow(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.readbuf_watermark is set."]
        #[inline(always)]
        pub fn readbuf_watermark(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.readbuf_flip is set."]
        #[inline(always)]
        pub fn readbuf_flip(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tpm_header_not_empty is set."]
        #[inline(always)]
        pub fn tpm_header_not_empty(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tpm_rdfifo_cmd_end is set."]
        #[inline(always)]
        pub fn tpm_rdfifo_cmd_end(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tpm_rdfifo_drop is set."]
        #[inline(always)]
        pub fn tpm_rdfifo_drop(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IntrEnableWriteVal {
            IntrEnableWriteVal(self.0)
        }
    }
    impl From<u32> for IntrEnableReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrEnableReadVal> for u32 {
        #[inline(always)]
        fn from(val: IntrEnableReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrEnableWriteVal(u32);
    impl IntrEnableWriteVal {
        #[doc = "Enable interrupt when !!INTR_STATE.upload_cmdfifo_not_empty is set."]
        #[inline(always)]
        pub fn upload_cmdfifo_not_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.upload_payload_not_empty is set."]
        #[inline(always)]
        pub fn upload_payload_not_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.upload_payload_overflow is set."]
        #[inline(always)]
        pub fn upload_payload_overflow(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.readbuf_watermark is set."]
        #[inline(always)]
        pub fn readbuf_watermark(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.readbuf_flip is set."]
        #[inline(always)]
        pub fn readbuf_flip(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tpm_header_not_empty is set."]
        #[inline(always)]
        pub fn tpm_header_not_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tpm_rdfifo_cmd_end is set."]
        #[inline(always)]
        pub fn tpm_rdfifo_cmd_end(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Enable interrupt when !!INTR_STATE.tpm_rdfifo_drop is set."]
        #[inline(always)]
        pub fn tpm_rdfifo_drop(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
    }
    impl From<u32> for IntrEnableWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrEnableWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntrEnableWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrStateReadVal(u32);
    impl IntrStateReadVal {
        #[doc = "Upload Command FIFO is not empty"]
        #[inline(always)]
        pub fn upload_cmdfifo_not_empty(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Upload payload is not empty.\n\nThe event occurs after SPI transaction completed"]
        #[inline(always)]
        pub fn upload_payload_not_empty(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "Upload payload overflow event.\n\nWhen a SPI Host system issues a command with payload more than 256B,\nthis event is reported. When it happens, SW should read the last\nwritten payload index CSR to figure out the starting address of the\nlast 256B."]
        #[inline(always)]
        pub fn upload_payload_overflow(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "Read Buffer Threshold event.\n\nThe host system accesses greater than or equal to the threshold of a\nbuffer."]
        #[inline(always)]
        pub fn readbuf_watermark(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "Read buffer flipped event.\n\nThe host system accesses other side of buffer."]
        #[inline(always)]
        pub fn readbuf_flip(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = "TPM Header(Command/Address) buffer available"]
        #[inline(always)]
        pub fn tpm_header_not_empty(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "TPM RdFIFO command ended.\n\nThe TPM Read command targeting the RdFIFO ended.\nCheck TPM_STATUS.rdfifo_aborted to see if the transaction completed."]
        #[inline(always)]
        pub fn tpm_rdfifo_cmd_end(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
        #[doc = "TPM RdFIFO data dropped.\n\nData was dropped from the RdFIFO.\nData was written while a read command was not active, and it was not accepted.\nThis can occur when the host aborts a read command."]
        #[inline(always)]
        pub fn tpm_rdfifo_drop(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> IntrStateWriteVal {
            IntrStateWriteVal(self.0)
        }
    }
    impl From<u32> for IntrStateReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrStateReadVal> for u32 {
        #[inline(always)]
        fn from(val: IntrStateReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrStateWriteVal(u32);
    impl IntrStateWriteVal {
        #[doc = "Upload Command FIFO is not empty"]
        #[inline(always)]
        pub fn upload_cmdfifo_not_empty_clear(self) -> Self {
            Self(self.0 | (1 << 0))
        }
        #[doc = "Upload payload is not empty.\n\nThe event occurs after SPI transaction completed"]
        #[inline(always)]
        pub fn upload_payload_not_empty_clear(self) -> Self {
            Self(self.0 | (1 << 1))
        }
        #[doc = "Upload payload overflow event.\n\nWhen a SPI Host system issues a command with payload more than 256B,\nthis event is reported. When it happens, SW should read the last\nwritten payload index CSR to figure out the starting address of the\nlast 256B."]
        #[inline(always)]
        pub fn upload_payload_overflow_clear(self) -> Self {
            Self(self.0 | (1 << 2))
        }
        #[doc = "Read Buffer Threshold event.\n\nThe host system accesses greater than or equal to the threshold of a\nbuffer."]
        #[inline(always)]
        pub fn readbuf_watermark_clear(self) -> Self {
            Self(self.0 | (1 << 3))
        }
        #[doc = "Read buffer flipped event.\n\nThe host system accesses other side of buffer."]
        #[inline(always)]
        pub fn readbuf_flip_clear(self) -> Self {
            Self(self.0 | (1 << 4))
        }
        #[doc = "TPM RdFIFO command ended.\n\nThe TPM Read command targeting the RdFIFO ended.\nCheck TPM_STATUS.rdfifo_aborted to see if the transaction completed."]
        #[inline(always)]
        pub fn tpm_rdfifo_cmd_end_clear(self) -> Self {
            Self(self.0 | (1 << 6))
        }
        #[doc = "TPM RdFIFO data dropped.\n\nData was dropped from the RdFIFO.\nData was written while a read command was not active, and it was not accepted.\nThis can occur when the host aborts a read command."]
        #[inline(always)]
        pub fn tpm_rdfifo_drop_clear(self) -> Self {
            Self(self.0 | (1 << 7))
        }
    }
    impl From<u32> for IntrStateWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrStateWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntrStateWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct IntrTestWriteVal(u32);
    impl IntrTestWriteVal {
        #[doc = "Write 1 to force !!INTR_STATE.upload_cmdfifo_not_empty to 1."]
        #[inline(always)]
        pub fn upload_cmdfifo_not_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Write 1 to force !!INTR_STATE.upload_payload_not_empty to 1."]
        #[inline(always)]
        pub fn upload_payload_not_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "Write 1 to force !!INTR_STATE.upload_payload_overflow to 1."]
        #[inline(always)]
        pub fn upload_payload_overflow(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "Write 1 to force !!INTR_STATE.readbuf_watermark to 1."]
        #[inline(always)]
        pub fn readbuf_watermark(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "Write 1 to force !!INTR_STATE.readbuf_flip to 1."]
        #[inline(always)]
        pub fn readbuf_flip(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
        #[doc = "Write 1 to force !!INTR_STATE.tpm_header_not_empty to 1."]
        #[inline(always)]
        pub fn tpm_header_not_empty(self, val: bool) -> Self {
            Self((self.0 & !(1 << 5)) | (u32::from(val) << 5))
        }
        #[doc = "Write 1 to force !!INTR_STATE.tpm_rdfifo_cmd_end to 1."]
        #[inline(always)]
        pub fn tpm_rdfifo_cmd_end(self, val: bool) -> Self {
            Self((self.0 & !(1 << 6)) | (u32::from(val) << 6))
        }
        #[doc = "Write 1 to force !!INTR_STATE.tpm_rdfifo_drop to 1."]
        #[inline(always)]
        pub fn tpm_rdfifo_drop(self, val: bool) -> Self {
            Self((self.0 & !(1 << 7)) | (u32::from(val) << 7))
        }
    }
    impl From<u32> for IntrTestWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<IntrTestWriteVal> for u32 {
        #[inline(always)]
        fn from(val: IntrTestWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct JedecCcReadVal(u32);
    impl JedecCcReadVal {
        #[doc = "Continuation Code byte"]
        #[inline(always)]
        pub fn cc(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "The number that Continuation Code repeats"]
        #[inline(always)]
        pub fn num_cc(&self) -> u32 {
            (self.0 >> 8) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> JedecCcWriteVal {
            JedecCcWriteVal(self.0)
        }
    }
    impl From<u32> for JedecCcReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<JedecCcReadVal> for u32 {
        #[inline(always)]
        fn from(val: JedecCcReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct JedecCcWriteVal(u32);
    impl JedecCcWriteVal {
        #[doc = "Continuation Code byte"]
        #[inline(always)]
        pub fn cc(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "The number that Continuation Code repeats"]
        #[inline(always)]
        pub fn num_cc(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 8)) | ((val & 0xff) << 8))
        }
    }
    impl From<u32> for JedecCcWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<JedecCcWriteVal> for u32 {
        #[inline(always)]
        fn from(val: JedecCcWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct JedecIdReadVal(u32);
    impl JedecIdReadVal {
        #[doc = "Device ID"]
        #[inline(always)]
        pub fn id(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = "Manufacturer ID"]
        #[inline(always)]
        pub fn mf(&self) -> u32 {
            (self.0 >> 16) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> JedecIdWriteVal {
            JedecIdWriteVal(self.0)
        }
    }
    impl From<u32> for JedecIdReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<JedecIdReadVal> for u32 {
        #[inline(always)]
        fn from(val: JedecIdReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct JedecIdWriteVal(u32);
    impl JedecIdWriteVal {
        #[doc = "Device ID"]
        #[inline(always)]
        pub fn id(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
        #[doc = "Manufacturer ID"]
        #[inline(always)]
        pub fn mf(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 16)) | ((val & 0xff) << 16))
        }
    }
    impl From<u32> for JedecIdWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<JedecIdWriteVal> for u32 {
        #[inline(always)]
        fn from(val: JedecIdWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ReadThresholdReadVal(u32);
    impl ReadThresholdReadVal {
        #[doc = "If 0, disable the watermark. If non-zero, when the host\naccess above or equal to the threshold, it reports an interrupt.\nThe value is byte-granularity not SRAM index."]
        #[inline(always)]
        pub fn threshold(&self) -> u32 {
            (self.0 >> 0) & 0x3ff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> ReadThresholdWriteVal {
            ReadThresholdWriteVal(self.0)
        }
    }
    impl From<u32> for ReadThresholdReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ReadThresholdReadVal> for u32 {
        #[inline(always)]
        fn from(val: ReadThresholdReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct ReadThresholdWriteVal(u32);
    impl ReadThresholdWriteVal {
        #[doc = "If 0, disable the watermark. If non-zero, when the host\naccess above or equal to the threshold, it reports an interrupt.\nThe value is byte-granularity not SRAM index."]
        #[inline(always)]
        pub fn threshold(self, val: u32) -> Self {
            Self((self.0 & !(0x3ff << 0)) | ((val & 0x3ff) << 0))
        }
    }
    impl From<u32> for ReadThresholdWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<ReadThresholdWriteVal> for u32 {
        #[inline(always)]
        fn from(val: ReadThresholdWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct StatusReadVal(u32);
    impl StatusReadVal {
        #[doc = "Direct input of CSb signal"]
        #[inline(always)]
        pub fn csb(&self) -> bool {
            ((self.0 >> 5) & 1) != 0
        }
        #[doc = "Direct input of TPM CSb"]
        #[inline(always)]
        pub fn tpm_csb(&self) -> bool {
            ((self.0 >> 6) & 1) != 0
        }
    }
    impl From<u32> for StatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<StatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: StatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmAccess0ReadVal(u32);
    impl TpmAccess0ReadVal {
        #[doc = "TPM_ACCESS"]
        #[inline(always)]
        pub fn access0(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "TPM_ACCESS"]
        #[inline(always)]
        pub fn access1(&self) -> u32 {
            (self.0 >> 8) & 0xff
        }
        #[doc = "TPM_ACCESS"]
        #[inline(always)]
        pub fn access2(&self) -> u32 {
            (self.0 >> 16) & 0xff
        }
        #[doc = "TPM_ACCESS"]
        #[inline(always)]
        pub fn access3(&self) -> u32 {
            (self.0 >> 24) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TpmAccess0WriteVal {
            TpmAccess0WriteVal(self.0)
        }
    }
    impl From<u32> for TpmAccess0ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmAccess0ReadVal> for u32 {
        #[inline(always)]
        fn from(val: TpmAccess0ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmAccess0WriteVal(u32);
    impl TpmAccess0WriteVal {
        #[doc = "TPM_ACCESS"]
        #[inline(always)]
        pub fn access0(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
        #[doc = "TPM_ACCESS"]
        #[inline(always)]
        pub fn access1(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 8)) | ((val & 0xff) << 8))
        }
        #[doc = "TPM_ACCESS"]
        #[inline(always)]
        pub fn access2(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 16)) | ((val & 0xff) << 16))
        }
        #[doc = "TPM_ACCESS"]
        #[inline(always)]
        pub fn access3(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 24)) | ((val & 0xff) << 24))
        }
    }
    impl From<u32> for TpmAccess0WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmAccess0WriteVal> for u32 {
        #[inline(always)]
        fn from(val: TpmAccess0WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmAccess1ReadVal(u32);
    impl TpmAccess1ReadVal {
        #[doc = "TPM_ACCESS"]
        #[inline(always)]
        pub fn access32(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TpmAccess1WriteVal {
            TpmAccess1WriteVal(self.0)
        }
    }
    impl From<u32> for TpmAccess1ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmAccess1ReadVal> for u32 {
        #[inline(always)]
        fn from(val: TpmAccess1ReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmAccess1WriteVal(u32);
    impl TpmAccess1WriteVal {
        #[doc = "TPM_ACCESS"]
        #[inline(always)]
        pub fn access32(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
    }
    impl From<u32> for TpmAccess1WriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmAccess1WriteVal> for u32 {
        #[inline(always)]
        fn from(val: TpmAccess1WriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmCapReadVal(u32);
    impl TpmCapReadVal {
        #[doc = "Revision of the TPM submodule"]
        #[inline(always)]
        pub fn rev(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "If 1, the TPM submodule supports 5 Locality.\nIf 0, only one Locality is provided"]
        #[inline(always)]
        pub fn locality(&self) -> bool {
            ((self.0 >> 8) & 1) != 0
        }
        #[doc = "The maximum write size in bytes the TPM submodule supports.\nThe value is the exponent of the 2.\n\n- 3'b 010: Support up to 4B\n- 3'b 011: Support up to 8B\n- 3'b 100: Support up to 16B\n- 3'b 101: Support up to 32B\n- 3'b 110: Support up to 64B\n\nAll other values are reserved.\n\nIt is not recommended for SW to advertise TPM supporting more than `max_wr_size` to the South Bridge."]
        #[inline(always)]
        pub fn max_wr_size(&self) -> u32 {
            (self.0 >> 16) & 7
        }
        #[doc = "The maximum read size in bytes the TPM submodule supports.\nThe value is the exponent of the 2.\n\n- 3'b 010: Support up to 4B\n- 3'b 011: Support up to 8B\n- 3'b 100: Support up to 16B\n- 3'b 101: Support up to 32B\n- 3'b 110: Support up to 64B\n\nAll other values are reserved.\n\nIt is not recommended for SW to advertise TPM supporting more than `max_rd_size` to the South Bridge."]
        #[inline(always)]
        pub fn max_rd_size(&self) -> u32 {
            (self.0 >> 20) & 7
        }
    }
    impl From<u32> for TpmCapReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmCapReadVal> for u32 {
        #[inline(always)]
        fn from(val: TpmCapReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmCfgReadVal(u32);
    impl TpmCfgReadVal {
        #[doc = "If 1, TPM submodule accepts the transactions over SPI"]
        #[inline(always)]
        pub fn en(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "Configure the TPM mode. 1 for CRB, 0 for FIFO.\n\nIf the SW set this field to 1, the HW logic always pushes the\ncommand/addr and write data to buffers. The logic does not compare\nthe incoming address to the list of managed-by-HW register\naddresses.\n\nThe invalid locality check still runs based on the invalid_locality\nconfiguration."]
        #[inline(always)]
        pub fn tpm_mode(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If 0, TPM submodule directly returns the return-by-HW registers for the read requests.\n\nIf 1, TPM submodule uploads the TPM command regardless of the address, and the SW may return the value through the read FIFO."]
        #[inline(always)]
        pub fn hw_reg_dis(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = "If 1, the logic does not compare the upper 8 bit of the\nreceived address with the TpmAddr constant, D4h.\n\nIf this field is 0, the HW uploads the command, address, and write\npayload to the buffers in case of address that is not 0xD4_XXXX."]
        #[inline(always)]
        pub fn tpm_reg_chk_dis(&self) -> bool {
            ((self.0 >> 3) & 1) != 0
        }
        #[doc = "If 1, TPM submodule returns the invalid data (0xFF) for the\nout of the max Locality request.\nIf it is a write request, HW still uploads the command and address.\nSW needs to process the incoming invalid command.\n\nIf 0, TPM submodule uploads the TPM command and address. The SW may\nwrite 0xFF to the read FIFO.\n\nNote: The TPM submodule uploads the TPM commands that do not fall\ninto the FIFO registers (0xD4_XXXX) regardless of\n`invalid_locality` bit."]
        #[inline(always)]
        pub fn invalid_locality(&self) -> bool {
            ((self.0 >> 4) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TpmCfgWriteVal {
            TpmCfgWriteVal(self.0)
        }
    }
    impl From<u32> for TpmCfgReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmCfgReadVal> for u32 {
        #[inline(always)]
        fn from(val: TpmCfgReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmCfgWriteVal(u32);
    impl TpmCfgWriteVal {
        #[doc = "If 1, TPM submodule accepts the transactions over SPI"]
        #[inline(always)]
        pub fn en(self, val: bool) -> Self {
            Self((self.0 & !(1 << 0)) | (u32::from(val) << 0))
        }
        #[doc = "Configure the TPM mode. 1 for CRB, 0 for FIFO.\n\nIf the SW set this field to 1, the HW logic always pushes the\ncommand/addr and write data to buffers. The logic does not compare\nthe incoming address to the list of managed-by-HW register\naddresses.\n\nThe invalid locality check still runs based on the invalid_locality\nconfiguration."]
        #[inline(always)]
        pub fn tpm_mode(self, val: bool) -> Self {
            Self((self.0 & !(1 << 1)) | (u32::from(val) << 1))
        }
        #[doc = "If 0, TPM submodule directly returns the return-by-HW registers for the read requests.\n\nIf 1, TPM submodule uploads the TPM command regardless of the address, and the SW may return the value through the read FIFO."]
        #[inline(always)]
        pub fn hw_reg_dis(self, val: bool) -> Self {
            Self((self.0 & !(1 << 2)) | (u32::from(val) << 2))
        }
        #[doc = "If 1, the logic does not compare the upper 8 bit of the\nreceived address with the TpmAddr constant, D4h.\n\nIf this field is 0, the HW uploads the command, address, and write\npayload to the buffers in case of address that is not 0xD4_XXXX."]
        #[inline(always)]
        pub fn tpm_reg_chk_dis(self, val: bool) -> Self {
            Self((self.0 & !(1 << 3)) | (u32::from(val) << 3))
        }
        #[doc = "If 1, TPM submodule returns the invalid data (0xFF) for the\nout of the max Locality request.\nIf it is a write request, HW still uploads the command and address.\nSW needs to process the incoming invalid command.\n\nIf 0, TPM submodule uploads the TPM command and address. The SW may\nwrite 0xFF to the read FIFO.\n\nNote: The TPM submodule uploads the TPM commands that do not fall\ninto the FIFO registers (0xD4_XXXX) regardless of\n`invalid_locality` bit."]
        #[inline(always)]
        pub fn invalid_locality(self, val: bool) -> Self {
            Self((self.0 & !(1 << 4)) | (u32::from(val) << 4))
        }
    }
    impl From<u32> for TpmCfgWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmCfgWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TpmCfgWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmCmdAddrReadVal(u32);
    impl TpmCmdAddrReadVal {
        #[doc = "received address"]
        #[inline(always)]
        pub fn addr(&self) -> u32 {
            (self.0 >> 0) & 0xffffff
        }
        #[doc = "received command"]
        #[inline(always)]
        pub fn cmd(&self) -> u32 {
            (self.0 >> 24) & 0xff
        }
    }
    impl From<u32> for TpmCmdAddrReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmCmdAddrReadVal> for u32 {
        #[inline(always)]
        fn from(val: TpmCmdAddrReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmDidVidReadVal(u32);
    impl TpmDidVidReadVal {
        #[doc = "TPM_VID"]
        #[inline(always)]
        pub fn vid(&self) -> u32 {
            (self.0 >> 0) & 0xffff
        }
        #[doc = "TPM_DID"]
        #[inline(always)]
        pub fn did(&self) -> u32 {
            (self.0 >> 16) & 0xffff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TpmDidVidWriteVal {
            TpmDidVidWriteVal(self.0)
        }
    }
    impl From<u32> for TpmDidVidReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmDidVidReadVal> for u32 {
        #[inline(always)]
        fn from(val: TpmDidVidReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmDidVidWriteVal(u32);
    impl TpmDidVidWriteVal {
        #[doc = "TPM_VID"]
        #[inline(always)]
        pub fn vid(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 0)) | ((val & 0xffff) << 0))
        }
        #[doc = "TPM_DID"]
        #[inline(always)]
        pub fn did(self, val: u32) -> Self {
            Self((self.0 & !(0xffff << 16)) | ((val & 0xffff) << 16))
        }
    }
    impl From<u32> for TpmDidVidWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmDidVidWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TpmDidVidWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmIntVectorReadVal(u32);
    impl TpmIntVectorReadVal {
        #[doc = "TPM_INT_VECTOR"]
        #[inline(always)]
        pub fn int_vector(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TpmIntVectorWriteVal {
            TpmIntVectorWriteVal(self.0)
        }
    }
    impl From<u32> for TpmIntVectorReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmIntVectorReadVal> for u32 {
        #[inline(always)]
        fn from(val: TpmIntVectorReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmIntVectorWriteVal(u32);
    impl TpmIntVectorWriteVal {
        #[doc = "TPM_INT_VECTOR"]
        #[inline(always)]
        pub fn int_vector(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
    }
    impl From<u32> for TpmIntVectorWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmIntVectorWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TpmIntVectorWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmRidReadVal(u32);
    impl TpmRidReadVal {
        #[doc = "TPM_RID"]
        #[inline(always)]
        pub fn rid(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TpmRidWriteVal {
            TpmRidWriteVal(self.0)
        }
    }
    impl From<u32> for TpmRidReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmRidReadVal> for u32 {
        #[inline(always)]
        fn from(val: TpmRidReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmRidWriteVal(u32);
    impl TpmRidWriteVal {
        #[doc = "TPM_RID"]
        #[inline(always)]
        pub fn rid(self, val: u32) -> Self {
            Self((self.0 & !(0xff << 0)) | ((val & 0xff) << 0))
        }
    }
    impl From<u32> for TpmRidWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmRidWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TpmRidWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmStatusReadVal(u32);
    impl TpmStatusReadVal {
        #[doc = "If 1, the TPM_CMD_ADDR has a valid data. This status is reported via the interrupt also."]
        #[inline(always)]
        pub fn cmdaddr_notempty(&self) -> bool {
            ((self.0 >> 0) & 1) != 0
        }
        #[doc = "If 1, the Write FIFO is reserved for software processing.\n\nThis bit becomes 1 when a complete write command is received.\nWhile it remains 1, subsequent write commands will block at the wait state until it is cleared.\nWrite 0 to release the Write FIFO back to the TPM module."]
        #[inline(always)]
        pub fn wrfifo_pending(&self) -> bool {
            ((self.0 >> 1) & 1) != 0
        }
        #[doc = "If 1, the last Read FIFO command was aborted.\n\nThis bit becomes 1 when a Read FIFO command became active, but the transaction did not complete.\nAn aborted transaction occurs when the host de-asserts CSB without clocking all the requested data.\nThis bit remains 1 until reset, or it will clear automatically after the next valid command is read from TPM_CMD_ADDR."]
        #[inline(always)]
        pub fn rdfifo_aborted(&self) -> bool {
            ((self.0 >> 2) & 1) != 0
        }
        #[doc = r" Construct a WriteVal that can be used to modify the contents of this register value."]
        #[inline(always)]
        pub fn modify(self) -> TpmStatusWriteVal {
            TpmStatusWriteVal(self.0)
        }
    }
    impl From<u32> for TpmStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: TpmStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct TpmStatusWriteVal(u32);
    impl TpmStatusWriteVal {
        #[doc = "If 1, the Write FIFO is reserved for software processing.\n\nThis bit becomes 1 when a complete write command is received.\nWhile it remains 1, subsequent write commands will block at the wait state until it is cleared.\nWrite 0 to release the Write FIFO back to the TPM module."]
        #[inline(always)]
        pub fn wrfifo_pending_clear(self) -> Self {
            Self(self.0 & !(1 << 1))
        }
    }
    impl From<u32> for TpmStatusWriteVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<TpmStatusWriteVal> for u32 {
        #[inline(always)]
        fn from(val: TpmStatusWriteVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UploadCmdfifoReadVal(u32);
    impl UploadCmdfifoReadVal {
        #[doc = "command opcode"]
        #[inline(always)]
        pub fn data(&self) -> u32 {
            (self.0 >> 0) & 0xff
        }
        #[doc = "State of BUSY bit at command time"]
        #[inline(always)]
        pub fn busy(&self) -> bool {
            ((self.0 >> 13) & 1) != 0
        }
        #[doc = "State of WEL bit at command time"]
        #[inline(always)]
        pub fn wel(&self) -> bool {
            ((self.0 >> 14) & 1) != 0
        }
        #[doc = "1 if address mode at command time is 4 Bytes, else 3 Bytes"]
        #[inline(always)]
        pub fn addr4b_mode(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
    }
    impl From<u32> for UploadCmdfifoReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UploadCmdfifoReadVal> for u32 {
        #[inline(always)]
        fn from(val: UploadCmdfifoReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UploadStatusReadVal(u32);
    impl UploadStatusReadVal {
        #[doc = "Command FIFO Entry"]
        #[inline(always)]
        pub fn cmdfifo_depth(&self) -> u32 {
            (self.0 >> 0) & 0x1f
        }
        #[doc = "Upload Command FIFO Not Empty"]
        #[inline(always)]
        pub fn cmdfifo_notempty(&self) -> bool {
            ((self.0 >> 7) & 1) != 0
        }
        #[doc = "Address FIFO Entry"]
        #[inline(always)]
        pub fn addrfifo_depth(&self) -> u32 {
            (self.0 >> 8) & 0x1f
        }
        #[doc = "Upload Address FIFO Not Empty"]
        #[inline(always)]
        pub fn addrfifo_notempty(&self) -> bool {
            ((self.0 >> 15) & 1) != 0
        }
    }
    impl From<u32> for UploadStatusReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UploadStatusReadVal> for u32 {
        #[inline(always)]
        fn from(val: UploadStatusReadVal) -> u32 {
            val.0
        }
    }
    #[derive(Clone, Copy)]
    pub struct UploadStatus2ReadVal(u32);
    impl UploadStatus2ReadVal {
        #[doc = "Payload buffer depth"]
        #[inline(always)]
        pub fn payload_depth(&self) -> u32 {
            (self.0 >> 0) & 0x1ff
        }
        #[doc = "Payload Start Index"]
        #[inline(always)]
        pub fn payload_start_idx(&self) -> u32 {
            (self.0 >> 16) & 0xff
        }
    }
    impl From<u32> for UploadStatus2ReadVal {
        #[inline(always)]
        fn from(val: u32) -> Self {
            Self(val)
        }
    }
    impl From<UploadStatus2ReadVal> for u32 {
        #[inline(always)]
        fn from(val: UploadStatus2ReadVal) -> u32 {
            val.0
        }
    }
}
pub mod enums {
    #![doc = r" Enumerations used by some register fields."]
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum Mode {
        Disabled = 0,
        Flashmode = 1,
        Passthrough = 2,
        Reserved3 = 3,
    }
    impl Mode {
        #[inline(always)]
        pub fn disabled(&self) -> bool {
            *self == Self::Disabled
        }
        #[inline(always)]
        pub fn flashmode(&self) -> bool {
            *self == Self::Flashmode
        }
        #[inline(always)]
        pub fn passthrough(&self) -> bool {
            *self == Self::Passthrough
        }
    }
    impl TryFrom<u32> for Mode {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<Mode, ()> {
            if val < 4 {
                Ok(unsafe { core::mem::transmute::<u32, Mode>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<Mode> for u32 {
        fn from(val: Mode) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum AddrMode {
        Addrdisabled = 0,
        Addrcfg = 1,
        Addr3b = 2,
        Addr4b = 3,
    }
    impl AddrMode {
        #[inline(always)]
        pub fn addr_disabled(&self) -> bool {
            *self == Self::Addrdisabled
        }
        #[inline(always)]
        pub fn addr_cfg(&self) -> bool {
            *self == Self::Addrcfg
        }
        #[inline(always)]
        pub fn addr3_b(&self) -> bool {
            *self == Self::Addr3b
        }
        #[inline(always)]
        pub fn addr4_b(&self) -> bool {
            *self == Self::Addr4b
        }
    }
    impl TryFrom<u32> for AddrMode {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<AddrMode, ()> {
            if val < 4 {
                Ok(unsafe { core::mem::transmute::<u32, AddrMode>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<AddrMode> for u32 {
        fn from(val: AddrMode) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum PayloadDir {
        Payloadin = 0,
        Payloadout = 1,
    }
    impl PayloadDir {
        #[inline(always)]
        pub fn payload_in(&self) -> bool {
            *self == Self::Payloadin
        }
        #[inline(always)]
        pub fn payload_out(&self) -> bool {
            *self == Self::Payloadout
        }
    }
    impl TryFrom<u32> for PayloadDir {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<PayloadDir, ()> {
            if val < 2 {
                Ok(unsafe { core::mem::transmute::<u32, PayloadDir>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<PayloadDir> for u32 {
        fn from(val: PayloadDir) -> Self {
            val as u32
        }
    }
    #[derive(Clone, Copy, Eq, PartialEq)]
    #[repr(u32)]
    pub enum ReadPipelineMode {
        ZeroStages = 0,
        TwoStagesHalfCycle = 1,
        TwoStagesFullCycle = 2,
        Reserved3 = 3,
    }
    impl ReadPipelineMode {
        #[inline(always)]
        pub fn zero_stages(&self) -> bool {
            *self == Self::ZeroStages
        }
        #[inline(always)]
        pub fn two_stages_half_cycle(&self) -> bool {
            *self == Self::TwoStagesHalfCycle
        }
        #[inline(always)]
        pub fn two_stages_full_cycle(&self) -> bool {
            *self == Self::TwoStagesFullCycle
        }
    }
    impl TryFrom<u32> for ReadPipelineMode {
        type Error = ();
        #[inline(always)]
        fn try_from(val: u32) -> Result<ReadPipelineMode, ()> {
            if val < 4 {
                Ok(unsafe { core::mem::transmute::<u32, ReadPipelineMode>(val) })
            } else {
                Err(())
            }
        }
    }
    impl From<ReadPipelineMode> for u32 {
        fn from(val: ReadPipelineMode) -> Self {
            val as u32
        }
    }
    pub mod selector {
        pub struct ModeSelector();
        impl ModeSelector {
            #[inline(always)]
            pub fn disabled(&self) -> super::Mode {
                super::Mode::Disabled
            }
            #[inline(always)]
            pub fn flashmode(&self) -> super::Mode {
                super::Mode::Flashmode
            }
            #[inline(always)]
            pub fn passthrough(&self) -> super::Mode {
                super::Mode::Passthrough
            }
        }
        pub struct AddrModeSelector();
        impl AddrModeSelector {
            #[inline(always)]
            pub fn addr_disabled(&self) -> super::AddrMode {
                super::AddrMode::Addrdisabled
            }
            #[inline(always)]
            pub fn addr_cfg(&self) -> super::AddrMode {
                super::AddrMode::Addrcfg
            }
            #[inline(always)]
            pub fn addr3_b(&self) -> super::AddrMode {
                super::AddrMode::Addr3b
            }
            #[inline(always)]
            pub fn addr4_b(&self) -> super::AddrMode {
                super::AddrMode::Addr4b
            }
        }
        pub struct PayloadDirSelector();
        impl PayloadDirSelector {
            #[inline(always)]
            pub fn payload_in(&self) -> super::PayloadDir {
                super::PayloadDir::Payloadin
            }
            #[inline(always)]
            pub fn payload_out(&self) -> super::PayloadDir {
                super::PayloadDir::Payloadout
            }
        }
        pub struct ReadPipelineModeSelector();
        impl ReadPipelineModeSelector {
            #[inline(always)]
            pub fn zero_stages(&self) -> super::ReadPipelineMode {
                super::ReadPipelineMode::ZeroStages
            }
            #[inline(always)]
            pub fn two_stages_half_cycle(&self) -> super::ReadPipelineMode {
                super::ReadPipelineMode::TwoStagesHalfCycle
            }
            #[inline(always)]
            pub fn two_stages_full_cycle(&self) -> super::ReadPipelineMode {
                super::ReadPipelineMode::TwoStagesFullCycle
            }
        }
    }
}
pub mod meta {
    #![doc = r" Additional metadata needed by ureg."]
    pub type IntrState =
        ureg::ReadWriteReg32<0, crate::regs::IntrStateReadVal, crate::regs::IntrStateWriteVal>;
    pub type IntrEnable =
        ureg::ReadWriteReg32<0, crate::regs::IntrEnableReadVal, crate::regs::IntrEnableWriteVal>;
    pub type IntrTest = ureg::WriteOnlyReg32<0, crate::regs::IntrTestWriteVal>;
    pub type AlertTest = ureg::WriteOnlyReg32<0, crate::regs::AlertTestWriteVal>;
    pub type Control =
        ureg::ReadWriteReg32<0x10, crate::regs::ControlReadVal, crate::regs::ControlWriteVal>;
    pub type Cfg = ureg::ReadWriteReg32<0, crate::regs::CfgReadVal, crate::regs::CfgWriteVal>;
    pub type Status = ureg::ReadOnlyReg32<crate::regs::StatusReadVal>;
    pub type InterceptEn =
        ureg::ReadWriteReg32<0, crate::regs::InterceptEnReadVal, crate::regs::InterceptEnWriteVal>;
    pub type AddrMode =
        ureg::ReadWriteReg32<0, crate::regs::AddrModeReadVal, crate::regs::AddrModeWriteVal>;
    pub type LastReadAddr = ureg::ReadOnlyReg32<u32>;
    pub type FlashStatus =
        ureg::ReadWriteReg32<0, crate::regs::FlashStatusReadVal, crate::regs::FlashStatusWriteVal>;
    pub type JedecCc =
        ureg::ReadWriteReg32<0x7f, crate::regs::JedecCcReadVal, crate::regs::JedecCcWriteVal>;
    pub type JedecId =
        ureg::ReadWriteReg32<0, crate::regs::JedecIdReadVal, crate::regs::JedecIdWriteVal>;
    pub type ReadThreshold = ureg::ReadWriteReg32<
        0,
        crate::regs::ReadThresholdReadVal,
        crate::regs::ReadThresholdWriteVal,
    >;
    pub type MailboxAddr = ureg::ReadWriteReg32<0, u32, u32>;
    pub type UploadStatus = ureg::ReadOnlyReg32<crate::regs::UploadStatusReadVal>;
    pub type UploadStatus2 = ureg::ReadOnlyReg32<crate::regs::UploadStatus2ReadVal>;
    pub type UploadCmdfifo = ureg::ReadOnlyReg32<crate::regs::UploadCmdfifoReadVal>;
    pub type UploadAddrfifo = ureg::ReadOnlyReg32<u32>;
    pub type CmdFilter0 =
        ureg::ReadWriteReg32<0, crate::regs::CmdFilter0ReadVal, crate::regs::CmdFilter0WriteVal>;
    pub type CmdFilter1 =
        ureg::ReadWriteReg32<0, crate::regs::CmdFilter1ReadVal, crate::regs::CmdFilter1WriteVal>;
    pub type CmdFilter2 =
        ureg::ReadWriteReg32<0, crate::regs::CmdFilter2ReadVal, crate::regs::CmdFilter2WriteVal>;
    pub type CmdFilter3 =
        ureg::ReadWriteReg32<0, crate::regs::CmdFilter3ReadVal, crate::regs::CmdFilter3WriteVal>;
    pub type CmdFilter4 =
        ureg::ReadWriteReg32<0, crate::regs::CmdFilter4ReadVal, crate::regs::CmdFilter4WriteVal>;
    pub type CmdFilter5 =
        ureg::ReadWriteReg32<0, crate::regs::CmdFilter5ReadVal, crate::regs::CmdFilter5WriteVal>;
    pub type CmdFilter6 =
        ureg::ReadWriteReg32<0, crate::regs::CmdFilter6ReadVal, crate::regs::CmdFilter6WriteVal>;
    pub type CmdFilter7 =
        ureg::ReadWriteReg32<0, crate::regs::CmdFilter7ReadVal, crate::regs::CmdFilter7WriteVal>;
    pub type AddrSwapMask = ureg::ReadWriteReg32<0, u32, u32>;
    pub type AddrSwapData = ureg::ReadWriteReg32<0, u32, u32>;
    pub type PayloadSwapMask = ureg::ReadWriteReg32<0, u32, u32>;
    pub type PayloadSwapData = ureg::ReadWriteReg32<0, u32, u32>;
    pub type CmdInfo =
        ureg::ReadWriteReg32<0x7000, crate::regs::CmdInfoReadVal, crate::regs::CmdInfoWriteVal>;
    pub type CmdInfoEn4b =
        ureg::ReadWriteReg32<0, crate::regs::CmdInfoEn4bReadVal, crate::regs::CmdInfoEn4bWriteVal>;
    pub type CmdInfoEx4b =
        ureg::ReadWriteReg32<0, crate::regs::CmdInfoEx4bReadVal, crate::regs::CmdInfoEx4bWriteVal>;
    pub type CmdInfoWren =
        ureg::ReadWriteReg32<0, crate::regs::CmdInfoWrenReadVal, crate::regs::CmdInfoWrenWriteVal>;
    pub type CmdInfoWrdi =
        ureg::ReadWriteReg32<0, crate::regs::CmdInfoWrdiReadVal, crate::regs::CmdInfoWrdiWriteVal>;
    pub type TpmCap = ureg::ReadOnlyReg32<crate::regs::TpmCapReadVal>;
    pub type TpmCfg =
        ureg::ReadWriteReg32<0, crate::regs::TpmCfgReadVal, crate::regs::TpmCfgWriteVal>;
    pub type TpmStatus =
        ureg::ReadWriteReg32<0, crate::regs::TpmStatusReadVal, crate::regs::TpmStatusWriteVal>;
    pub type TpmAccess0 =
        ureg::ReadWriteReg32<0, crate::regs::TpmAccess0ReadVal, crate::regs::TpmAccess0WriteVal>;
    pub type TpmAccess1 =
        ureg::ReadWriteReg32<0, crate::regs::TpmAccess1ReadVal, crate::regs::TpmAccess1WriteVal>;
    pub type TpmSts = ureg::ReadWriteReg32<0, u32, u32>;
    pub type TpmIntfCapability = ureg::ReadWriteReg32<0, u32, u32>;
    pub type TpmIntEnable = ureg::ReadWriteReg32<0, u32, u32>;
    pub type TpmIntVector = ureg::ReadWriteReg32<
        0,
        crate::regs::TpmIntVectorReadVal,
        crate::regs::TpmIntVectorWriteVal,
    >;
    pub type TpmIntStatus = ureg::ReadWriteReg32<0, u32, u32>;
    pub type TpmDidVid =
        ureg::ReadWriteReg32<0, crate::regs::TpmDidVidReadVal, crate::regs::TpmDidVidWriteVal>;
    pub type TpmRid =
        ureg::ReadWriteReg32<0, crate::regs::TpmRidReadVal, crate::regs::TpmRidWriteVal>;
    pub type TpmCmdAddr = ureg::ReadOnlyReg32<crate::regs::TpmCmdAddrReadVal>;
    pub type TpmReadFifo = ureg::WriteOnlyReg32<0, u32>;
    pub type EgressBuffer = ureg::WriteOnlyReg32<0, u32>;
    pub type IngressBuffer = ureg::ReadOnlyReg32<u32>;
}

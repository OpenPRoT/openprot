// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! OpenTitan SPI Device driver for Earlgrey.

#![no_std]

use aligned::{Aligned, A4};
use util_error::ErrorCode;
use util_regcpy::{copy_from_reg_array, copy_to_reg_array};

// SRAM buffer constants are based upon the OpenTitan programmers guide:
// https://opentitan.org/book/hw/ip/spi_device/doc/programmers_guide.html

// Egress buffer constants
pub const MAILBOX_START_WORDS: usize = 0x800 / 4;
pub const MAILBOX_LEN_WORDS: usize = 1024 / 4;
pub const SFDP_START_WORDS: usize = 0xC00 / 4;
pub const SFDP_LEN_WORDS: usize = 256 / 4;

// Ingress buffer constants
pub const PAYLOAD_FIFO_START_WORDS: usize = 0;
pub const PAYLOAD_FIFO_LEN_WORDS: usize = 256 / 4;

// Command info list slots
pub const CMD_INFO_READ_STATUS: u8 = 0;
pub const CMD_INFO_JEDEC: u8 = 3;
pub const CMD_INFO_SFDP: u8 = 4;
pub const CMD_INFO_READ: u8 = 5;
pub const CMD_INFO_FASTREAD: u8 = 6;
pub const CMD_INFO_READ4B: u8 = 7;
pub const CMD_INFO_FAST_QUAD_READ: u8 = 8;
pub const CMD_INFO_FAST_QUAD_READ4B: u8 = 9;
pub const CMD_INFO_PAGEPROGRAM: u8 = 11;
pub const CMD_INFO_PAGEPROGRAM4B: u8 = 12;
pub const CMD_INFO_SECTORERASE: u8 = 13;
pub const CMD_INFO_SECTORERASE4B: u8 = 14;
pub const CMD_INFO_BLOCKERASE32K: u8 = 15;
pub const CMD_INFO_BLOCKERASE32K4B: u8 = 16;
pub const CMD_INFO_BLOCKERASE64K: u8 = 17;
pub const CMD_INFO_BLOCKERASE64K4B: u8 = 18;
pub const CMD_INFO_CHIPERASE: u8 = 19;
pub const CMD_INFO_CHIPERASE2: u8 = 20;
pub const CMD_INFO_PAGEPROGRAMQUAD: u8 = 21;
pub const CMD_INFO_PAGEPROGRAMQUAD4B: u8 = 22;

pub use spi_flash_opcode::Opcode as SpiFlashOpcode;

pub struct SpiDev {
    mmio: spi_device::RegisterBlock<ureg::RealMmioMut<'static>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpiPayloadIoCfg {
    SingleIoIn,  // Payload is sent on the MOSI line (IO[0])
    SingleIoOut, // Payload is returned on the MISO line (IO[1])
    DualIoIn,
    DualIoOut,
    QuadIoIn,
    QuadIoOut,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddrMode {
    CFG,
    _3B,
    _4B,
}

pub struct SpiFlashCmdCfg {
    pub opcode: SpiFlashOpcode,
    pub upload: bool,
    pub busy: bool,
    pub payload_io: Option<SpiPayloadIoCfg>,
    pub addr_mode: Option<AddrMode>,
    pub dummy_cyc: u8,
    pub filter: bool,
}

impl SpiFlashCmdCfg {
    pub const JEDEC_ID: Self = Self {
        opcode: SpiFlashOpcode::JEDEC_ID,
        upload: false,
        busy: false,
        payload_io: Some(SpiPayloadIoCfg::SingleIoOut),
        addr_mode: None,
        dummy_cyc: 0,
        filter: true,
    };

    pub const READ_STATUS: Self = Self {
        opcode: SpiFlashOpcode::READ_STATUS,
        upload: false,
        busy: false,
        payload_io: Some(SpiPayloadIoCfg::SingleIoOut),
        addr_mode: None,
        dummy_cyc: 0,
        filter: false,
    };

    pub const READ: Self = Self {
        opcode: SpiFlashOpcode::READ,
        upload: false,
        busy: false,
        payload_io: Some(SpiPayloadIoCfg::SingleIoOut),
        addr_mode: Some(AddrMode::CFG),
        dummy_cyc: 0,
        filter: false,
    };

    pub const FAST_READ: Self = Self {
        opcode: SpiFlashOpcode::FAST_READ,
        upload: false,
        busy: false,
        payload_io: Some(SpiPayloadIoCfg::SingleIoOut),
        addr_mode: Some(AddrMode::CFG),
        dummy_cyc: 8,
        filter: false,
    };

    pub const FAST_QUAD_READ: Self = Self {
        opcode: SpiFlashOpcode::FAST_QUAD_READ,
        upload: false,
        busy: false,
        payload_io: Some(SpiPayloadIoCfg::QuadIoOut),
        addr_mode: Some(AddrMode::CFG),
        dummy_cyc: 8,
        filter: false,
    };

    pub const READ_4B: Self = Self {
        opcode: SpiFlashOpcode::READ_4B,
        upload: false,
        busy: false,
        payload_io: Some(SpiPayloadIoCfg::SingleIoOut),
        addr_mode: Some(AddrMode::_4B),
        dummy_cyc: 0,
        filter: false,
    };

    pub const FAST_QUAD_READ_4B: Self = Self {
        opcode: SpiFlashOpcode::FAST_QUAD_READ_4B,
        upload: false,
        busy: false,
        payload_io: Some(SpiPayloadIoCfg::QuadIoOut),
        addr_mode: Some(AddrMode::_4B),
        dummy_cyc: 8,
        filter: false,
    };

    pub const SFDP: Self = Self {
        opcode: SpiFlashOpcode::SFDP,
        upload: false,
        busy: false,
        payload_io: Some(SpiPayloadIoCfg::SingleIoOut),
        addr_mode: Some(AddrMode::_3B),
        dummy_cyc: 8,
        filter: true,
    };

    pub const PAGE_PROGRAM: Self = Self {
        opcode: SpiFlashOpcode::PAGE_PROGRAM,
        upload: true,
        busy: true,
        payload_io: Some(SpiPayloadIoCfg::SingleIoIn),
        addr_mode: Some(AddrMode::CFG),
        dummy_cyc: 0,
        filter: true,
    };

    pub const PAGE_PROGRAM_QUAD: Self = Self {
        opcode: SpiFlashOpcode::PAGE_PROGRAM_QUAD,
        upload: true,
        busy: true,
        payload_io: Some(SpiPayloadIoCfg::QuadIoIn),
        addr_mode: Some(AddrMode::CFG),
        dummy_cyc: 0,
        filter: true,
    };

    pub const PAGE_PROGRAM_4B: Self = Self {
        opcode: SpiFlashOpcode::PAGE_PROGRAM_4B,
        upload: true,
        busy: true,
        payload_io: Some(SpiPayloadIoCfg::SingleIoIn),
        addr_mode: Some(AddrMode::_4B),
        dummy_cyc: 0,
        filter: true,
    };

    pub const PAGE_PROGRAM_QUAD_4B: Self = Self {
        opcode: SpiFlashOpcode::PAGE_PROGRAM_QUAD_4B,
        upload: true,
        busy: true,
        payload_io: Some(SpiPayloadIoCfg::QuadIoIn),
        addr_mode: Some(AddrMode::_4B),
        dummy_cyc: 0,
        filter: true,
    };

    pub const SECTOR_ERASE: Self = Self {
        opcode: SpiFlashOpcode::SECTOR_ERASE,
        upload: true,
        busy: true,
        payload_io: None,
        addr_mode: Some(AddrMode::CFG),
        dummy_cyc: 0,
        filter: true,
    };

    pub const SECTOR_ERASE_4B: Self = Self {
        opcode: SpiFlashOpcode::SECTOR_ERASE_4B,
        upload: true,
        busy: true,
        payload_io: None,
        addr_mode: Some(AddrMode::_4B),
        dummy_cyc: 0,
        filter: true,
    };

    pub const BLOCK_ERASE_32K: Self = Self {
        opcode: SpiFlashOpcode::BLOCK_ERASE_32K,
        upload: true,
        busy: true,
        payload_io: None,
        addr_mode: Some(AddrMode::CFG),
        dummy_cyc: 0,
        filter: true,
    };

    pub const BLOCK_ERASE_32K_4B: Self = Self {
        opcode: SpiFlashOpcode::BLOCK_ERASE_32K_4B,
        upload: true,
        busy: true,
        payload_io: None,
        addr_mode: Some(AddrMode::_4B),
        dummy_cyc: 0,
        filter: true,
    };

    pub const BLOCK_ERASE_64K: Self = Self {
        opcode: SpiFlashOpcode::BLOCK_ERASE_64K,
        upload: true,
        busy: true,
        payload_io: None,
        addr_mode: Some(AddrMode::CFG),
        dummy_cyc: 0,
        filter: true,
    };

    pub const BLOCK_ERASE_64K_4B: Self = Self {
        opcode: SpiFlashOpcode::BLOCK_ERASE_64K_4B,
        upload: true,
        busy: true,
        payload_io: None,
        addr_mode: Some(AddrMode::_4B),
        dummy_cyc: 0,
        filter: true,
    };

    pub const CHIP_ERASE: Self = Self {
        opcode: SpiFlashOpcode::CHIP_ERASE,
        upload: true,
        busy: true,
        payload_io: None,
        addr_mode: None,
        dummy_cyc: 0,
        filter: true,
    };

    pub const CHIP_ERASE2: Self = Self {
        opcode: SpiFlashOpcode::CHIP_ERASE,
        upload: true,
        busy: true,
        payload_io: None,
        addr_mode: None,
        dummy_cyc: 0,
        filter: true,
    };
}

/// JEP-106 Identification code config
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct JedecIdConfig {
    /// Number of continuation codes
    pub num_cc: u8,
    /// Manufacturing ID
    pub manf_id: u8,
    /// Device ID
    pub dev_id: u16,
}

impl JedecIdConfig {
    pub const GOOGLE: Self = Self {
        num_cc: 0x8,
        manf_id: 0x26,
        dev_id: (0x17 << 8) | 0x31,
    };
}

// The JEDEC Identity Continuation Code
pub const JEDEC_CC: u32 = 0x7F;

pub struct SpiFlashCmd<'a> {
    pub opcode: SpiFlashOpcode,
    pub wel: bool,
    pub busy: bool,
    pub address: Option<u32>,
    pub payload: Option<&'a mut Aligned<A4, [u8]>>,
}

pub use spi_device::enums::Mode;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpiDevCfg {
    pub jedec: JedecIdConfig,
    pub mailbox: Option<u32>,
    pub mode: Mode,
    pub initial_address_mode_4b: bool,
}

impl Default for SpiDevCfg {
    fn default() -> Self {
        Self {
            jedec: JedecIdConfig::GOOGLE,
            mailbox: None,
            mode: Mode::Flashmode,
            initial_address_mode_4b: true,
        }
    }
}

pub trait SpiDevice {
    fn write_to_mbx(&mut self, payload: &Aligned<A4, [u8]>);
    fn poll<'a>(&mut self, payload_buf: &'a mut Aligned<A4, [u8]>) -> Option<SpiFlashCmd<'a>>;
    fn retire_cmd(&mut self);
    fn set_mode(&mut self, mode: Mode);
    /// Under passthrough mode, intercept read address and swap the high address to achieve bank switching.
    fn read_addr_swap(
        &mut self,
        enable: bool,
        swap_addr_mask: Option<u32>,
        swap_addr_data: Option<u32>,
    );
}

impl SpiDev {
    /// Create a new SpiDev driver instance.
    ///
    /// # Safety
    ///
    /// The caller must ensure exclusive ownership of the `spi_device` peripheral register block.
    pub unsafe fn new(mmio: spi_device::RegisterBlock<ureg::RealMmioMut<'static>>) -> Self {
        Self { mmio }
    }

    /// Initialize the SPI Device peripheral with the provided configuration.
    pub fn init(&mut self, cfg: &SpiDevCfg) -> Result<(), ErrorCode> {
        self.mmio.control().write(|w| w.with_mode(cfg.mode));

        self.mmio
            .egress_buffer()
            .get_sub_array::<MAILBOX_LEN_WORDS>(MAILBOX_START_WORDS)
            .unwrap()
            .fill(0xD15EA5ED);

        self.mmio
            .jedec_cc()
            .write(|w| w.cc(JEDEC_CC).num_cc(cfg.jedec.num_cc.into()));

        self.mmio
            .jedec_id()
            .write(|w| w.mf(cfg.jedec.manf_id.into()).id(cfg.jedec.dev_id.into()));

        self.mmio
            .addr_mode()
            .write(|w| w.addr_4b_en(cfg.initial_address_mode_4b));

        self.mmio
            .cmd_info_wren()
            .write(|w| w.opcode(SpiFlashOpcode::WRITE_ENABLE.into()).valid(true));

        self.mmio
            .cmd_info_wrdi()
            .write(|w| w.opcode(SpiFlashOpcode::WRITE_DISABLE.into()).valid(true));

        self.mmio.cmd_info_en4_b().write(|w| {
            w.opcode(SpiFlashOpcode::ENTER_4B_ADDR_MODE.into())
                .valid(true)
        });

        self.mmio.cmd_info_ex4_b().write(|w| {
            w.opcode(SpiFlashOpcode::EXIT_4B_ADDR_MODE.into())
                .valid(true)
        });

        if let Some(mbx_addr) = cfg.mailbox {
            self.mmio.cfg().write(|w| w.mailbox_en(true));
            self.mmio.mailbox_addr().write(|_| mbx_addr);
        }

        self.mmio.intercept_en().write(|w| {
            w.sfdp(true)
                .jedec(true)
                .status(true)
                .mbx(cfg.mailbox.is_some())
        });

        self.mmio
            .intr_enable()
            .write(|w| w.upload_cmdfifo_not_empty(true));

        self.configure_cmd_info(CMD_INFO_READ_STATUS, &SpiFlashCmdCfg::READ_STATUS);
        self.configure_cmd_info(CMD_INFO_JEDEC, &SpiFlashCmdCfg::JEDEC_ID);
        self.configure_cmd_info(CMD_INFO_READ, &SpiFlashCmdCfg::READ);
        self.configure_cmd_info(CMD_INFO_FASTREAD, &SpiFlashCmdCfg::FAST_READ);
        self.configure_cmd_info(CMD_INFO_FAST_QUAD_READ, &SpiFlashCmdCfg::FAST_QUAD_READ);
        self.configure_cmd_info(CMD_INFO_READ4B, &SpiFlashCmdCfg::READ_4B);
        self.configure_cmd_info(
            CMD_INFO_FAST_QUAD_READ4B,
            &SpiFlashCmdCfg::FAST_QUAD_READ_4B,
        );
        self.configure_cmd_info(CMD_INFO_SFDP, &SpiFlashCmdCfg::SFDP);
        self.configure_cmd_info(CMD_INFO_PAGEPROGRAM, &SpiFlashCmdCfg::PAGE_PROGRAM);
        self.configure_cmd_info(CMD_INFO_PAGEPROGRAMQUAD, &SpiFlashCmdCfg::PAGE_PROGRAM_QUAD);
        self.configure_cmd_info(CMD_INFO_PAGEPROGRAM4B, &SpiFlashCmdCfg::PAGE_PROGRAM_4B);
        self.configure_cmd_info(
            CMD_INFO_PAGEPROGRAMQUAD4B,
            &SpiFlashCmdCfg::PAGE_PROGRAM_QUAD_4B,
        );
        self.configure_cmd_info(CMD_INFO_SECTORERASE, &SpiFlashCmdCfg::SECTOR_ERASE);
        self.configure_cmd_info(CMD_INFO_SECTORERASE4B, &SpiFlashCmdCfg::SECTOR_ERASE_4B);
        self.configure_cmd_info(CMD_INFO_BLOCKERASE32K, &SpiFlashCmdCfg::BLOCK_ERASE_32K);
        self.configure_cmd_info(
            CMD_INFO_BLOCKERASE32K4B,
            &SpiFlashCmdCfg::BLOCK_ERASE_32K_4B,
        );
        self.configure_cmd_info(CMD_INFO_BLOCKERASE64K, &SpiFlashCmdCfg::BLOCK_ERASE_64K);
        self.configure_cmd_info(
            CMD_INFO_BLOCKERASE64K4B,
            &SpiFlashCmdCfg::BLOCK_ERASE_64K_4B,
        );
        self.configure_cmd_info(CMD_INFO_CHIPERASE, &SpiFlashCmdCfg::CHIP_ERASE);
        self.configure_cmd_info(CMD_INFO_CHIPERASE2, &SpiFlashCmdCfg::CHIP_ERASE2);

        Ok(())
    }

    /// Populate the SFDP table in the SRAM egress buffer.
    pub fn set_sfdp(&mut self, sfdp: &Aligned<A4, [u8]>) {
        let sfdp_regs = self
            .mmio
            .egress_buffer()
            .get_sub_array::<SFDP_LEN_WORDS>(SFDP_START_WORDS)
            .unwrap();

        copy_to_reg_array(&sfdp_regs, sfdp);
    }

    /// Configure a command slot in the CMD_INFO array and apply command filter if required.
    pub fn configure_cmd_info(&mut self, slot: u8, cfg: &SpiFlashCmdCfg) {
        self.mmio.cmd_info().at(slot.into()).write(|w| {
            w.valid(true)
                .opcode(cfg.opcode.into())
                .payload_dir(|w| match cfg.payload_io {
                    None
                    | Some(SpiPayloadIoCfg::SingleIoIn)
                    | Some(SpiPayloadIoCfg::DualIoIn)
                    | Some(SpiPayloadIoCfg::QuadIoIn) => w.payload_in(),
                    Some(SpiPayloadIoCfg::SingleIoOut)
                    | Some(SpiPayloadIoCfg::DualIoOut)
                    | Some(SpiPayloadIoCfg::QuadIoOut) => w.payload_out(),
                })
                .upload(cfg.upload)
                .payload_en(match cfg.payload_io {
                    None => 0,
                    Some(SpiPayloadIoCfg::SingleIoIn) => 0x01,
                    Some(SpiPayloadIoCfg::SingleIoOut) => 0x02,
                    Some(SpiPayloadIoCfg::DualIoIn) => 0x03,
                    Some(SpiPayloadIoCfg::DualIoOut) => 0x03,
                    Some(SpiPayloadIoCfg::QuadIoIn) => 0x0F,
                    Some(SpiPayloadIoCfg::QuadIoOut) => 0x0F,
                })
                .addr_swap_en(false)
                .busy(cfg.busy)
                .addr_mode(|w| match cfg.addr_mode {
                    None => w.addr_disabled(),
                    Some(AddrMode::CFG) => w.addr_cfg(),
                    Some(AddrMode::_3B) => w.addr3_b(),
                    Some(AddrMode::_4B) => w.addr4_b(),
                })
                .dummy_en(cfg.dummy_cyc > 0)
                .dummy_size(cfg.dummy_cyc.wrapping_sub(1).into())
                .mbyte_en(false)
        });

        if cfg.filter {
            let f_slot = cfg.opcode.0 / 32;
            let idx: u32 = 1 << (cfg.opcode.0 % 32);

            match f_slot {
                0 => self
                    .mmio
                    .cmd_filter0()
                    .read_and_modify(|_w, r| (u32::from(r) | idx).into()),
                1 => self
                    .mmio
                    .cmd_filter1()
                    .read_and_modify(|_w, r| (u32::from(r) | idx).into()),
                2 => self
                    .mmio
                    .cmd_filter2()
                    .read_and_modify(|_w, r| (u32::from(r) | idx).into()),
                3 => self
                    .mmio
                    .cmd_filter3()
                    .read_and_modify(|_w, r| (u32::from(r) | idx).into()),
                4 => self
                    .mmio
                    .cmd_filter4()
                    .read_and_modify(|_w, r| (u32::from(r) | idx).into()),
                5 => self
                    .mmio
                    .cmd_filter5()
                    .read_and_modify(|_w, r| (u32::from(r) | idx).into()),
                6 => self
                    .mmio
                    .cmd_filter6()
                    .read_and_modify(|_w, r| (u32::from(r) | idx).into()),
                7 => self
                    .mmio
                    .cmd_filter7()
                    .read_and_modify(|_w, r| (u32::from(r) | idx).into()),
                _ => {
                    unreachable!("Error configuring cmd_filter")
                }
            }
        }
    }
    /// Write a payload into the mailbox egress buffer.
    pub fn write_to_mbx(&mut self, payload: &Aligned<A4, [u8]>) {
        let mailbox = self
            .mmio
            .egress_buffer()
            .get_sub_array::<MAILBOX_LEN_WORDS>(MAILBOX_START_WORDS)
            .unwrap();

        copy_to_reg_array(&mailbox, payload);
    }

    /// Poll for uploaded SPI flash commands from host.
    pub fn poll<'a>(
        &mut self,
        mut payload_buf: &'a mut Aligned<A4, [u8]>,
    ) -> Option<SpiFlashCmd<'a>> {
        let upload_status = self.mmio.upload_status().read();
        if !upload_status.cmdfifo_notempty() {
            return None;
        }

        let uploadstatus2 = self.mmio.upload_status2().read();
        if uploadstatus2.payload_start_idx() != 0 {
            // Payload overflow, drop the command
            self.retire_cmd();
            return None;
        }

        let upload_cmdfifo = self.mmio.upload_cmdfifo().read();

        let opcode: u8 = upload_cmdfifo.data() as u8;
        let addr = if upload_status.addrfifo_notempty() {
            Some(self.mmio.upload_addrfifo().read())
        } else {
            None
        };

        let payload_len = uploadstatus2.payload_depth() as u16;
        if payload_len > 256 {
            self.retire_cmd();
            return None;
        }

        payload_buf = &mut payload_buf[..payload_len.into()];

        let payload_fifo = self
            .mmio
            .ingress_buffer()
            .get_sub_array::<PAYLOAD_FIFO_LEN_WORDS>(PAYLOAD_FIFO_START_WORDS)
            .unwrap();

        copy_from_reg_array(payload_buf, &payload_fifo);

        self.mmio.intr_state().write(|w| {
            w.upload_cmdfifo_not_empty_clear()
                .upload_payload_overflow_clear()
                .upload_payload_not_empty_clear()
        });

        Some(SpiFlashCmd {
            opcode: SpiFlashOpcode(opcode),
            wel: upload_cmdfifo.wel(),
            busy: upload_cmdfifo.busy(),
            address: addr,
            payload: Some(payload_buf),
        })
    }

    /// Clear busy and WEL in flash status.
    pub fn retire_cmd(&mut self) {
        self.mmio
            .flash_status()
            .write(|w| w.busy_clear().wel_clear());
    }

    /// Set SPI device operation mode.
    pub fn set_mode(&mut self, mode: Mode) {
        self.mmio.control().modify(|w| w.with_mode(mode));
    }

    /// Configure address swapping for read commands.
    pub fn read_addr_swap(
        &mut self,
        enable: bool,
        swap_addr_mask: Option<u32>,
        swap_addr_data: Option<u32>,
    ) {
        // Temporarily disable address swapping for read commands before modifying registers.
        for cmd in [CMD_INFO_READ, CMD_INFO_FASTREAD, CMD_INFO_READ4B] {
            if let Some(reg) = self.mmio.cmd_info().get(usize::from(cmd)) {
                reg.modify(|w| w.addr_swap_en(false));
            }
        }

        if !enable {
            return;
        }

        if let Some(mask) = swap_addr_mask {
            self.mmio.addr_swap_mask().write(|_| mask);
        }

        if let Some(data) = swap_addr_data {
            self.mmio.addr_swap_data().write(|_| data);
        }

        // Re-enable address swapping for the relevant read commands.
        for cmd in [CMD_INFO_READ, CMD_INFO_FASTREAD, CMD_INFO_READ4B] {
            if let Some(reg) = self.mmio.cmd_info().get(usize::from(cmd)) {
                reg.modify(|w| w.addr_swap_en(true));
            }
        }
    }
}

impl SpiDevice for SpiDev {
    fn write_to_mbx(&mut self, payload: &Aligned<A4, [u8]>) {
        self.write_to_mbx(payload)
    }

    fn poll<'a>(&mut self, payload_buf: &'a mut Aligned<A4, [u8]>) -> Option<SpiFlashCmd<'a>> {
        self.poll(payload_buf)
    }

    fn retire_cmd(&mut self) {
        self.retire_cmd()
    }

    fn set_mode(&mut self, mode: Mode) {
        self.set_mode(mode)
    }

    fn read_addr_swap(
        &mut self,
        enable: bool,
        swap_addr_mask: Option<u32>,
        swap_addr_data: Option<u32>,
    ) {
        self.read_addr_swap(enable, swap_addr_mask, swap_addr_data)
    }
}

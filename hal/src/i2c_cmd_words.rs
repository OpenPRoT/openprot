// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Opt-in command-word I2C kit: the target fills the bit values, the HAL owns the vocabulary + one-write assembly. Not the mandatory I2C seam; FIFO chips implement `embedded_hal::i2c::I2c` directly instead.

use crate::field_mux::RegBlock;
use core::marker::PhantomData;

/// Target-supplied I2C master COMMAND register: the named config surface a new target fills out.
/// HAL owns the command vocabulary; the target supplies this chip's bit values + register offset.
pub trait I2cMasterCmdBits {
    /// Command register offset.
    const OFFSET: u32;
    /// Enable packet mode.
    const PACKET: u32;
    /// Send START condition.
    const START: u32;
    /// Send STOP condition.
    const STOP: u32;
    /// TX command.
    const TX: u32;
    /// RX command (ACK).
    const RX: u32;
    /// RX command with NACK on the last byte.
    const RX_LAST: u32;
    /// Enable TX buffer mode.
    const TX_BUFF: u32;
    /// Enable RX buffer mode.
    const RX_BUFF: u32;
    /// Enable TX DMA mode.
    const TX_DMA: u32;
    /// Enable RX DMA mode.
    const RX_DMA: u32;
    /// Bus-recovery bit (set via RMW, not a fresh command word).
    const RECOVER: u32;
    /// Encode the packet-mode target address field.
    fn pkt_addr(addr: u8) -> u32;
}

/// Target-supplied I2C master STATUS register: the named config surface a new target fills out.
/// HAL owns the status vocabulary; the target supplies this chip's flag values + register offset.
pub trait I2cMasterStatusBits {
    /// Status register offset (write-1-to-clear).
    const OFFSET: u32;
    /// TX received ACK.
    const TX_ACK: u32;
    /// TX received NACK.
    const TX_NAK: u32;
    /// RX transfer done.
    const RX_DONE: u32;
    /// Arbitration loss.
    const ARBIT_LOSS: u32;
    /// Normal STOP condition.
    const NORMAL_STOP: u32;
    /// Abnormal STOP condition.
    const ABNORMAL: u32;
    /// SCL low timeout.
    const SCL_LOW_TO: u32;
    /// Packet mode done.
    const PKT_DONE: u32;
    /// Packet mode error.
    const PKT_ERROR: u32;
    /// Bus recovery done.
    const BUS_RECOVER: u32;
    /// Bus recovery failed.
    const BUS_RECOVER_FAIL: u32;
    /// SDA data line timeout.
    const SDA_DL_TO: u32;
}

/// Master command bit names — the value of each is resolved from the target's `I2cMasterCmdBits`.
#[derive(Clone, Copy)]
pub enum I2cCmd {
    /// Enable packet mode.
    Packet,
    /// Send START condition.
    Start,
    /// Send STOP condition.
    Stop,
    /// TX command.
    Tx,
    /// RX command (ACK).
    Rx,
    /// RX command with NACK on the last byte.
    RxLast,
    /// Enable TX buffer mode.
    TxBuff,
    /// Enable RX buffer mode.
    RxBuff,
    /// Enable TX DMA mode.
    TxDma,
    /// Enable RX DMA mode.
    RxDma,
}

impl I2cCmd {
    /// This command's bit mask in the target's command register.
    #[must_use]
    pub const fn mask<B: I2cMasterCmdBits>(self) -> u32 {
        match self {
            I2cCmd::Packet => B::PACKET,
            I2cCmd::Start => B::START,
            I2cCmd::Stop => B::STOP,
            I2cCmd::Tx => B::TX,
            I2cCmd::Rx => B::RX,
            I2cCmd::RxLast => B::RX_LAST,
            I2cCmd::TxBuff => B::TX_BUFF,
            I2cCmd::RxBuff => B::RX_BUFF,
            I2cCmd::TxDma => B::TX_DMA,
            I2cCmd::RxDma => B::RX_DMA,
        }
    }
}

/// Master status flag names — the value of each is resolved from the target's `I2cMasterStatusBits`.
#[derive(Clone, Copy)]
pub enum I2cStat {
    /// TX received ACK.
    TxAck,
    /// TX received NACK.
    TxNak,
    /// RX transfer done.
    RxDone,
    /// Arbitration loss.
    ArbitLoss,
    /// Normal STOP condition.
    NormalStop,
    /// Abnormal STOP condition.
    Abnormal,
    /// SCL low timeout.
    SclLowTo,
    /// Packet mode done.
    PktDone,
    /// Packet mode error.
    PktError,
    /// Bus recovery done.
    BusRecover,
    /// Bus recovery failed.
    BusRecoverFail,
    /// SDA data line timeout.
    SdaDlTo,
}

impl I2cStat {
    /// This flag's bit mask in the target's status register.
    #[must_use]
    pub const fn mask<B: I2cMasterStatusBits>(self) -> u32 {
        match self {
            I2cStat::TxAck => B::TX_ACK,
            I2cStat::TxNak => B::TX_NAK,
            I2cStat::RxDone => B::RX_DONE,
            I2cStat::ArbitLoss => B::ARBIT_LOSS,
            I2cStat::NormalStop => B::NORMAL_STOP,
            I2cStat::Abnormal => B::ABNORMAL,
            I2cStat::SclLowTo => B::SCL_LOW_TO,
            I2cStat::PktDone => B::PKT_DONE,
            I2cStat::PktError => B::PKT_ERROR,
            I2cStat::BusRecover => B::BUS_RECOVER,
            I2cStat::BusRecoverFail => B::BUS_RECOVER_FAIL,
            I2cStat::SdaDlTo => B::SDA_DL_TO,
        }
    }
}

/// Builder for the master command register: only command names are reachable, so a status flag can
/// never be written as one. `PhantomData<B>` keeps it a plain `u32` at runtime.
#[derive(Clone, Copy)]
pub struct I2cMasterCmd<B: I2cMasterCmdBits>(u32, PhantomData<B>);

impl<B: I2cMasterCmdBits> I2cMasterCmd<B> {
    /// Start a packet-mode command — every full-command site begins here.
    #[must_use]
    pub fn packet() -> Self {
        Self(B::PACKET, PhantomData)
    }

    /// OR in one command bit.
    #[must_use]
    pub fn with(self, c: I2cCmd) -> Self {
        Self(self.0 | c.mask::<B>(), PhantomData)
    }

    /// Set the packet-mode target address field.
    #[must_use]
    pub fn address(self, addr: u8) -> Self {
        Self(self.0 | B::pkt_addr(addr), PhantomData)
    }

    /// Write the assembled command (one register write).
    pub fn issue(self, regs: &impl RegBlock) {
        regs.write_reg(B::OFFSET, self.0);
    }

    /// Issue the bus-recovery command (read-modify-write of the one recovery bit).
    pub fn recover(regs: &impl RegBlock) {
        let cur = regs.read_reg(B::OFFSET);
        regs.write_reg(B::OFFSET, cur | B::RECOVER);
    }
}

/// Flag-set over the master status register: only status names are reachable, so no command bit is
/// testable or clearable here. `PhantomData<B>` keeps it a plain `u32` at runtime.
#[derive(Clone, Copy)]
pub struct I2cMasterStatus<B: I2cMasterStatusBits>(u32, PhantomData<B>);

impl<B: I2cMasterStatusBits> I2cMasterStatus<B> {
    /// The clear-everything (write-1-to-clear) mask.
    #[must_use]
    pub const fn all() -> Self {
        Self(0xffff_ffff, PhantomData)
    }

    /// A single-flag clear mask.
    #[must_use]
    pub const fn flag(s: I2cStat) -> Self {
        Self(s.mask::<B>(), PhantomData)
    }

    /// Read the current status word.
    pub fn read(regs: &impl RegBlock) -> Self {
        Self(regs.read_reg(B::OFFSET), PhantomData)
    }

    /// True if `s` is set.
    #[must_use]
    pub fn has(self, s: I2cStat) -> bool {
        self.0 & s.mask::<B>() != 0
    }

    /// Raw status word — for the few sites that round-trip the full value.
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Write these flags back (write-1-to-clear the bits held).
    pub fn clear(self, regs: &impl RegBlock) {
        regs.write_reg(B::OFFSET, self.0);
    }
}

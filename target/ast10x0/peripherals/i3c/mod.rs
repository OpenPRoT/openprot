// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST1060 I3C bare-metal driver core
//!
//! # Overview
//!
//! This module provides a hardware abstraction layer for I3C controllers,
//! supporting both controller (master) and target (secondary) modes. It is a
//! behavioral-parity port of `aspeed-rust/src/i3c/` @ ce3b567 into the openprot
//! AST10x0 peripheral HAL. See `plans/goal.md` for the parity standard and the
//! deltas ledger (notably: confined-`unsafe` façade + injected yield closure on
//! [`hardware::Ast1060I3c`], and `proposed_traits` replaced by inherent methods
//! on [`controller::I3cController`]).
//!
//! # Architecture
//!
//! - [`controller`]: Main I3C controller abstraction + master/target operations
//! - [`config`]: Configuration types and device management
//! - [`types`]: Core data types (commands, messages, transfers)
//! - [`error`]: Error types
//! - [`constants`]: Hardware register definitions
//! - [`hardware`]: Hardware interface (traits + AST1060 implementation)
//! - [`ccc`]: Common Command Code operations
//! - [`ibi`]: In-Band Interrupt work queue
//!
//! # Features
//!
//! - I3C SDR and HDR modes
//! - Dynamic address assignment (ENTDAA)
//! - In-Band Interrupts (IBI)
//! - Hot-Join support
//! - Target mode operation
//! - Legacy I2C device support

pub mod ccc;
pub mod config;
pub mod constants;
pub mod controller;
pub mod error;
pub mod hardware;
pub mod ibi;
pub mod registers;
pub mod types;

// =============================================================================
// Public Re-exports
// =============================================================================

// Controller (two-state lifecycle: Uninitialized -> Ready, matching the SMC
// peripheral's precedent)
pub use controller::{I3cController, I3cCore, Ready, Uninitialized};

// Error types
pub use error::{CccErrorKind, I3cError, Result};

// Configuration
pub use config::{
    AddrBook, Attached, CommonCfg, CommonState, DeviceEntry, I3C_MAX_CORE_CLK,
    I3C_MIN_CORE_CLK_HDR, I3C_MIN_CORE_CLK_SDR, I3cConfig, I3cTargetConfig, ResetSpec,
};

// Core types
pub use types::{
    Completion, DevKind, I3cCmd, I3cDeviceId, I3cIbi, I3cIbiType, I3cMsg, I3cPid, I3cStatus,
    I3cXfer, SpeedI2c, SpeedI3c, Tid,
};

// Hardware interface
pub use hardware::{
    Ast1060I3c, HardwareClock, HardwareCore, HardwareFifo, HardwareInterface, HardwareRecovery,
    HardwareTarget, HardwareTransfer, dispatch_i3c_irq, i3c_bus_interrupt,
    register_i3c_irq_handler,
};

// Confined-unsafe MMIO façade (runtime bus selection)
pub use registers::I3cRegisters;

// CCC operations
pub use ccc::{
    Ccc, CccPayload, CccRstActDefByte, CccTargetPayload, GetStatusDefByte, GetStatusFormat,
    GetStatusResp, ccc_events_all_set, ccc_events_set, ccc_getbcr, ccc_getpid, ccc_getstatus,
    ccc_getstatus_fmt1, ccc_rstact_all, ccc_rstdaa_all, ccc_setnewda,
};

// IBI work queue
pub use ibi::{
    IbiConsumer, IbiWork, i3c_ibi_work_enqueue_hotjoin, i3c_ibi_work_enqueue_target_da_assignment,
    i3c_ibi_work_enqueue_target_irq, i3c_ibi_work_enqueue_target_master_write,
    i3c_ibi_workq_consumer,
};

// Constants (wildcard export for convenience)
pub use constants::*;

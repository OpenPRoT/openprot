// Licensed under the Apache-2.0 license

//! AST1060 I3C bare-metal driver core
//!
//! # Overview
//!
//! This module provides a hardware abstraction layer for I3C controllers,
//! supporting both controller (master) and target (slave) modes.
//!
//! # Architecture
//!
//! - [`controller`]: Main I3C controller abstraction
//! - [`config`]: Configuration types and device management
//! - [`types`]: Core data types (commands, messages, transfers)
//! - [`error`]: Error types with trait implementations
//! - [`constants`]: Hardware register definitions
//! - [`hardware`]: Hardware interface
//! - [`ccc`]: Common Command Code operations
//! - [`ibi`]: In-Band Interrupt work queue
//! - [`hal_impl`]: External trait implementations (embedded-hal, proposed-traits)
//!
//! # Features
//!
//! - I3C SDR and HDR modes
//! - Dynamic address assignment (ENTDAA)
//! - In-Band Interrupts (IBI)
//! - Hot-Join support
//! - Target mode operation
//! - Legacy I2C device support
//!
//! # Usage Example
//!
//! ```rust,no_run
//! use aspeed_rust::i3c_refactored::*;
//!
//! // Create controller with hardware implementation
//! let mut ctrl = I3cController::new(hw, I3cConfig::new(), logger);
//! ctrl.init();
//!
//! // Attach a device
//! ctrl.attach_i3c_dev(pid, dynamic_addr, slot)?;
//! ```

// Private module declarations
pub mod ccc;
pub mod config;
pub mod constants;
pub mod controller;
pub mod error;
pub mod hal_impl;
pub mod hardware;
pub mod ibi;
pub mod types;

// =============================================================================
// Public Re-exports
// =============================================================================

// Controller
pub use controller::I3cController;

// Error types
pub use error::{CccErrorKind, I3cError, Result};

// Configuration
pub use config::{
    AddrBook, Attached, CommonCfg, CommonState, DeviceEntry, I3cConfig, I3cTargetConfig, ResetSpec,
    I3C_MAX_CORE_CLK, I3C_MIN_CORE_CLK_HDR, I3C_MIN_CORE_CLK_SDR,
};

// Core types
pub use types::{
    Completion, DevKind, I3cCmd, I3cDeviceId, I3cIbi, I3cIbiType, I3cMsg, I3cPid, I3cStatus,
    I3cXfer, SpeedI2c, SpeedI3c, Tid,
};

// Hardware interface
pub use hardware::{
    dispatch_i3c_irq, register_i3c_irq_handler, HardwareClock, HardwareCore, HardwareFifo,
    HardwareInterface, HardwareRecovery, HardwareTarget, HardwareTransfer,
};

// CCC operations
pub use ccc::{
    ccc_events_all_set, ccc_events_set, ccc_getbcr, ccc_getpid, ccc_getstatus, ccc_getstatus_fmt1,
    ccc_rstact_all, ccc_rstdaa_all, ccc_setnewda, Ccc, CccPayload, CccRstActDefByte,
    CccTargetPayload, GetStatusDefByte, GetStatusFormat, GetStatusResp,
};

// IBI work queue
pub use ibi::{
    i3c_ibi_work_enqueue_hotjoin, i3c_ibi_work_enqueue_target_da_assignment,
    i3c_ibi_work_enqueue_target_irq, i3c_ibi_workq_consumer, IbiWork,
};

// Constants (wildcard export for convenience)
pub use constants::*;

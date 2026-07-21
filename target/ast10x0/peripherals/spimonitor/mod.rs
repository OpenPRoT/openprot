// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 SPI monitor (SPIPF) module.

pub mod commands;
pub mod controller;
pub mod policy;
pub mod profile;
pub mod registers;
pub mod traits;
pub mod types;

pub use commands::{descriptor as command_descriptor, table_value as command_table_value};
pub use controller::{
    Configured, ConfiguredSpiMonitor, Locked, LockedSpiMonitor, SpiMonitor, UninitSpiMonitor,
    Uninitialized,
};
pub use policy::{SpiMonitorPolicy, MAX_CMD_SLOTS, MAX_REGION_SLOTS};
pub use registers::{
    SpiMonitorController, SpiMonitorRegisters, SPIPF1_BASE, SPIPF2_BASE, SPIPF3_BASE, SPIPF4_BASE,
    SPIPF_REG_SIZE,
};
pub use traits::SpiMonitorControl;
pub use types::{
    BootConfig, BootError, BootPhase, BootResult, ExtMuxSel, LockState, MuxSelect, PassthroughMode,
    PrivilegeDirection, PrivilegeOp, RegionPolicy, Result as SpiMonitorResult, SpiMonitorError,
    SpiMonitorId, SpiMonitorState, SpiMonitorStatus, ViolationLogEntry,
};

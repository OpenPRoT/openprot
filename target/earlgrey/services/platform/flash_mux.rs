// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPI Flash MUX IPC opcodes and control client for Earlgrey.

use userspace::time::Instant;
use util_error::ErrorCode;
use util_ipc::{IpcChannel, IpcHandle};
use util_types::Opcode;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// IPC opcode for notice of impending SPI MUX switch (quiesce SPI flash transactions).
pub const IPC_OP_FLASH_SWITCH_MUX_NOTICE: Opcode = Opcode::new(*b"FLXS");

/// IPC opcode for notice of completed SPI MUX switch (re-initialize 4-byte address mode).
pub const IPC_OP_FLASH_SWITCH_MUX_FIN_NOTICE: Opcode = Opcode::new(*b"FLXE");

/// Arguments for the `IPC_OP_FLASH_SWITCH_MUX_FIN_NOTICE` request.
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct SwitchMuxFinOp {
    /// A bitmap indicating which SPI flash devices are accessible.
    /// Bit `i` set to 1 means Flash device `i` is accessible (e.g., bit 0 = 0x1 for Host 0, bit 1 = 0x2 for Host 1).
    pub accessible_flash_bitmap: u8,
}

/// Client for the Flash MUX synchronization and control channel.
pub struct FlashMuxClient {
    ipc: IpcHandle,
}

impl FlashMuxClient {
    /// Creates a new `FlashMuxClient` using the specified IPC handle.
    pub fn new(ipc: IpcHandle) -> Self {
        Self { ipc }
    }

    /// Sends switch_mux_notice to the flash server to quiesce/lock SPI flash access.
    pub fn switch_mux_notice(&self) -> Result<(), ErrorCode> {
        let mut result = 0u32;
        self.ipc
            .transact(
                &[IPC_OP_FLASH_SWITCH_MUX_NOTICE.as_bytes()],
                &mut [result.as_mut_bytes()],
                Instant::MAX,
            )
            .map_err(ErrorCode::kernel_error)?;
        ErrorCode::check_status(result)
    }

    /// Sends switch_mux_fin_notice to the flash server to re-initialize 4-byte address mode and restore SPI flash access.
    pub fn switch_mux_fin_notice(&self, accessible_flash_bitmap: u8) -> Result<(), ErrorCode> {
        let mut result = 0u32;
        let op = SwitchMuxFinOp {
            accessible_flash_bitmap,
        };
        self.ipc
            .transact(
                &[IPC_OP_FLASH_SWITCH_MUX_FIN_NOTICE.as_bytes(), op.as_bytes()],
                &mut [result.as_mut_bytes()],
                Instant::MAX,
            )
            .map_err(ErrorCode::kernel_error)?;
        ErrorCode::check_status(result)
    }
}

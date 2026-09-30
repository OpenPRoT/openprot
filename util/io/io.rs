// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Input/Output traits and utilities.

#![no_std]

mod byte_source;
mod byte_window;
mod random_read;
mod storage;

pub use byte_source::{ByteReadError, ByteSource};
pub use byte_window::ByteWindow;
pub use random_read::{RandomRead, IO_GENERIC, IO_GENERIC_READ_OUT_OF_BOUNDS};
pub use storage::Storage;

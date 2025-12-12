// Licensed under the Apache-2.0 license

//! Mock/Stub Platform Implementation
//!
//! This module provides stub implementations of OpenPRoT platform traits
//! for testing and development purposes when real hardware is not available.

#![no_std]
// Allow security lints for mock/test code
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::arithmetic_side_effects)]

pub mod hash;
pub mod i2c_hardware;
pub mod system_control;

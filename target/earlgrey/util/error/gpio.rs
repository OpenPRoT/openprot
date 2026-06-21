// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! GPIO-specific error codes for Earlgrey.

use openprot_hal_blocking::gpio_port::GpioErrorKind;
use pw_status::Error;
use util_error::{ErrorCode, ErrorModule};

/// The Earlgrey GPIO error module (ASCII `'EI'`).
pub const EG_GPIO: ErrorModule = ErrorModule::new(0x4549);

/// The requested configuration is not supported.
pub const EG_GPIO_INVALID_CONFIGURATION: ErrorCode = EG_GPIO.from_pw(
    GpioErrorKind::UnsupportedConfiguration as u8,
    Error::InvalidArgument,
);

/// Hardware failure during operation.
pub const EG_GPIO_HARDWARE_FAILURE: ErrorCode =
    EG_GPIO.from_pw(GpioErrorKind::HardwareFailure as u8, Error::Internal);

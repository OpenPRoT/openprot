// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! AST10x0 HACE (Hash and Crypto Engine) peripheral support.

mod aes;
mod constants;
mod context;
mod digest;
mod error;
mod device;
mod helpers;
mod hmac;
mod registers;

pub use aes::{AES_BLOCK, AesCipher, AesKey, AesOp, AesSkin, Cbc, Ecb};
pub use digest::HaceDigest;
pub use error::HaceError;
pub use device::{HaceDevice, HashAlgo};
pub use hmac::{HaceHmac, HaceHmacCtx, HmacKey, HMAC_KEY_CAP};
pub use registers::HaceRegisters;

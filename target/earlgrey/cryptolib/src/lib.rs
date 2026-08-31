// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Safe Rust wrapper and FFI bindings for OpenTitan `libotcrypto` on Earlgrey.

#![no_std]

pub mod otcrypto_sys;

pub use otcrypto_sys::{
    hmac_hash_sha256, hmac_hash_sha384, hmac_hash_sha512, AesGcmTagLen, AesKeyMode, AesMode,
    AesOperation, AesPadding, BlindedKey, ByteBuf, ConstByteBuf, ConstWord32Buf, HardenedBool,
    HashDigest, HashMode, HmacKeyMode, KeyConfig, KeyMode, KeySecurityLevel, KeyType, LibVersion,
    RsaPadding, RsaSize, Status, StatusValue, UnblindedKey, Word32Buf,
};

/// Error types for OpenTitan cryptolib operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// Invalid arguments passed to cryptolib.
    BadArgs,
    /// Internal hardware or driver error.
    InternalError,
    /// Fatal error, requires reset or re-initialization.
    FatalError,
    /// Asynchronous operation incomplete.
    AsyncIncomplete,
    /// Requested cryptographic primitive is not implemented.
    NotImplemented,
    /// Unknown status code returned by C library.
    Unknown(i32),
}

impl CryptoError {
    /// Returns a string representation of the error.
    pub const fn as_str(&self) -> &'static str {
        match self {
            CryptoError::BadArgs => "BadArgs",
            CryptoError::InternalError => "InternalError",
            CryptoError::FatalError => "FatalError",
            CryptoError::AsyncIncomplete => "AsyncIncomplete",
            CryptoError::NotImplemented => "NotImplemented",
            CryptoError::Unknown(_) => "Unknown",
        }
    }
}

/// Converts a Status returned by libotcrypto into a Rust Result.
pub fn status_to_result(status: Status) -> Result<(), CryptoError> {
    if status.value == StatusValue::Ok as i32 {
        Ok(())
    } else if status.value == StatusValue::BadArgs as i32 {
        Err(CryptoError::BadArgs)
    } else if status.value == StatusValue::InternalError as i32 {
        Err(CryptoError::InternalError)
    } else if status.value == StatusValue::FatalError as i32 {
        Err(CryptoError::FatalError)
    } else if status.value == StatusValue::AsyncIncomplete as i32 {
        Err(CryptoError::AsyncIncomplete)
    } else if status.value == StatusValue::NotImplemented as i32 {
        Err(CryptoError::NotImplemented)
    } else {
        Err(CryptoError::Unknown(status.value))
    }
}

pub const INIT_INTEGRITY_CHECKSUM: u32 = 0x5a3;

#[inline]
fn compute_buf_checksum(data: *const u8, len: usize) -> u32 {
    INIT_INTEGRITY_CHECKSUM
        .wrapping_add(data as u32)
        .wrapping_add(len as u32)
}

impl ConstByteBuf {
    /// Constructs a `ConstByteBuf` from an immutable byte slice.
    pub fn from_slice(slice: &[u8]) -> Self {
        let data = slice.as_ptr();
        let len = slice.len();
        Self {
            data,
            len,
            ptr_checksum: compute_buf_checksum(data, len),
        }
    }
}

impl ByteBuf {
    /// Constructs a `ByteBuf` from a mutable byte slice.
    pub fn from_mut_slice(slice: &mut [u8]) -> Self {
        let data = slice.as_mut_ptr();
        let len = slice.len();
        Self {
            data,
            len,
            ptr_checksum: compute_buf_checksum(data, len),
        }
    }
}

impl ConstWord32Buf {
    /// Constructs a `ConstWord32Buf` from an immutable 32-bit word slice.
    pub fn from_words(words: &[u32]) -> Self {
        let data = words.as_ptr();
        let len = words.len();
        Self {
            data,
            len,
            ptr_checksum: compute_buf_checksum(data as *const u8, len),
        }
    }
}

impl Word32Buf {
    /// Constructs a `Word32Buf` from a mutable 32-bit word slice.
    pub fn from_mut_words(words: &mut [u32]) -> Self {
        let data = words.as_mut_ptr();
        let len = words.len();
        Self {
            data,
            len,
            ptr_checksum: compute_buf_checksum(data as *const u8, len),
        }
    }
}

impl BlindedKey {
    /// Attaches key material buffer to the blinded key structure.
    pub fn with_key_material(&mut self, km: &[u8]) -> &mut Self {
        self.set_keyblob(km.as_ptr() as *mut u32);
        self
    }
}

impl UnblindedKey {
    /// Attaches key material buffer to the unblinded key structure.
    pub fn with_key_material(&mut self, km: &[u8]) -> &mut Self {
        self.set_key(km.as_ptr() as *mut u32);
        self
    }
}

/// Initializes the cryptolib with the given security level.
pub fn init(security_level: KeySecurityLevel) -> Result<(), CryptoError> {
    let status = unsafe { otcrypto_sys::init(security_level) };
    status_to_result(status)
}

/// Computes SHA-256 digest over the input buffer in a single shot using Earlgrey HMAC HWIP.
pub fn sha256(data: &[u8], digest_out: &mut [u8; 32]) -> Result<(), CryptoError> {
    let buf = ConstByteBuf::from_slice(data);
    let status =
        unsafe { otcrypto_sys::hmac_hash_sha256(&buf, digest_out.as_mut_ptr() as *mut u32) };
    status_to_result(status)
}

/// Computes SHA-384 digest over the input buffer in a single shot using Earlgrey HMAC HWIP.
pub fn sha384(data: &[u8], digest_out: &mut [u8; 48]) -> Result<(), CryptoError> {
    let buf = ConstByteBuf::from_slice(data);
    let status =
        unsafe { otcrypto_sys::hmac_hash_sha384(&buf, digest_out.as_mut_ptr() as *mut u32) };
    status_to_result(status)
}

/// Computes SHA-512 digest over the input buffer in a single shot using Earlgrey HMAC HWIP.
pub fn sha512(data: &[u8], digest_out: &mut [u8; 64]) -> Result<(), CryptoError> {
    let buf = ConstByteBuf::from_slice(data);
    let status =
        unsafe { otcrypto_sys::hmac_hash_sha512(&buf, digest_out.as_mut_ptr() as *mut u32) };
    status_to_result(status)
}

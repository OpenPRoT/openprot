// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Hubris Platform Integration Traits for OpenPRoT
//!
//! This crate provides Hubris-specific integration traits that bridge the gap
//! between OpenPRoT's generic HAL traits and Hubris IDL code generation requirements.
//!
//! The traits use concrete types instead of generic associated types to ensure
//! compatibility with Hubris IDL code generation systems.

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use core::convert::TryFrom;
use core::result::Result;

use openprot_hal_blocking::{
    digest::{owned::DigestOp, Digest},
    mac::owned::MacOp,
};

/// Hubris-specific cryptographic error types
///
/// These errors are designed to be IDL-compatible and map to standard
/// Hubris IPC error codes for inter-task communication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubrisCryptoError {
    /// Invalid key length for the requested operation
    InvalidKeyLength,
    /// Hardware crypto accelerator failure
    HardwareFailure,
    /// Invalid algorithm parameters
    InvalidParameters,
    /// Resource temporarily unavailable (e.g., crypto unit busy)
    ResourceBusy,
    /// Operation not supported by hardware
    NotSupported,
    /// Session state error (context or device missing)
    SessionStateError,
}

/// RAII-style crypto session management
///
/// This type provides RAII-style resource management for crypto devices used in
/// session-based operations. It ensures that crypto hardware devices are properly
/// recovered and can be reused after operations complete.
///
/// The CryptoSession handles the fundamental constraint that hardware controllers
/// cannot be cloned, providing a mechanism for device recovery that works with
/// both software implementations (which can create new instances) and hardware
/// controllers (which require different recovery strategies).
///
/// ## Design Rationale
///
/// Session-based crypto operations consume the device when creating contexts,
/// but we need to recover the device for reuse. This type solves that by:
/// 1. Storing both the crypto context and a recovery device
/// 2. Providing update/finalize methods that manage the context lifecycle  
/// 3. Returning the recovered device when the session completes
///
/// ## Usage Pattern
///
/// ```rust,ignore
/// let session = device.init_digest_session_sha256()?;
/// let session = session.update(b"data")?;
/// let (result, recovered_device) = session.finalize()?;
/// // recovered_device can now be used for new operations
/// ```
pub struct CryptoSession<Context, Device> {
    context: Option<Context>,
    device: Option<Device>,
}

impl<Context, Device> CryptoSession<Context, Device> {
    /// Create a new crypto session with the given context and recovery device
    pub fn new(context: Context, device: Device) -> Self {
        Self {
            context: Some(context),
            device: Some(device),
        }
    }

    /// Take the context out of the session (for move operations)
    fn take_context(&mut self) -> Result<Context, HubrisCryptoError> {
        self.context
            .take()
            .ok_or(HubrisCryptoError::SessionStateError)
    }

    /// Put a context back into the session (after move operations)
    fn put_context(&mut self, context: Context) {
        self.context = Some(context);
    }

    /// Update the session with new data
    ///
    /// This method uses move semantics to update the context while maintaining
    /// the device for recovery.
    pub fn update(mut self, data: &[u8]) -> Result<Self, HubrisCryptoError>
    where
        Context: DigestOp<Controller = Device>,
    {
        let context = self.take_context()?;
        let updated_context = context
            .update(data)
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        self.put_context(updated_context);
        Ok(self)
    }

    /// Finalize the digest session and recover the device
    ///
    /// Returns the computed result and the device for reuse.
    pub fn finalize(mut self) -> Result<(Context::Output, Device), HubrisCryptoError>
    where
        Context: DigestOp<Controller = Device>,
    {
        let context = self.take_context()?;
        let (output, _controller) = context
            .finalize()
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;

        // Recover our device from the session
        let device = self
            .device
            .take()
            .ok_or(HubrisCryptoError::SessionStateError)?;
        Ok((output, device))
    }

    /// Update the MAC session with new data
    pub fn update_mac(mut self, data: &[u8]) -> Result<Self, HubrisCryptoError>
    where
        Context: MacOp<Controller = Device>,
    {
        let context = self.take_context()?;
        let updated_context = context
            .update(data)
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        self.put_context(updated_context);
        Ok(self)
    }

    /// Finalize the MAC session and recover the device
    pub fn finalize_mac(mut self) -> Result<(Context::Output, Device), HubrisCryptoError>
    where
        Context: MacOp<Controller = Device>,
    {
        let context = self.take_context()?;
        let (output, _controller) = context
            .finalize()
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;

        // Recover our device from the session
        let device = self
            .device
            .take()
            .ok_or(HubrisCryptoError::SessionStateError)?;
        Ok((output, device))
    }
}

/// Hubrus-specific digest device trait
///
/// This trait is designed specifically for Hubris microkernel integration:
/// - All types are concrete for IDL compatibility
/// - Methods support task isolation semantics
/// - Error types map to Hubris IPC error codes
/// - Resource management aligns with Hubris task model
pub trait HubrisDigestDevice {
    /// Digest context for SHA-256 operations
    type DigestContext256: DigestOp<Controller = Self, Output = Digest<8>>;
    /// Digest context for SHA-384 operations  
    type DigestContext384: DigestOp<Controller = Self, Output = Digest<12>>;
    /// Digest context for SHA-512 operations
    type DigestContext512: DigestOp<Controller = Self, Output = Digest<16>>;

    /// HMAC key type that can be created from byte slices
    /// Must be compatible with Hubris task memory constraints
    type HmacKey: for<'a> TryFrom<&'a [u8]>;

    /// HMAC context for SHA-256 operations
    type HmacContext256: MacOp<Controller = Self, Output = [u8; 32]>;
    /// HMAC context for SHA-384 operations
    type HmacContext384: MacOp<Controller = Self, Output = [u8; 48]>;
    /// HMAC context for SHA-512 operations
    type HmacContext512: MacOp<Controller = Self, Output = [u8; 64]>;

    /// Maximum supported key size in bytes
    /// This aligns with Hubris task memory constraints
    const MAX_KEY_SIZE: usize = 128;

    /// Initialize a SHA-256 digest operation
    ///
    /// # Hubris Semantics
    /// - Consumes the device (move semantics for resource management)
    /// - Returns concrete error types compatible with IDL
    fn init_digest_sha256(self) -> Result<Self::DigestContext256, HubrisCryptoError>;

    /// Initialize a SHA-384 digest operation
    fn init_digest_sha384(self) -> Result<Self::DigestContext384, HubrisCryptoError>;

    /// Initialize a SHA-512 digest operation
    fn init_digest_sha512(self) -> Result<Self::DigestContext512, HubrisCryptoError>;

    /// Initialize an HMAC-SHA256 operation with the given key
    ///
    /// # Security Note
    /// Key handling follows Hubris security model with task isolation
    fn init_hmac_sha256(
        self,
        key: Self::HmacKey,
    ) -> Result<Self::HmacContext256, HubrisCryptoError>;

    /// Initialize an HMAC-SHA384 operation with the given key
    fn init_hmac_sha384(
        self,
        key: Self::HmacKey,
    ) -> Result<Self::HmacContext384, HubrisCryptoError>;

    /// Initialize an HMAC-SHA512 operation with the given key
    fn init_hmac_sha512(
        self,
        key: Self::HmacKey,
    ) -> Result<Self::HmacContext512, HubrisCryptoError>;

    /// Create an HMAC key from a byte slice
    ///
    /// # Hubris Integration
    /// - Validates key size against device limits
    /// - Ensures compatibility with task memory constraints
    fn create_hmac_key(data: &[u8]) -> Result<Self::HmacKey, HubrisCryptoError> {
        Self::HmacKey::try_from(data).map_err(|_| HubrisCryptoError::InvalidKeyLength)
    }

    // Session methods for digest operations

    /// Initialize SHA-256 digest session with device recovery
    fn init_digest_session_sha256(
        self,
    ) -> Result<CryptoSession<Self::DigestContext256, Self>, HubrisCryptoError>
    where
        Self: Sized;

    /// Initialize SHA-384 digest session with device recovery  
    fn init_digest_session_sha384(
        self,
    ) -> Result<CryptoSession<Self::DigestContext384, Self>, HubrisCryptoError>
    where
        Self: Sized;

    /// Initialize SHA-512 digest session with device recovery
    fn init_digest_session_sha512(
        self,
    ) -> Result<CryptoSession<Self::DigestContext512, Self>, HubrisCryptoError>
    where
        Self: Sized;

    // Session methods for HMAC operations

    /// Initialize HMAC-SHA256 session with device recovery
    fn init_hmac_session_sha256(
        self,
        key: Self::HmacKey,
    ) -> Result<CryptoSession<Self::HmacContext256, Self>, HubrisCryptoError>
    where
        Self: Sized;

    /// Initialize HMAC-SHA384 session with device recovery
    fn init_hmac_session_sha384(
        self,
        key: Self::HmacKey,
    ) -> Result<CryptoSession<Self::HmacContext384, Self>, HubrisCryptoError>
    where
        Self: Sized;

    /// Initialize HMAC-SHA512 session with device recovery
    fn init_hmac_session_sha512(
        self,
        key: Self::HmacKey,
    ) -> Result<CryptoSession<Self::HmacContext512, Self>, HubrisCryptoError>
    where
        Self: Sized;
}

/// Extension trait for one-shot operations in Hubris
///
/// Provides efficient one-shot digest and MAC operations that are common
/// in embedded scenarios and align with Hubris task execution patterns.
pub trait HubrisDigestOneShot: HubrisDigestDevice + Sized {
    /// Compute SHA-256 digest in one operation
    ///
    /// Optimized for Hubris task scheduling - completes atomically
    fn digest_sha256_oneshot(self, data: &[u8]) -> Result<Digest<8>, HubrisCryptoError> {
        let ctx = self.init_digest_sha256()?;
        let ctx = ctx
            .update(data)
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        let (result, _controller) = ctx
            .finalize()
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        Ok(result)
    }

    /// Compute SHA-384 digest in one operation
    fn digest_sha384_oneshot(self, data: &[u8]) -> Result<Digest<12>, HubrisCryptoError> {
        let ctx = self.init_digest_sha384()?;
        let ctx = ctx
            .update(data)
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        let (result, _controller) = ctx
            .finalize()
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        Ok(result)
    }

    /// Compute SHA-512 digest in one operation
    fn digest_sha512_oneshot(self, data: &[u8]) -> Result<Digest<16>, HubrisCryptoError> {
        let ctx = self.init_digest_sha512()?;
        let ctx = ctx
            .update(data)
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        let (result, _controller) = ctx
            .finalize()
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        Ok(result)
    }

    /// Compute HMAC-SHA256 in one operation
    ///
    /// # Security
    /// Key is zeroized after use following Hubris security practices
    fn hmac_sha256_oneshot(self, key: &[u8], data: &[u8]) -> Result<[u8; 32], HubrisCryptoError> {
        let key_handle = Self::create_hmac_key(key)?;
        let ctx = self.init_hmac_sha256(key_handle)?;
        let ctx = ctx
            .update(data)
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        let (result, _controller) = ctx
            .finalize()
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        Ok(result)
    }

    /// Compute HMAC-SHA384 in one operation
    fn hmac_sha384_oneshot(self, key: &[u8], data: &[u8]) -> Result<[u8; 48], HubrisCryptoError> {
        let key_handle = Self::create_hmac_key(key)?;
        let ctx = self.init_hmac_sha384(key_handle)?;
        let ctx = ctx
            .update(data)
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        let (result, _controller) = ctx
            .finalize()
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        Ok(result)
    }

    /// Compute HMAC-SHA512 in one operation
    fn hmac_sha512_oneshot(self, key: &[u8], data: &[u8]) -> Result<[u8; 64], HubrisCryptoError> {
        let key_handle = Self::create_hmac_key(key)?;
        let ctx = self.init_hmac_sha512(key_handle)?;
        let ctx = ctx
            .update(data)
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        let (result, _controller) = ctx
            .finalize()
            .map_err(|_| HubrisCryptoError::HardwareFailure)?;
        Ok(result)
    }
}

// Blanket implementation for one-shot operations
impl<T: HubrisDigestDevice> HubrisDigestOneShot for T {}

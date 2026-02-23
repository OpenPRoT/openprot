// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

/// Error kind for key management operations.
///
/// This represents a common set of key management operation errors that can occur across
/// different implementations. The enum is `#[non_exhaustive]` to allow for future
/// additions without breaking API compatibility.
///
/// Implementations are free to define more specific or additional error types.
/// However, by providing a mapping to these common errors through the [`Error::kind`]
/// method, generic code can still react to them appropriately.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The operation is busy and cannot be completed
    ///
    /// This indicates that the hardware or implementation is currently
    /// busy with another operation. The caller should retry later.
    Busy,

    /// The specified key was not found
    ///
    /// Returned when attempting to access a key that does not exist
    /// in the storage backend.
    KeyNotFound,

    /// Access to the key was denied
    ///
    /// This could indicate insufficient permissions, key usage policy
    /// violations, or other access control restrictions.
    AccessDenied,

    /// Invalid key usage specification
    ///
    /// Returned when the specified key usage constraints are invalid
    /// or incompatible with the key or storage backend.
    InvalidUsage,

    /// Hardware fault or failure
    ///
    /// Indicates a hardware-level error in secure storage elements,
    /// HSMs, or other hardware-backed key storage.
    HardwareFault,

    /// Other implementation-specific error
    ///
    /// Catch-all for errors that don't fit other categories.
    Other,
}

impl core::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Busy => write!(f, "key storage hardware is busy"),
            Self::KeyNotFound => write!(f, "specified key was not found"),
            Self::AccessDenied => write!(f, "access to key was denied"),
            Self::InvalidUsage => write!(f, "invalid key usage specification"),
            Self::HardwareFault => write!(f, "hardware fault during key operation"),
            Self::Other => write!(f, "key management operation failed"),
        }
    }
}
/// Common interface for key management errors.
///
/// This trait provides a standardized way to categorize and handle errors
/// from different key management implementations.
pub trait Error: core::fmt::Debug {
    /// Convert error to a generic error kind
    ///
    /// By using this method, errors freely defined by HAL implementations
    /// can be converted to a set of generic errors upon which generic
    /// code can act.
    fn kind(&self) -> ErrorKind;
}

/// Trait for associating a type with a key management error type.
///
/// This trait is used throughout the key management module to associate operations
/// with their specific error types while maintaining type safety.
pub trait ErrorType {
    /// Error type.
    type Error: Error;
}

/// Configuration and setup operations for key vaults
///
/// This trait provides methods for configuring key vault implementations
/// with runtime parameters and checking their configuration state.
pub trait KeyVaultSetup: ErrorType {
    /// Configuration type for the key vault
    ///
    /// This type contains all the runtime parameters needed to configure
    /// the key vault implementation.
    type KeyVaultConfig;

    /// Configure the key vault with runtime parameters
    ///
    /// This method applies the provided configuration to the key vault,
    /// setting up any necessary resources, security policies, or hardware
    /// initialization required for operation.
    ///
    /// # Parameters
    ///
    /// - `config`: Configuration parameters specific to this key vault implementation
    ///
    /// # Errors
    ///
    /// - `ErrorKind::InvalidUsage`: Invalid configuration parameters
    /// - `ErrorKind::HardwareFault`: Hardware initialization failed
    /// - `ErrorKind::AccessDenied`: Insufficient permissions for configuration
    /// - `ErrorKind::Busy`: Key vault is currently busy and cannot be reconfigured
    fn configure(&mut self, config: Self::KeyVaultConfig) -> Result<(), Self::Error>;

    /// Check if the key vault is properly configured
    ///
    /// Returns `true` if the key vault has been successfully configured
    /// and is ready for key operations, `false` otherwise.
    ///
    /// # Note
    ///
    /// This method is infallible as it only checks internal state.
    /// It does not perform any hardware operations that could fail.
    fn is_configured(&self) -> bool;
}

/// Core key vault operations
pub trait KeyStore: ErrorType {
    /// Type representing a key identifier
    type KeyId: Copy + Clone + PartialEq + Eq;
    /// Type representing key usage permissions
    type KeyUsage: Copy + Clone + PartialEq + Eq;

    /// Erase a specific key
    fn erase_key(&mut self, id: Self::KeyId) -> Result<(), Self::Error>;

    /// Erase all keys that are not locked
    fn erase_all_keys(&mut self) -> Result<(), Self::Error>;

    /// Check if a key exists and is valid
    fn key_exists(&self, id: Self::KeyId) -> Result<bool, Self::Error>;

    /// Get the usage permissions for a key
    fn get_key_usage(&self, id: Self::KeyId) -> Result<Self::KeyUsage, Self::Error>;

    /// Set the usage permissions for a key
    fn set_key_usage(&mut self, id: Self::KeyId, usage: Self::KeyUsage) -> Result<(), Self::Error>;
}

/// Key locking mechanisms for security
pub trait KeyLocking: ErrorType {
    /// Type representing a key identifier
    type KeyId: Copy + Clone + PartialEq + Eq;

    /// Check if key has write lock
    fn is_write_locked(&self, id: Self::KeyId) -> Result<bool, Self::Error>;

    /// Set write lock on key
    fn set_write_lock(&mut self, id: Self::KeyId) -> Result<(), Self::Error>;

    /// Clear write lock on key
    fn clear_write_lock(&mut self, id: Self::KeyId) -> Result<(), Self::Error>;

    /// Check if key has use lock
    fn is_use_locked(&self, id: Self::KeyId) -> Result<bool, Self::Error>;

    /// Set use lock on key
    fn set_use_lock(&mut self, id: Self::KeyId) -> Result<(), Self::Error>;

    /// Clear use lock on key
    fn clear_use_lock(&mut self, id: Self::KeyId) -> Result<(), Self::Error>;
}

/// Key lifecycle management
/// Complete key lifecycle from creation to retrieval
pub trait KeyLifecycle: ErrorType {
    /// Type representing a key identifier
    type KeyId: Copy + Clone + PartialEq + Eq;

    /// Type representing key data
    type KeyData;

    /// Type representing key metadata
    type KeyMetadata;

    /// Store a key with metadata
    fn store_key(
        &mut self,
        id: Self::KeyId,
        data: Self::KeyData,
        metadata: Self::KeyMetadata,
    ) -> Result<(), Self::Error>;

    /// Retrieve key data (if permitted)
    fn retrieve_key(&self, id: Self::KeyId) -> Result<Self::KeyData, Self::Error>;

    /// Get key metadata
    fn get_key_metadata(&self, id: Self::KeyId) -> Result<Self::KeyMetadata, Self::Error>;

    /// Update key metadata
    fn update_key_metadata(
        &mut self,
        id: Self::KeyId,
        metadata: Self::KeyMetadata,
    ) -> Result<(), Self::Error>;
}

/// Blanket trait implementation for types that implement key vsault setup, core operations, and locking
pub trait KeyVault: KeyVaultSetup + KeyStore + KeyLocking {}
impl<T> KeyVault for T where T: KeyVaultSetup + KeyStore + KeyLocking {}

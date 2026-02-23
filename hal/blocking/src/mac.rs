// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use crate::digest::Digest;
use core::fmt::Debug;
use subtle::ConstantTimeEq;
use zerocopy::IntoBytes;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secure wrapper for MAC keys that automatically zeros on drop.
///
/// This wrapper ensures that cryptographic keys are securely erased from memory
/// when no longer needed, preventing key material from remaining in memory.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecureKey<const N: usize> {
    /// The actual key bytes, zeroized on drop
    bytes: [u8; N],
}

impl<const N: usize> SecureKey<N> {
    /// Create a new secure key from a byte array.
    ///
    /// # Security
    /// The input array will be zeroized after copying to prevent key material
    /// from remaining in multiple memory locations.
    pub fn new(mut key_bytes: [u8; N]) -> Self {
        let key = Self { bytes: key_bytes };
        key_bytes.zeroize();
        key
    }

    /// Create a new secure key from a byte slice.
    ///
    /// # Returns
    /// - `Ok(SecureKey)` if the slice length matches the key size
    /// - `Err(ErrorKind::InvalidInputLength)` if the slice is the wrong size
    pub fn from_slice(key_slice: &[u8]) -> Result<Self, ErrorKind> {
        if key_slice.len() != N {
            return Err(ErrorKind::InvalidInputLength);
        }

        let mut key_bytes = [0u8; N];
        key_bytes.copy_from_slice(key_slice);
        Ok(Self::new(key_bytes))
    }

    /// Get a reference to the key bytes.
    ///
    /// # Security
    /// Use this sparingly and ensure the returned reference doesn't outlive
    /// the SecureKey instance.
    pub fn as_bytes(&self) -> &[u8; N] {
        &self.bytes
    }

    /// Verify a MAC tag using constant-time comparison.
    ///
    /// # Security
    /// This function uses constant-time comparison to prevent timing attacks
    /// that could reveal information about the expected MAC value.
    pub fn verify_mac(&self, computed_mac: &[u8], expected_mac: &[u8]) -> bool {
        if computed_mac.len() != expected_mac.len() {
            return false;
        }
        computed_mac.ct_eq(expected_mac).into()
    }
}

impl<const N: usize> Debug for SecureKey<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SecureKey")
            .field("len", &N)
            .field("bytes", &"[REDACTED]")
            .finish()
    }
}

impl<const N: usize> PartialEq for SecureKey<N> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes.ct_eq(&other.bytes).into()
    }
}

impl<const N: usize> Eq for SecureKey<N> {}

impl<const N: usize> AsRef<[u8]> for SecureKey<N> {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl<const N: usize> KeyHandle for SecureKey<N> {}

/// Common error kinds for MAC operations.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The input data length is not valid for the MAC function.
    InvalidInputLength,
    /// The specified MAC algorithm is not supported by the hardware or software implementation.
    UnsupportedAlgorithm,
    /// Failed to allocate memory for the MAC computation.
    MemoryAllocationFailure,
    /// Failed to initialize the MAC computation context.
    InitializationError,
    /// Error occurred while updating the MAC computation with new data.
    UpdateError,
    /// Error occurred while finalizing the MAC computation.
    FinalizationError,
    /// The hardware accelerator is busy and cannot process the MAC computation.
    Busy,
    /// General hardware failure during MAC computation.
    HardwareFailure,
    /// The specified output size is not valid for the MAC function.
    InvalidOutputSize,
    /// Insufficient permissions to access the hardware or perform the MAC computation.
    PermissionDenied,
    /// The MAC computation context has not been initialized.
    NotInitialized,
    /// MAC verification failed - computed MAC does not match expected value.
    VerificationFailed,
}

/// Trait for converting implementation-specific errors into a common error kind.
pub trait Error: Debug {
    /// Returns a generic error kind corresponding to the specific error.
    fn kind(&self) -> ErrorKind;
}

impl Error for core::convert::Infallible {
    fn kind(&self) -> ErrorKind {
        match *self {}
    }
}

/// Trait for types that associate with a specific error type.
pub trait ErrorType {
    /// The associated error type.
    type Error: Error;
}

/// Trait representing a MAC algorithm and its output characteristics.
pub trait MacAlgorithm: Copy + Debug {
    /// The number of bits in the MAC output.
    const OUTPUT_BITS: usize;

    /// The type representing the MAC output.
    type MacOutput: IntoBytes;
}

/// Trait for key handles - implementation determines security model.
///
/// This is a marker trait with no methods, which means there's no way to extract
/// raw key bytes through this interface. Different implementations can provide
/// different security properties:
/// - Software keys may contain raw bytes internally
/// - Hardware keys may be opaque handles to secure storage
/// - TPM keys may be persistent handle references
pub trait KeyHandle {
    // Marker trait - no methods means no way to extract raw bytes
    // Different implementations can have different security properties
}

/// Trait for initializing a MAC operation for a specific algorithm.
///
/// This trait is generic over the key type, allowing different implementations
/// to accept different kinds of keys with varying security properties.
pub trait MacInit<A: MacAlgorithm>: ErrorType {
    /// The key type this implementation accepts.
    ///
    /// Each implementation specifies what kind of keys it can work with:
    /// - Software implementations might use `SecureKey<N>`
    /// - Hardware implementations might use `HardwareKeySlot`
    /// - TPM implementations might use `TpmKeyHandle`
    type Key: KeyHandle;

    /// The type representing the operational context for the MAC.
    type OpContext<'a>: MacOp<Output = A::MacOutput>
    where
        Self: 'a;

    /// Initializes the MAC operation with the specified algorithm and key.
    ///
    /// # Parameters
    ///
    /// - `algo`: A zero-sized type representing the MAC algorithm to use.
    /// - `key`: The key handle. The type is determined by the implementation's
    ///   security model and requirements.
    ///
    /// # Returns
    ///
    /// A result containing the operational context for the MAC, or an error.
    ///
    /// # Security
    ///
    /// The key parameter's type determines the security properties:
    /// - Software keys may expose raw bytes to the implementation
    /// - Hardware keys remain opaque and never expose raw material
    fn init<'a>(&'a mut self, algo: A, key: Self::Key) -> Result<Self::OpContext<'a>, Self::Error>;
}

/// Optional trait for resetting a MAC context to its initial state.
pub trait MacCtrlReset: ErrorType {
    /// Resets the MAC context.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    fn reset(&mut self) -> Result<(), Self::Error>;
}

/// Trait for performing MAC operations.
pub trait MacOp: ErrorType {
    /// The type of the MAC output.
    type Output: IntoBytes;

    /// Updates the MAC state with the provided input data.
    ///
    /// # Parameters
    ///
    /// - `input`: A byte slice containing the data to authenticate.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error>;

    /// Finalizes the MAC computation and returns the result.
    ///
    /// # Returns
    ///
    /// A result containing the MAC output, or an error.
    fn finalize(self) -> Result<Self::Output, Self::Error>;
}

/// Utility function for constant-time MAC verification.
///
/// This function provides a secure way to verify MAC values using constant-time
/// comparison to prevent timing attacks.
///
/// # Parameters
///
/// - `computed_mac`: The computed MAC bytes
/// - `expected_mac`: The expected MAC bytes to verify against
///
/// # Returns
///
/// `true` if the MACs match, `false` otherwise
///
/// # Security
///
/// This function uses constant-time comparison to prevent timing attacks
/// that could reveal information about the expected MAC value.
pub fn verify_mac_constant_time(computed_mac: &[u8], expected_mac: &[u8]) -> bool {
    if computed_mac.len() != expected_mac.len() {
        return false;
    }
    computed_mac.ct_eq(expected_mac).into()
}

// =============================================================================
// MAC Algorithm Marker Types
// =============================================================================

/// HMAC-SHA-256 MAC algorithm marker type.
///
/// This zero-sized type represents the HMAC-SHA-256 message authentication code
/// algorithm, which produces a 256-bit (32-byte) MAC output using SHA-256 as
/// the underlying hash function.
///
/// HMAC-SHA-256 combines the SHA-256 hash function with a secret key to provide
/// both data integrity and authentication.
#[derive(Clone, Copy, Debug)]
pub struct HmacSha2_256;
impl MacAlgorithm for HmacSha2_256 {
    const OUTPUT_BITS: usize = 256;
    type MacOutput = Digest<{ Self::OUTPUT_BITS / 32 }>;
}

/// HMAC-SHA-384 MAC algorithm marker type.
///
/// This zero-sized type represents the HMAC-SHA-384 message authentication code
/// algorithm, which produces a 384-bit (48-byte) MAC output using SHA-384 as
/// the underlying hash function.
///
/// HMAC-SHA-384 provides a larger output size than HMAC-SHA-256 for applications
/// requiring additional security margin.
#[derive(Clone, Copy, Debug)]
pub struct HmacSha2_384;
impl MacAlgorithm for HmacSha2_384 {
    const OUTPUT_BITS: usize = 384;
    type MacOutput = Digest<{ Self::OUTPUT_BITS / 32 }>;
}

/// HMAC-SHA-512 MAC algorithm marker type.
///
/// This zero-sized type represents the HMAC-SHA-512 message authentication code
/// algorithm, which produces a 512-bit (64-byte) MAC output using SHA-512 as
/// the underlying hash function.
///
/// HMAC-SHA-512 provides the largest standard output size, offering maximum
/// collision resistance and authentication strength.
#[derive(Clone, Copy, Debug)]
pub struct HmacSha2_512;
impl MacAlgorithm for HmacSha2_512 {
    const OUTPUT_BITS: usize = 512;
    type MacOutput = Digest<{ Self::OUTPUT_BITS / 32 }>;
}

/// Computes a MAC using a key retrieved from a key vault.
///
/// This function provides integrated MAC computation with secure key storage,
/// ensuring that MAC keys are retrieved from a secure vault and used for
/// authentication operations without exposing the key material.
///
/// # Parameters
/// - `mac_impl`: The MAC implementation to use
/// - `vault`: The key vault containing the MAC key
/// - `key_id`: Unique identifier for the key in the vault
/// - `algorithm`: The MAC algorithm to use (zero-sized type)
/// - `data`: The data to authenticate
///
/// # Returns
/// The computed MAC output
///
/// # Security Notes
/// - MAC key is retrieved securely from vault
/// - Key material is only exposed to the MAC implementation during initialization
/// - Key retrieval and MAC computation are atomic
/// - Supports vault access control and locking mechanisms
/// - Automatic key zeroization after use (vault dependent)
///
/// # Example
/// ```rust,ignore
/// use openprot_hal_blocking::mac::{compute_mac_with_vault, HmacSha2_256, verify_mac_constant_time};
/// use openprot_hal_blocking::key_vault::{KeyLifecycle};
///
/// let mut mac_impl = MyMacImplementation::new();
/// let vault = MyKeyVault::new();
/// let data = b"Hello, world!";
///
/// // Compute MAC with vault-stored key
/// let mac_output = compute_mac_with_vault(
///     &mut mac_impl,
///     &vault,
///     KeyId::new(42),
///     HmacSha2_256,
///     data
/// )?;
///
/// ```
/// Computes a MAC using a key retrieved from a key vault.
///
/// This function provides integrated MAC computation with secure key storage,
/// ensuring that MAC keys are retrieved from a secure vault and used for
/// authentication operations. The security properties depend on the vault
/// and MAC implementation types.
///
/// # Type Parameters
/// - `A`: The MAC algorithm type
/// - `M`: The MAC implementation type
/// - `V`: The key vault type
/// - `E`: The unified error type
///
/// # Parameters
/// - `mac_impl`: The MAC implementation to use
/// - `vault`: The key vault containing the MAC key
/// - `key_id`: Unique identifier for the key in the vault
/// - `algorithm`: The MAC algorithm to use (zero-sized type)
/// - `data`: The data to authenticate
///
/// # Returns
/// The computed MAC output
///
/// # Security Notes
/// - MAC key is retrieved securely from vault
/// - Key exposure depends on implementation types:
///   - Software implementations may see raw key bytes during initialization
///   - Hardware implementations may keep keys opaque throughout
/// - Key retrieval and MAC computation are atomic
/// - Supports vault access control and locking mechanisms
/// - Automatic key zeroization after use (vault dependent)
///
/// # Example
/// ```rust,ignore
/// use openprot_hal_blocking::mac::{compute_mac_with_vault_generic, HmacSha2_256};
/// use openprot_hal_blocking::key_vault::{KeyVault};
///
/// let mut mac_impl = MyMacImplementation::new();
/// let vault = MyKeyVault::new();
/// let data = b"Hello, world!";
///
/// // Compute MAC with vault-stored key
/// let mac_output = compute_mac_with_vault_generic(
///     &mut mac_impl,
///     &vault,
///     KeyId::new(42),
///     HmacSha2_256,
///     data
/// )?;
/// ```
pub fn compute_mac_with_vault_generic<A, M, V, E>(
    mac_impl: &mut M,
    vault: &V,
    key_id: V::KeyId,
    algorithm: A,
    data: &[u8],
) -> Result<A::MacOutput, E>
where
    A: MacAlgorithm,
    M: MacInit<A, Key = V::Key>, // MAC impl must accept vault's key type
    V: KeyVault,
    E: From<M::Error> + From<V::Error>,
    for<'a> <M::OpContext<'a> as ErrorType>::Error: Into<E>,
{
    // Retrieve key from vault
    let key = vault.retrieve_key(key_id).map_err(E::from)?;

    // Initialize MAC operation with key handle
    let mut mac_ctx = mac_impl.init(algorithm, key).map_err(E::from)?;

    // Update with data
    mac_ctx.update(data).map_err(Into::into)?;

    // Finalize and return MAC
    mac_ctx.finalize().map_err(Into::into)
}

/// Trait for key vaults that can provide different key types.
///
/// This trait abstracts over different key storage mechanisms, allowing
/// implementations to provide different security models through the type system.
pub trait KeyVault {
    /// The type used to identify keys in this vault.
    type KeyId;

    /// The type of keys this vault provides.
    ///
    /// This determines the security properties:
    /// - Software vaults might provide `SecureKey<N>`
    /// - Hardware vaults might provide opaque `HardwareKeySlot`
    /// - TPM vaults might provide `TpmKeyHandle`
    type Key: KeyHandle;

    /// The error type for vault operations.
    type Error;

    /// Retrieve a key from the vault.
    ///
    /// # Parameters
    /// - `key_id`: The identifier for the key to retrieve
    ///
    /// # Returns
    /// The key handle, or an error if the key cannot be retrieved
    fn retrieve_key(&self, key_id: Self::KeyId) -> Result<Self::Key, Self::Error>;
}

pub mod scoped {
    //! Scoped MAC API with borrowed contexts (legacy)
    //!
    //! This module provides the traditional scoped MAC API where contexts are borrowed
    //! and have lifetime constraints. This API is suitable for simple embedded applications
    //! and direct hardware mapping.
    //!
    //! This module re-exports the traditional MAC traits that use borrowed contexts
    //! with lifetime constraints. These traits are suitable for embedded applications
    //! where MAC contexts cannot be stored in structs due to lifetime constraints.

    pub use super::{ErrorType, MacAlgorithm, MacCtrlReset, MacInit, MacOp};
}

pub mod owned {
    //! Owned MAC API with move-based resource management
    //!
    //! This module provides a move-based MAC API where contexts are owned
    //! rather than borrowed. This enables:
    //! - Persistent session storage
    //! - Multiple concurrent contexts
    //! - IPC boundary crossing
    //! - Resource recovery patterns
    //! - Compile-time prevention of use-after-finalize
    //!
    //! This API is specifically designed for server applications like Hubris
    //! MAC servers that need to maintain long-lived sessions.

    use super::{ErrorType, IntoBytes, KeyHandle, MacAlgorithm};
    use core::result::Result;

    /// Trait for initializing MAC operations with owned contexts.
    ///
    /// This trait takes ownership of the controller and returns an owned context
    /// that can be stored, moved, and persisted across function boundaries.
    /// Unlike the scoped API, there are no lifetime constraints.
    ///
    /// Like the scoped API, this trait is generic over the key type, allowing
    /// different implementations to accept different kinds of keys with varying
    /// security properties.
    ///
    /// # Type Parameters
    ///
    /// * `A` - The MAC algorithm type that implements [`MacAlgorithm`]
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use openprot_hal_blocking::mac::*;
    /// # use openprot_hal_blocking::mac::owned::{MacInit, MacOp};
    /// # struct MyController;
    /// # impl ErrorType for MyController { type Error = core::convert::Infallible; }
    /// # struct MyContext;
    /// # impl ErrorType for MyContext { type Error = core::convert::Infallible; }
    /// # #[derive(Debug, Clone)]
    /// # struct MyKey([u8; 32]);
    /// # impl KeyHandle for MyKey {}
    /// # impl MacOp for MyContext {
    /// #     type Output = [u8; 32];
    /// #     type Controller = MyController;
    /// #     fn update(self, _: &[u8]) -> Result<Self, Self::Error> { Ok(self) }
    /// #     fn finalize(self) -> Result<(Self::Output, Self::Controller), Self::Error> { todo!() }
    /// #     fn cancel(self) -> Self::Controller { todo!() }
    /// # }
    /// # impl MacInit<HmacSha2_256> for MyController {
    /// #     type Key = MyKey;
    /// #     type Context = MyContext;
    /// #     type Output = [u8; 32];
    /// #     fn init(self, _: HmacSha2_256, _key: MyKey) -> Result<Self::Context, Self::Error> { todo!() }
    /// # }
    /// let controller = MyController;
    /// let key = MyKey([0u8; 32]); // Key handle type determined by implementation
    /// let context = controller.init(HmacSha2_256, key)?;
    /// // Context can be stored in structs, moved across functions, etc.
    /// # Ok::<(), core::convert::Infallible>(())
    /// ```
    pub trait MacInit<A: MacAlgorithm>: ErrorType + Sized {
        /// The key type this implementation accepts.
        ///
        /// Each implementation specifies what kind of keys it can work with:
        /// - Software implementations might use `SecureKey<N>`
        /// - Hardware implementations might use `HardwareKeySlot`
        /// - TPM implementations might use `TpmKeyHandle`
        type Key: KeyHandle;

        /// The owned context type that will handle the MAC computation.
        ///
        /// This context has no lifetime constraints and can be stored in structs,
        /// moved between functions, and persisted across IPC boundaries.
        type Context: MacOp<Output = Self::Output, Controller = Self>;

        /// The output type produced by this MAC implementation.
        ///
        /// This type must implement [`IntoBytes`] to allow conversion to byte arrays
        /// for interoperability with other systems and zero-copy operations.
        type Output: IntoBytes;

        /// Initialize a new MAC computation context.
        ///
        /// Takes ownership of the controller and returns an owned context.
        /// The controller will be returned when the context is finalized or cancelled.
        ///
        /// # Parameters
        ///
        /// - `algorithm`: Algorithm-specific initialization parameters
        /// - `key`: The key handle. The type is determined by the implementation's
        ///   security model and requirements.
        ///
        /// # Returns
        ///
        /// An owned context that can be used for MAC operations.
        ///
        /// # Security
        ///
        /// The key parameter's type determines the security properties:
        /// - Software keys may expose raw bytes to the implementation
        /// - Hardware keys remain opaque and never expose raw material
        fn init(self, algorithm: A, key: Self::Key) -> Result<Self::Context, Self::Error>;
    }

    /// Trait for performing MAC operations with owned contexts.
    ///
    /// This trait uses move semantics where each operation consumes the
    /// context and returns a new context (for `update`) or the final result
    /// with a recovered controller (for `finalize`/`cancel`).
    ///
    /// # Move-based Safety
    ///
    /// The move-based pattern provides compile-time guarantees:
    /// - Cannot use a context after finalization
    /// - Cannot finalize the same context twice
    /// - Controller is always recovered for reuse
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use openprot_hal_blocking::mac::*;
    /// # use openprot_hal_blocking::mac::owned::{MacInit, MacOp};
    /// # struct MyContext;
    /// # impl ErrorType for MyContext { type Error = core::convert::Infallible; }
    /// # struct MyController;
    /// # impl MacOp for MyContext {
    /// #     type Output = [u8; 32];
    /// #     type Controller = MyController;
    /// #     fn update(self, _: &[u8]) -> Result<Self, Self::Error> { Ok(self) }
    /// #     fn finalize(self) -> Result<(Self::Output, Self::Controller), Self::Error> { todo!() }
    /// #     fn cancel(self) -> Self::Controller { todo!() }
    /// # }
    /// # fn get_context() -> MyContext { todo!() }
    /// let context = get_context(); // MyContext
    /// let context = context.update(b"hello")?;
    /// let context = context.update(b" world")?;
    /// let (mac_output, controller) = context.finalize()?;
    /// // Controller recovered for reuse
    /// # Ok::<(), core::convert::Infallible>(())
    /// ```
    pub trait MacOp: ErrorType + Sized {
        /// The MAC output type.
        ///
        /// This type represents the final MAC value produced by [`finalize`](Self::finalize).
        /// It must implement [`IntoBytes`] to enable zero-copy conversion to byte arrays.
        type Output: IntoBytes;

        /// The controller type that will be recovered after finalization or cancellation.
        ///
        /// This enables resource recovery and reuse patterns essential for server applications.
        type Controller;

        /// Update the MAC state with input data.
        ///
        /// This method consumes the current context and returns a new context with
        /// the updated state. This prevents use-after-update bugs at compile time
        /// through move semantics.
        ///
        /// # Parameters
        ///
        /// - `data`: Input data to be authenticated by the MAC algorithm
        ///
        /// # Returns
        ///
        /// A new context with updated state, or an error
        fn update(self, data: &[u8]) -> Result<Self, Self::Error>;

        /// Finalize the MAC computation and recover the controller.
        ///
        /// This method consumes the context and returns both the final MAC output
        /// and the original controller, enabling resource reuse.
        ///
        /// # Returns
        ///
        /// A tuple containing the MAC output and the recovered controller
        fn finalize(self) -> Result<(Self::Output, Self::Controller), Self::Error>;

        /// Cancel the MAC computation and recover the controller.
        ///
        /// This method cancels the current computation and returns the controller
        /// in a clean state, ready for reuse. Unlike `finalize`, this cannot fail.
        ///
        /// # Returns
        ///
        /// The recovered controller in a clean state
        fn cancel(self) -> Self::Controller;
    }
}

/// Computes a MAC using an owned controller and a key retrieved from a key vault.
///
/// This function provides integrated MAC computation with secure key storage using
/// the owned API pattern. It enables server applications to perform MAC operations
/// with vault-stored keys while maintaining resource recovery patterns.
///
/// # Type Parameters
///
/// * `A` - The MAC algorithm type
/// * `C` - The owned MAC controller type
/// * `V` - The key vault type
/// * `E` - The unified error type
///
/// # Parameters
///
/// * `controller` - The owned MAC controller (consumed by this function)
/// * `vault` - Reference to the key vault
/// * `key_id` - Identifier for the key to retrieve from the vault
/// * `algorithm` - The MAC algorithm to use
/// * `data` - The data to authenticate
///
/// # Returns
///
/// A tuple containing the MAC output and the recovered controller, or an error.
///
/// # Examples
///
/// ```rust,ignore
/// use openprot_hal_blocking::mac::{compute_mac_with_vault_owned, HmacSha2_256};
/// use openprot_hal_blocking::mac::owned::{MacInit, MacOp};
/// use openprot_hal_blocking::key_vault::KeyLifecycle;
///
/// let controller = MyOwnedMacController::new();
/// let vault = MyKeyVault::new();
/// let data = b"Hello, world!";
///
/// // Compute MAC with vault-stored key using owned API
/// let (mac_output, recovered_controller) = compute_mac_with_vault_owned(
///     controller,
///     &vault,
///     KeyId::new(42),
///     HmacSha2_256,
///     data
/// )?;
///
/// // Reuse the recovered controller for next operation
/// let (next_mac, _) = compute_mac_with_vault_owned(
///     recovered_controller,
///     &vault,
///     KeyId::new(43),
///     HmacSha2_256,
///     b"Next message"
/// )?;
/// ```
/// Computes a MAC using an owned controller and a key retrieved from a key vault.
///
/// This function provides integrated MAC computation with secure key storage using
/// the owned API pattern. It enables server applications to perform MAC operations
/// with vault-stored keys while maintaining resource recovery patterns.
///
/// # Type Parameters
///
/// * `A` - The MAC algorithm type
/// * `C` - The owned MAC controller type
/// * `V` - The key vault type
/// * `E` - The unified error type
///
/// # Parameters
///
/// * `controller` - The owned MAC controller (consumed by this function)
/// * `vault` - Reference to the key vault
/// * `key_id` - Identifier for the key to retrieve from the vault
/// * `algorithm` - The MAC algorithm to use
/// * `data` - The data to authenticate
///
/// # Returns
///
/// A tuple containing the MAC output and the recovered controller, or an error.
///
/// # Examples
///
/// ```rust,ignore
/// use openprot_hal_blocking::mac::{compute_mac_with_vault_owned_generic, HmacSha2_256};
/// use openprot_hal_blocking::mac::owned::{MacInit, MacOp};
/// use openprot_hal_blocking::mac::KeyVault;
///
/// let controller = MyOwnedMacController::new();
/// let vault = MyKeyVault::new();
/// let data = b"Hello, world!";
///
/// // Compute MAC with vault-stored key using owned API
/// let (mac_output, recovered_controller) = compute_mac_with_vault_owned_generic(
///     controller,
///     &vault,
///     KeyId::new(42),
///     HmacSha2_256,
///     data
/// )?;
///
/// // Reuse the recovered controller for next operation
/// let (next_mac, _) = compute_mac_with_vault_owned_generic(
///     recovered_controller,
///     &vault,
///     KeyId::new(43),
///     HmacSha2_256,
///     b"Next message"
/// )?;
/// ```
pub fn compute_mac_with_vault_owned_generic<A, C, V, E>(
    controller: C,
    vault: &V,
    key_id: V::KeyId,
    algorithm: A,
    data: &[u8],
) -> Result<(A::MacOutput, C), E>
where
    A: MacAlgorithm,
    C: owned::MacInit<A, Key = V::Key, Output = A::MacOutput>,
    C::Context: owned::MacOp<Output = A::MacOutput, Controller = C>,
    V: KeyVault,
    E: From<C::Error> + From<V::Error>,
    E: From<<C::Context as ErrorType>::Error>,
{
    use owned::MacOp; // Bring trait into scope

    // Retrieve key from vault
    let key = vault.retrieve_key(key_id).map_err(E::from)?;

    // Initialize MAC operation with key handle
    let context = controller.init(algorithm, key).map_err(E::from)?;

    // Update with data
    let context = context.update(data).map_err(E::from)?;

    // Finalize and return MAC with recovered controller
    context.finalize().map_err(E::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_algorithm_traits() {
        // Test that MAC algorithm types implement the correct traits
        let _sha256 = HmacSha2_256;
        let _sha384 = HmacSha2_384;
        let _sha512 = HmacSha2_512;

        // Test output sizes (in bits)
        assert_eq!(<HmacSha2_256 as MacAlgorithm>::OUTPUT_BITS, 256);
        assert_eq!(<HmacSha2_384 as MacAlgorithm>::OUTPUT_BITS, 384);
        assert_eq!(<HmacSha2_512 as MacAlgorithm>::OUTPUT_BITS, 512);
    }

    #[test]
    fn test_secure_key_creation() {
        let key_bytes = [0u8; 32];
        let secure_key = SecureKey::new(key_bytes);
        assert_eq!(secure_key.as_bytes().len(), 32);
    }

    #[test]
    fn test_secure_key_from_slice() {
        let key_slice = &[0u8; 32][..];
        let result = SecureKey::<32>::from_slice(key_slice);
        assert!(result.is_ok());

        // Test wrong size
        let wrong_size = &[0u8; 16][..];
        let result = SecureKey::<32>::from_slice(wrong_size);
        assert!(matches!(result, Err(ErrorKind::InvalidInputLength)));
    }

    #[test]
    fn test_secure_key_constant_time_eq() {
        let key1 = SecureKey::new([1u8; 32]);
        let key2 = SecureKey::new([1u8; 32]);
        let key3 = SecureKey::new([2u8; 32]);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_secure_key_verify_mac() {
        let key = SecureKey::new([0u8; 32]);
        let mac1 = [1, 2, 3, 4];
        let mac2 = [1, 2, 3, 4];
        let mac3 = [1, 2, 3, 5];

        assert!(key.verify_mac(&mac1, &mac2));
        assert!(!key.verify_mac(&mac1, &mac3));

        // Different lengths should return false
        let mac_short = [1, 2, 3];
        assert!(!key.verify_mac(&mac1, &mac_short));
    }

    #[test]
    fn test_secure_key_as_ref() {
        let key = SecureKey::new([1, 2, 3, 4, 5, 6, 7, 8]);
        let bytes: &[u8] = key.as_ref();
        assert_eq!(bytes, &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_flexible_key_inputs() {
        // Test that different key types can be used via AsRef<[u8]>

        // Array reference
        let array_key = [0u8; 32];
        let _array_ref: &[u8] = array_key.as_ref();

        // SecureKey
        let secure_key = SecureKey::new([0u8; 32]);
        let _secure_ref: &[u8] = secure_key.as_ref();

        // Slice
        let slice_key: &[u8] = &[0u8; 32];
        let _slice_ref: &[u8] = slice_key;

        // All should be compatible with the new API
        assert_eq!(_array_ref.len(), 32);
        assert_eq!(_secure_ref.len(), 32);
        assert_eq!(_slice_ref.len(), 32);
    }
}

// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # ECDSA Digital Signature Operations
//!
//! This module provides a comprehensive, type-safe abstraction for Elliptic Curve Digital
//! Signature Algorithm (ECDSA) operations. The design follows security best practices and
//! provides a generic interface that can work with any elliptic curve.
//!
//! ## Features
//!
//! - **Type Safety**: Generic over curve types to prevent cross-curve contamination
//! - **Security First**: Mandatory cryptographic RNG, proper key validation, secure memory clearing
//! - **No-std Compatible**: Works in embedded environments without standard library
//! - **Comprehensive Error Handling**: Detailed error types for proper debugging and security
//! - **Optional Serialization**: Flexible serialization support for software keys, with hardware key compatibility
//!
//! ## Architecture
//!
//! The module follows a trait-based design with the following key components:
//!
//! ```text
//! Curve (Abstract EC Parameters)
//! ├── DigestType: DigestAlgorithm
//! └── Scalar: IntoBytes + FromBytes
//!
//! Key Management
//! ├── PrivateKey<C>: Zeroize + Validation
//! ├── SerializablePrivateKey<C>: PrivateKey<C> + Serialization (optional)
//! ├── PublicKey<C>: Coordinate Access + Validation  
//! └── SerializablePublicKey<C>: PublicKey<C> + Serialization (optional)
//!
//! Signatures
//! ├── Signature<C>: Component Access + Validation
//! └── SerializableSignature<C>: Signature<C> + Serialization (optional)
//!
//! Operations
//! ├── EcdsaKeyGen<C>: Key pair generation
//! ├── EcdsaSign<C>: Digital signing with RNG
//! └── EcdsaVerify<C>: Signature verification
//!
//! Error Handling
//! ├── Error: Debug → ErrorKind mapping
//! ├── ErrorType: Associated error types
//! └── ErrorKind: Common error classifications
//!
//! Key Storage Patterns
//! ├── Software Keys: Implement SerializablePrivateKey for persistence
//! ├── Hardware Keys: Implement only PrivateKey for HSM/secure enclave
//! └── Hybrid Systems: Mix both patterns as needed
//! ```
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! # use openprot_hal_blocking::ecdsa::*;
//! # use rand_core::{RngCore, CryptoRng};
//! #
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // This example shows both software and hardware key patterns
//!
//! // SOFTWARE KEYS (with optional serialization):
//! // let mut key_generator = SoftwareKeyGen::new();
//! // let mut rng = YourCryptoRng::new();
//! // let (private_key, public_key) = key_generator.generate_keypair(&mut rng)?;
//!
//! // Software keys can be serialized:
//! // let private_bytes = private_key.as_bytes(); // SerializablePrivateKey
//! // let public_bytes = public_key.as_bytes();   // SerializablePublicKey
//!
//! // HARDWARE KEYS (no serialization):
//! // let mut hsm_generator = HsmKeyGen::new();
//! // let (hsm_private, hsm_public) = hsm_generator.generate_keypair(&mut rng)?;
//!
//! // Hardware keys cannot be serialized (no IntoBytes/FromBytes bounds)
//! // but can still be used for cryptographic operations:
//! // hsm_private.validate()?; // PrivateKey trait method
//!
//! // Both key types work the same for crypto operations:
//! // let mut signer = YourSignerImpl::new();
//! // let message_digest = your_hash_function(message);
//! // let signature = signer.sign(&private_key, &message_digest, &mut rng)?;
//! // let signature_hsm = signer.sign(&hsm_private, &message_digest, &mut rng)?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! ## Security Considerations
//!
//! - **Always validate inputs**: Use the `validate()` methods on keys and signatures
//! - **Use cryptographic RNG**: Only `CryptoRng + RngCore` is accepted for signing
//! - **Clear sensitive data**: Private keys implement `Zeroize` for secure memory clearing
//! - **Hardware key support**: Private keys can be non-serializable for HSM/secure enclave use
//! - **Constant-time operations**: Implementers should use constant-time algorithms where possible
//! - **Side-channel protection**: Be aware of timing attacks in verification operations

use crate::digest::DigestAlgorithm;
use core::fmt::Debug;
use zerocopy::Immutable;
use zerocopy::{FromBytes, IntoBytes};
use zeroize::Zeroize;
/// Trait for converting implementation-specific ECDSA errors into generic error kinds.
///
/// This trait allows HAL implementations to define their own detailed error types
/// while still providing a common interface for generic code to handle errors.
///
/// # Example
///
/// ```rust
/// # use openprot_hal_blocking::ecdsa::{Error, ErrorKind};
/// #[derive(Debug)]
/// enum MyEcdsaError {
///     HardwareFault,
///     InvalidParameters,
///     Timeout,
/// }
///
/// impl Error for MyEcdsaError {
///     fn kind(&self) -> ErrorKind {
///         match self {
///             MyEcdsaError::HardwareFault => ErrorKind::Other,
///             MyEcdsaError::InvalidParameters => ErrorKind::InvalidKeyFormat,
///             MyEcdsaError::Timeout => ErrorKind::Busy,
///         }
///     }
/// }
/// ```
pub trait Error: core::fmt::Debug {
    /// Convert error to a generic error kind
    ///
    /// By using this method, errors freely defined by HAL implementations
    /// can be converted to a set of generic errors upon which generic
    /// code can act.
    fn kind(&self) -> ErrorKind;
}

impl Error for core::convert::Infallible {
    fn kind(&self) -> ErrorKind {
        match *self {}
    }
}

/// Trait for associating a type with an ECDSA error type.
///
/// This trait is used throughout the ECDSA module to associate operations
/// with their specific error types while maintaining type safety.
///
/// # Example
///
/// ```rust
/// # use openprot_hal_blocking::ecdsa::{ErrorType, Error, ErrorKind};
/// # #[derive(Debug)]
/// # enum MyError { Failed }
/// # impl Error for MyError {
/// #     fn kind(&self) -> ErrorKind { ErrorKind::Other }
/// # }
/// struct MyEcdsaDevice;
///
/// impl ErrorType for MyEcdsaDevice {
///     type Error = MyError;
/// }
/// ```
pub trait ErrorType {
    /// Error type.
    type Error: Error;
}

/// Error kind for ECDSA operations.
///
/// This represents a common set of ECDSA operation errors that can occur across
/// different implementations. The enum is `#[non_exhaustive]` to allow for future
/// additions without breaking API compatibility.
///
/// Implementations are free to define more specific or additional error types.
/// However, by providing a mapping to these common errors through the [`Error::kind`]
/// method, generic code can still react to them appropriately.
///
/// # Security Note
///
/// Error types should not leak sensitive information. For example, avoid
/// distinguishing between "key not found" and "wrong key" errors, as this
/// could provide timing attack vectors.
///
/// # Examples
///
/// ```rust
/// # use openprot_hal_blocking::ecdsa::ErrorKind;
/// # let error_kind = ErrorKind::Busy;
/// match error_kind {
///     ErrorKind::InvalidSignature => {
///         // Handle signature verification failure
///         eprintln!("Signature verification failed");
///     }
///     ErrorKind::WeakKey => {
///         // Handle weak key detection
///         eprintln!("Weak key detected - regenerate keypair");
///     }
///     ErrorKind::Busy => {
///         // Handle resource busy - retry later
///         eprintln!("ECDSA hardware is busy, retry later");
///     }
///     _ => {
///         // Handle other errors
///         eprintln!("ECDSA operation failed: {:?}", error_kind);
///     }
/// }
/// ```
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The operation is busy and cannot be completed
    ///
    /// This indicates that the hardware or implementation is currently
    /// busy with another operation. The caller should retry later.
    Busy,

    /// The signature is invalid
    ///
    /// Returned when signature verification fails. This could indicate:
    /// - The signature was corrupted during transmission
    /// - The signature was created with a different key
    /// - The message was modified after signing
    /// - The signature components (r, s) are invalid
    InvalidSignature,

    /// Key generation failed
    ///
    /// Indicates that the key generation process could not complete successfully.
    /// This might be due to insufficient entropy, hardware failures, or other
    /// random number generation issues.
    KeyGenError,

    /// Signing operation failed
    ///
    /// The signing process encountered an error. This is distinct from key
    /// generation errors and typically indicates issues during the actual
    /// signing computation.
    SigningError,

    /// Invalid key format or encoding
    ///
    /// The provided key data could not be parsed or is in an unsupported format.
    /// This includes issues with:
    /// - Incorrect key length
    /// - Invalid encoding (DER, PEM, etc.)
    /// - Malformed key structure
    InvalidKeyFormat,

    /// Point is not on the curve
    ///
    /// The provided coordinates do not represent a valid point on the specified
    /// elliptic curve. This is a critical security check that prevents attacks
    /// using invalid curve points.
    InvalidPoint,

    /// Unsupported curve or algorithm
    ///
    /// The requested elliptic curve or algorithm parameters are not supported
    /// by this implementation. Common reasons include:
    /// - Curve not implemented in hardware
    /// - Disabled curve due to security concerns
    /// - Incompatible curve parameters
    UnsupportedCurve,

    /// Weak key detected (e.g., zero key, key equal to curve order)
    ///
    /// The key fails cryptographic strength requirements. This includes:
    /// - Zero private keys
    /// - Private keys equal to the curve order
    /// - Public keys at the identity point
    /// - Other mathematically weak keys
    WeakKey,

    /// Other unspecified error
    ///
    /// A catch-all for errors that don't fit into the specific categories above.
    /// Implementations should prefer specific error types when possible.
    Other,
}

/// Trait for ECC private keys associated with a specific curve.
///
/// Private keys must implement secure memory clearing through the [`Zeroize`] trait
/// to ensure cryptographic material is properly destroyed when no longer needed.
///
/// # Security Requirements
///
/// Implementations must:
/// - Validate keys are within the valid scalar range (1 < key < curve_order)
/// - Implement constant-time operations where possible
/// - Clear sensitive data from memory using [`Zeroize`]
/// - Hardware keys may store only handles/references, not actual key material
///
/// # Hardware Key Considerations
///
/// For hardware-based keys (HSMs, TPMs, secure enclaves):
/// - The `Zeroize` implementation may clear key handles/references
/// - Actual key material remains securely stored in hardware
/// - Consider calling hardware APIs to invalidate keys during zeroization
/// - Some implementations may use no-op zeroization if hardware manages lifecycle
///
/// # Example
///
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::{PrivateKey, SerializablePrivateKey, Curve, ErrorKind};
/// use zeroize::Zeroize;
/// use zerocopy::{IntoBytes, FromBytes};
///
/// // Software private key (can be serialized)
/// #[derive(IntoBytes, FromBytes)]
/// struct SoftwarePrivateKey([u8; 32]);
///
/// impl<C: Curve> PrivateKey<C> for SoftwarePrivateKey {
///     fn validate(&self) -> Result<(), ErrorKind> {
///         // Check if key is zero or equal to curve order
///         if self.0.iter().all(|&b| b == 0) {
///             return Err(ErrorKind::WeakKey);
///         }
///         Ok(())
///     }
/// }
///
/// // Implement optional serialization
/// impl<C: Curve> SerializablePrivateKey<C> for SoftwarePrivateKey {}
///
/// // Hardware private key (cannot be serialized)
/// struct HardwarePrivateKey {
///     key_handle: u32, // Handle to key in HSM
/// }
///
/// impl<C: Curve> PrivateKey<C> for HardwarePrivateKey {
///     fn validate(&self) -> Result<(), ErrorKind> {
///         // Validate key exists in hardware
///         // (implementation would call into HSM API)
///         Ok(())
///     }
/// }
///
/// // No SerializablePrivateKey implementation for hardware keys
/// ```
pub trait PrivateKey<C: Curve>: Zeroize {
    /// Validate that this private key is valid for the curve.
    ///
    /// This method should verify that the private key is within the valid
    /// range for the curve (typically 1 < key < curve_order).
    ///
    /// # Returns
    /// - `Ok(())`: The private key is valid
    /// - `Err(ErrorKind::WeakKey)`: The key is zero, equal to curve order, or otherwise weak
    /// - `Err(ErrorKind::InvalidKeyFormat)`: The key format is invalid
    fn validate(&self, curve: &C) -> Result<(), ErrorKind>;
}

/// A trait representing an abstract elliptic curve with associated types for cryptographic operations.
///
/// This trait defines the fundamental components required for elliptic curve cryptography,
/// including the digest algorithm for hashing operations and the scalar field element type
/// for curve arithmetic.
///
/// # Associated Types
///
/// - [`DigestType`]: The hash function used for message digests in signing operations
/// - [`Scalar`]: The field element type representing curve coordinates and private keys
///
/// # Example
///
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::{Curve, DigestAlgorithm};
/// use openprot_hal_blocking::digest::Sha256;
///
/// struct P256;
///
/// impl Curve for P256 {
///     type DigestType = Sha256;
///     type Scalar = [u8; 32];
/// }
/// ```
///
/// [`DigestType`]: Curve::DigestType
/// [`Scalar`]: Curve::Scalar
pub trait Curve {
    /// The digest algorithm used by this elliptic curve for cryptographic operations.
    type DigestType: DigestAlgorithm;
    /// The scalar field element type used in elliptic curve operations.
    type Scalar: IntoBytes + FromBytes;
}

/// Trait for ECDSA signatures associated with a specific curve.
///
/// This trait provides access to signature components and validation methods
/// for ECDSA signatures over elliptic curves.
///
/// # Security Considerations
///
/// - Signature components (r, s) must be in the range [1, curve_order)
/// - Zero values for r or s make the signature invalid
/// - Implementations should validate signatures before use
///
/// # Example
///
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::{Signature, Curve, ErrorKind};
///
/// struct MySignature {
///     r: [u8; 32],
///     s: [u8; 32],
/// }
///
/// impl<C: Curve> Signature<C> for MySignature {
///     fn from_coordinates(r: C::Scalar, s: C::Scalar) -> Result<Self, ErrorKind> {
///         // Validate that r and s are in valid range [1, curve_order)
///         if r.iter().all(|&b| b == 0) || s.iter().all(|&b| b == 0) {
///             return Err(ErrorKind::InvalidSignature);
///         }
///         Ok(Self { r, s })
///     }
///
///     fn coordinates(&self, r_out: &mut C::Scalar, s_out: &mut C::Scalar) {
///         // Zero-allocation coordinate access for embedded environments
///         *r_out = self.r;
///         *s_out = self.s;
///     }
/// }
/// ```
pub trait Signature<C: Curve> {
    /// Create a new signature from r and s components with validation.
    ///
    /// This method validates that both r and s are within the valid range
    /// for the curve (typically 1 ≤ r,s < curve_order).
    ///
    /// # Parameters
    /// - `r`: The r component of the signature
    /// - `s`: The s component of the signature
    ///
    /// # Returns
    /// - `Ok(Self)`: Valid signature
    /// - `Err(ErrorKind::InvalidSignature)`: If r or s are invalid (zero or ≥ curve order)
    fn from_coordinates(r: C::Scalar, s: C::Scalar) -> Result<Self, ErrorKind>
    where
        Self: Sized;

    /// Write coordinates to output buffers (zero-allocation for embedded).
    ///
    /// This method provides zero-allocation coordinate access for embedded
    /// environments. Implementations can extract coordinates directly into
    /// the provided buffers without temporary allocation.
    fn coordinates(&self, x_out: &mut C::Scalar, y_out: &mut C::Scalar);
}

/// A trait representing a public key associated with a specific elliptic curve.
/// Trait for ECC public keys associated with a specific curve.
///
/// Public keys represent points on elliptic curves and are used for signature verification.
/// This trait provides coordinate access and validation methods to ensure keys represent
/// valid curve points.
///
/// # Security Considerations
///
/// - Points must lie on the specified elliptic curve
/// - The identity element (point at infinity) is typically invalid for cryptographic use
/// - Always validate public keys before using them for verification
///
/// # Example
///
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::{PublicKey, Curve, ErrorKind};
///
/// struct MyPublicKey {
///     x: [u8; 32],
///     y: [u8; 32],
/// }
///
/// impl<C: Curve> PublicKey<C> for MyPublicKey {
///     fn coordinates(&self, x_out: &mut C::Scalar, y_out: &mut C::Scalar) {
///         // Zero-allocation coordinate access
///         *x_out = self.x;
///         *y_out = self.y;
///     }
///
///     fn from_coordinates(x: C::Scalar, y: C::Scalar) -> Result<Self, ErrorKind> {
///         // Validate point is on curve during construction
///         // (validation logic would go here)
///         Ok(Self { x, y })
///     }
/// }
/// ```
pub trait PublicKey<C: Curve> {
    /// Write coordinates to output buffers (zero-allocation for embedded).
    ///
    /// This method provides zero-allocation coordinate access for embedded
    /// environments. Implementations can extract coordinates directly into
    /// the provided buffers without temporary allocation.
    fn coordinates(&self, x_out: &mut C::Scalar, y_out: &mut C::Scalar);

    /// Create a new public key from x and y coordinates with validation.
    ///
    /// This method validates that the point (x, y) lies on the specified curve
    /// and is cryptographically valid. Validation is performed during construction
    /// to ensure all created public keys are valid.
    ///
    /// # Parameters
    /// - `x`: The x coordinate of the point
    /// - `y`: The y coordinate of the point  
    ///
    /// # Returns
    /// - `Ok(Self)`: Valid public key if the point is on the curve
    /// - `Err(ErrorKind::InvalidPoint)`: If the point is not on the curve
    /// - `Err(ErrorKind::WeakKey)`: If the point is the identity element
    fn from_coordinates(x: C::Scalar, y: C::Scalar) -> Result<Self, ErrorKind>
    where
        Self: Sized;
}

/// Optional serialization support for private keys.
///
/// This trait provides serialization capabilities for private keys that support it,
/// such as software-based keys. Hardware-based keys (HSMs, secure enclaves, etc.)
/// do not need to implement this trait.
///
/// # Security Note
///
/// Implementing this trait exposes private key material as bytes. Only implement
/// this for software keys where serialization is appropriate and secure.
///
/// # Example
///
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::{PrivateKey, SerializablePrivateKey, Curve};
/// use zerocopy::{IntoBytes, FromBytes};
///
/// #[derive(IntoBytes, FromBytes)]
/// struct SoftwarePrivateKey([u8; 32]);
///
/// impl<C: Curve> PrivateKey<C> for SoftwarePrivateKey { /* ... */ }
/// impl<C: Curve> SerializablePrivateKey<C> for SoftwarePrivateKey {}
/// ```
pub trait SerializablePrivateKey<C: Curve>: PrivateKey<C> + IntoBytes + FromBytes {}

/// Optional serialization support for public keys.
///
/// This trait provides serialization capabilities for public keys that support it.
/// Most public key implementations should implement this since public keys are
/// not sensitive and often need to be transmitted or stored.
///
/// # Example
///
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::{PublicKey, SerializablePublicKey, Curve};
/// use zerocopy::{IntoBytes, FromBytes};
///
/// #[derive(IntoBytes, FromBytes)]
/// struct MyPublicKey { /* ... */ }
///
/// impl<C: Curve> PublicKey<C> for MyPublicKey { /* ... */ }
/// impl<C: Curve> SerializablePublicKey<C> for MyPublicKey {}
/// ```
pub trait SerializablePublicKey<C: Curve>: PublicKey<C> + IntoBytes + FromBytes {}

/// Optional serialization support for signatures.
///
/// This trait provides serialization capabilities for signatures that support it.
/// Most signature implementations should implement this since signatures need
/// to be transmitted and verified.
///
/// # Example
///
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::{Signature, SerializableSignature, Curve};
/// use zerocopy::{IntoBytes, FromBytes};
///
/// #[derive(IntoBytes, FromBytes)]
/// struct MySignature { /* ... */ }
///
/// impl<C: Curve> Signature<C> for MySignature { /* ... */ }
/// impl<C: Curve> SerializableSignature<C> for MySignature {}
/// ```
pub trait SerializableSignature<C: Curve>: Signature<C> + IntoBytes + FromBytes {}

/// Trait for ECDSA key generation over a specific elliptic curve.
///
/// This trait enables generation of cryptographically secure ECDSA key pairs
/// using a cryptographic random number generator. Keys can be generated either
/// as standalone objects or stored directly in a key vault for enhanced security.
///
/// # Security Requirements
///
/// - Must use a cryptographically secure random number generator
/// - Generated keys must be uniformly distributed over the valid scalar range
/// - Private keys must be properly zeroized after use
/// - Key vault storage should be preferred for hardware-backed security
///
/// # Example
///
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::{EcdsaKeyGen, Curve, generate_and_store_keypair};
/// use openprot_hal_blocking::key_vault::{KeyLifecycle, KeyStore};
/// use rand_core::{RngCore, CryptoRng};
///
/// struct MyKeyGenerator;
///
/// impl<C: Curve> EcdsaKeyGen<C> for MyKeyGenerator {
///     type PrivateKey = MyPrivateKey;
///     type PublicKey = MyPublicKey;
///
///     fn generate_keypair<R>(&mut self, rng: &mut R)
///         -> Result<(Self::PrivateKey, Self::PublicKey), Self::Error>
///     where
///         R: RngCore + CryptoRng,
///     {
///         // Generate cryptographically secure key pair
///         unimplemented!()
///     }
/// }
///
/// // Usage example with standalone function:
/// let mut key_gen = MyKeyGenerator;
/// let mut vault = MyKeyVault::new();
/// let mut rng = MyRng::new();
///
/// let public_key = generate_and_store_keypair(
///     &mut key_gen,
///     &mut vault,
///     KeyId::new(42),
///     KeyUsage::SIGNING,
///     KeyMetadata::default(),
///     &mut rng
/// )?;
/// ```
pub trait EcdsaKeyGen<C: Curve>: ErrorType {
    /// The type representing the private key for the curve.
    type PrivateKey: PrivateKey<C>;
    /// The type representing the public key for the curve.
    type PublicKey: PublicKey<C>;

    /// Generates an ECDSA key pair.
    ///
    /// # Parameters
    /// - `rng`: A cryptographically secure random number generator.
    ///
    /// # Returns
    /// A tuple containing the generated private key and public key.
    fn generate_keypair<R>(
        &mut self,
        rng: &mut R,
    ) -> Result<(Self::PrivateKey, Self::PublicKey), Self::Error>
    where
        R: rand_core::RngCore + rand_core::CryptoRng;
}

/// Generates an ECDSA key pair and stores the private key in a key vault.
///
/// This function provides integrated key generation and secure storage, ensuring
/// that private keys are immediately stored in a secure vault rather than
/// being exposed in memory. Only the public key is returned to the caller.
///
/// # Parameters
/// - `key_gen`: The key generator implementation
/// - `vault`: The key vault for secure private key storage
/// - `key_id`: Unique identifier for the key in the vault
/// - `usage`: Usage permissions for the stored key
/// - `metadata`: Additional metadata to store with the key
/// - `rng`: A cryptographically secure random number generator
///
/// # Returns
/// The generated public key (private key is securely stored in vault)
///
/// # Security Benefits
/// - Private key is never exposed to caller
/// - Immediate secure storage reduces attack surface
/// - Usage permissions are set atomically with storage
/// - Vault locking mechanisms can be applied immediately
///
/// # Example
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::{generate_and_store_keypair, P256};
/// use openprot_hal_blocking::key_vault::{KeyLifecycle, KeyStore};
///
/// let mut key_gen = MyKeyGenerator::new();
/// let mut vault = MyKeyVault::new();
/// let mut rng = MyRng::new();
///
/// let public_key = generate_and_store_keypair::<P256, _, _, _>(
///     &mut key_gen,
///     &mut vault,
///     KeyId::new(42),
///     KeyUsage::SIGNING,
///     KeyMetadata::default(),
///     &mut rng
/// )?;
/// ```
pub fn generate_and_store_keypair<C, G, V, R>(
    key_gen: &mut G,
    vault: &mut V,
    key_id: <V as crate::key_vault::KeyStore>::KeyId,
    usage: <V as crate::key_vault::KeyStore>::KeyUsage,
    metadata: <V as crate::key_vault::KeyLifecycle>::KeyMetadata,
    rng: &mut R,
) -> Result<G::PublicKey, G::Error>
where
    C: Curve,
    G: EcdsaKeyGen<C>,
    R: rand_core::RngCore + rand_core::CryptoRng,
    V: crate::key_vault::KeyLifecycle<KeyData = G::PrivateKey> + crate::key_vault::KeyStore,
    <V as crate::key_vault::KeyLifecycle>::KeyId: From<<V as crate::key_vault::KeyStore>::KeyId>,
    G::Error: From<V::Error>,
{
    // Generate key pair
    let (private_key, public_key) = key_gen.generate_keypair(rng)?;

    // Store private key in vault with metadata
    let lifecycle_key_id = <V as crate::key_vault::KeyLifecycle>::KeyId::from(key_id);
    vault
        .store_key(lifecycle_key_id, private_key, metadata)
        .map_err(G::Error::from)?;
    vault.set_key_usage(key_id, usage).map_err(G::Error::from)?;

    Ok(public_key)
}

/// Trait for ECDSA signing using a digest algorithm.
///
/// This trait provides ECDSA signature generation from message digests.
/// The digest should be produced by a cryptographically secure hash function
/// that matches the curve's security level.
///
/// # Security Considerations
///
/// - Use cryptographically secure random number generators for nonce generation
/// - Ensure digest length matches the curve's security level
/// - Private keys must be validated before use
///
/// # Example
///
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::{EcdsaSign, Curve, PrivateKey};
/// use rand_core::{RngCore, CryptoRng};
///
/// struct MySigner;
///
/// impl<C: Curve> EcdsaSign<C> for MySigner {
///     type PrivateKey = MyPrivateKey;
///     type Signature = MySignature;
///
///     fn sign<R>(&mut self, key: &Self::PrivateKey, digest: &[u8], rng: &mut R)
///         -> Result<Self::Signature, Self::Error>
///     where
///         R: RngCore + CryptoRng,
///     {
///         // Generate ECDSA signature
///         unimplemented!()
///     }
/// }
/// ```
pub trait EcdsaSign<C: Curve>: ErrorType {
    /// The type representing the private key for the curve.
    type PrivateKey: PrivateKey<C>;
    /// The type representing the signature for the curve.
    type Signature: Signature<C>;

    /// Signs a digest produced by a compatible hash function.
    ///
    /// # Parameters
    /// - `private_key`: The private key used for signing.
    /// - `digest`: The digest output from a hash function.
    /// - `rng`: A cryptographically secure random number generator.
    fn sign<R>(
        &mut self,
        private_key: &Self::PrivateKey,
        digest: <<C as Curve>::DigestType as DigestAlgorithm>::Digest,
        rng: &mut R,
    ) -> Result<Self::Signature, Self::Error>
    where
        R: rand_core::RngCore + rand_core::CryptoRng;
}

/// Trait for ECDSA signature verification using a digest algorithm.
///
/// This trait provides ECDSA signature verification against message digests
/// using public keys. Verification should be performed in constant time
/// where possible to prevent timing attacks.
///
/// # Security Considerations
///
/// - Always validate public keys before verification
/// - Validate signature components are within valid ranges
/// - Use constant-time comparison operations when possible
///
/// # Example
///
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::{EcdsaVerify, Curve, PublicKey};
///
/// struct MyVerifier;
///
/// impl<C: Curve> EcdsaVerify<C> for MyVerifier {
///     type PublicKey = MyPublicKey;
///     type Signature = MySignature;
///
///     fn verify(&mut self, key: &Self::PublicKey, digest: &[u8], signature: &Self::Signature)
///         -> Result<(), Self::Error>
///     {
///         // Verify ECDSA signature
///         unimplemented!()
///     }
/// }
/// ```
pub trait EcdsaVerify<C: Curve>: ErrorType {
    /// The type representing the public key for the curve.
    type PublicKey: PublicKey<C>;
    /// The type representing the signature for the curve.
    type Signature: Signature<C>;

    /// Verifies a signature against a digest.
    ///
    /// # Parameters
    /// - `public_key`: The public key used for verification.
    /// - `digest`: The digest output from a hash function.
    /// - `signature`: The signature to verify.
    fn verify(
        &mut self,
        public_key: &Self::PublicKey,
        digest: <<C as Curve>::DigestType as DigestAlgorithm>::Digest,
        signature: &Self::Signature,
    ) -> Result<(), Self::Error>;
}

/// secp256k1 elliptic curve marker type.
///
/// This zero-sized type represents the secp256k1 elliptic curve, widely used in
/// Bitcoin and other cryptocurrencies. It provides ~128-bit security level.
///
/// ## Parameters
/// - **Field Size**: 256 bits
/// - **Security Level**: ~128 bits
/// - **Standard**: SEC 2
/// - **Common Uses**: Bitcoin, Ethereum, cryptocurrency applications
///
/// ## Example
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::Secp256k1;
///
/// // Type-safe key generation for secp256k1
/// let key_gen = MyKeyGenerator;
/// let (private_key, public_key) = key_gen.generate_keypair(Secp256k1)?;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Secp256k1;

impl Curve for Secp256k1 {
    type DigestType = crate::digest::Sha2_256;
    type Scalar = [u8; 32];
}

/// NIST P-256 elliptic curve marker type.
///
/// This zero-sized type represents the NIST P-256 elliptic curve (secp256r1).
/// This is the most widely used elliptic curve for ECDSA, providing ~128-bit security.
///
/// ## Parameters
/// - **Field Size**: 256 bits
/// - **Security Level**: ~128 bits
/// - **Standard**: FIPS 186-4, RFC 5480
/// - **Common Uses**: TLS certificates, JWT signing, general-purpose cryptography
///
/// ## Example
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::P256;
///
/// // Type-safe key generation for P-256
/// let key_gen = MyKeyGenerator;
/// let (private_key, public_key) = key_gen.generate_keypair(P256)?;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct P256;

impl Curve for P256 {
    type DigestType = crate::digest::Sha2_256;
    type Scalar = [u8; 32];
}

/// NIST P-384 elliptic curve marker type.
///
/// This zero-sized type represents the NIST P-384 elliptic curve (secp384r1).
/// Provides higher security level than P-256 with ~192-bit security.
///
/// ## Parameters
/// - **Field Size**: 384 bits
/// - **Security Level**: ~192 bits
/// - **Standard**: FIPS 186-4, RFC 5480
/// - **Common Uses**: High-security applications, government systems
///
/// ## Example
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::P384;
///
/// // Type-safe key generation for P-384
/// let key_gen = MyKeyGenerator;
/// let (private_key, public_key) = key_gen.generate_keypair(P384)?;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct P384;

impl Curve for P384 {
    type DigestType = crate::digest::Sha2_384;
    type Scalar = [u8; 48];
}

/// A serializable public key for the P384 elliptic curve.
///
/// This implementation provides both coordinate access and serialization
/// capabilities for P384 public keys, supporting the standard 96-byte
/// uncompressed format (48 bytes each for x and y coordinates).
#[derive(Clone, Debug, IntoBytes, FromBytes, Immutable)]
#[repr(C)]
pub struct P384PublicKey {
    /// X coordinate (48 bytes for P384)
    x: [u8; 48],
    /// Y coordinate (48 bytes for P384)  
    y: [u8; 48],
}

impl P384PublicKey {
    /// Create a new P384 public key from raw coordinates
    pub fn new(x: [u8; 48], y: [u8; 48]) -> Self {
        Self { x, y }
    }
}

impl PublicKey<P384> for P384PublicKey {
    fn coordinates(
        &self,
        x_out: &mut <P384 as Curve>::Scalar,
        y_out: &mut <P384 as Curve>::Scalar,
    ) {
        // P384::Scalar is [u8; 48], so we can copy directly
        *x_out = self.x;
        *y_out = self.y;
    }

    fn from_coordinates(
        x: <P384 as Curve>::Scalar,
        y: <P384 as Curve>::Scalar,
    ) -> Result<Self, ErrorKind> {
        Ok(Self::new(x, y))
    }
}

impl SerializablePublicKey<P384> for P384PublicKey {}

/// A serializable signature for the P384 elliptic curve.
///
/// This implementation provides both signature validation and serialization
/// capabilities for P384 ECDSA signatures, supporting the standard 96-byte
/// format (48 bytes each for r and s components).
#[derive(Clone, Debug, IntoBytes, FromBytes, Immutable)]
#[repr(C)]
pub struct P384Signature {
    /// R component (48 bytes for P384)
    r: [u8; 48],
    /// S component (48 bytes for P384)
    s: [u8; 48],
}

impl P384Signature {
    /// Create a new P384 signature from r and s components
    pub fn new(r: [u8; 48], s: [u8; 48]) -> Self {
        Self { r, s }
    }
}

impl Signature<P384> for P384Signature {
    fn from_coordinates(
        r: <P384 as Curve>::Scalar,
        s: <P384 as Curve>::Scalar,
    ) -> Result<Self, ErrorKind> {
        // TODO: Add proper signature validation here
        // For now, we accept any r,s values but in a real implementation
        // we should validate that 1 ≤ r,s < curve_order
        Ok(Self::new(r, s))
    }

    fn coordinates(
        &self,
        r_out: &mut <P384 as Curve>::Scalar,
        s_out: &mut <P384 as Curve>::Scalar,
    ) {
        // P384::Scalar is [u8; 48], so we can copy directly
        *r_out = self.r;
        *s_out = self.s;
    }
}

impl SerializableSignature<P384> for P384Signature {}

/// NIST P-521 elliptic curve marker type.
///
/// This zero-sized type represents the NIST P-521 elliptic curve (secp521r1).
/// Provides maximum security among NIST curves with ~256-bit security.
///
/// ## Parameters
/// - **Field Size**: 521 bits
/// - **Security Level**: ~256 bits
/// - **Standard**: FIPS 186-4, RFC 5480
/// - **Common Uses**: Maximum security applications, long-term archival
///
/// ## Example
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::P521;
///
/// // Type-safe key generation for P-521
/// let key_gen = MyKeyGenerator;
/// let (private_key, public_key) = key_gen.generate_keypair(P521)?;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct P521;

impl Curve for P521 {
    type DigestType = crate::digest::Sha2_512;
    type Scalar = [u8; 66];
}

/// Brainpool P256r1 elliptic curve marker type.
///
/// This zero-sized type represents the Brainpool P256r1 elliptic curve.
/// Brainpool curves are alternative curves to NIST curves with potentially better properties.
///
/// ## Parameters
/// - **Field Size**: 256 bits
/// - **Security Level**: ~128 bits
/// - **Standard**: RFC 5639
/// - **Common Uses**: Alternative to NIST curves, European standards
///
/// ## Example
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::BrainpoolP256r1;
///
/// // Type-safe key generation for Brainpool P256r1
/// let key_gen = MyKeyGenerator;
/// let (private_key, public_key) = key_gen.generate_keypair(BrainpoolP256r1)?;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct BrainpoolP256r1;

impl Curve for BrainpoolP256r1 {
    type DigestType = crate::digest::Sha2_256;
    type Scalar = [u8; 32];
}

/// Brainpool P384r1 elliptic curve marker type.
///
/// This zero-sized type represents the Brainpool P384r1 elliptic curve.
/// Provides higher security level than P256r1 with 192-bit security.
///
/// ## Parameters
/// - **Field Size**: 384 bits
/// - **Security Level**: ~192 bits
/// - **Standard**: RFC 5639
/// - **Common Uses**: High-security applications, European standards
///
/// ## Example
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::BrainpoolP384r1;
///
/// // Type-safe key generation for Brainpool P384r1
/// let key_gen = MyKeyGenerator;
/// let (private_key, public_key) = key_gen.generate_keypair(BrainpoolP384r1)?;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct BrainpoolP384r1;

impl Curve for BrainpoolP384r1 {
    type DigestType = crate::digest::Sha2_384;
    type Scalar = [u8; 48];
}

/// Brainpool P512r1 elliptic curve marker type.
///
/// This zero-sized type represents the Brainpool P512r1 elliptic curve.
/// Provides maximum security among Brainpool curves with ~256-bit security.
///
/// ## Parameters
/// - **Field Size**: 512 bits
/// - **Security Level**: ~256 bits
/// - **Standard**: RFC 5639
/// - **Common Uses**: Maximum security applications, European standards
///
/// ## Example
/// ```rust,ignore
/// use openprot_hal_blocking::ecdsa::BrainpoolP512r1;
///
/// // Type-safe key generation for Brainpool P512r1
/// let key_gen = MyKeyGenerator;
/// let (private_key, public_key) = key_gen.generate_keypair(BrainpoolP512r1)?;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct BrainpoolP512r1;

impl Curve for BrainpoolP512r1 {
    type DigestType = crate::digest::Sha2_512;
    type Scalar = [u8; 64];
}

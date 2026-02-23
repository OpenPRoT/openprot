// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # Digest HAL Traits
//!
//! This module provides blocking/synchronous Hardware Abstraction Layer (HAL) traits
//! for cryptographic digest operations. It defines a common interface for hash functions
//! and message authentication codes that can be implemented by various hardware and
//! software backends.
//!
//! ## API Evolution
//!
//! This module provides two complementary APIs:
//!
//! ### Scoped API (Current)
//! - **Use case**: One-shot operations, simple baremetal applications
//! - **Pattern**: Borrowed contexts with lifetime constraints
//! - **Benefits**: Minimal overhead, direct hardware mapping
//! - **Limitations**: Cannot store contexts, no persistent sessions
//!
//! ### Owned API (New - Move-based Resource Management)
//! - **Use case**: Server applications, persistent sessions, IPC boundaries
//! - **Pattern**: Owned contexts with resource recovery
//! - **Benefits**: Persistent storage, multiple concurrent contexts, IPC-safe
//! - **Limitations**: Slightly more complex ownership model
//!
//! ## Key Components
//!
//! - `Digest` - A generic container for digest output values
//! - `DigestAlgorithm` - Trait defining digest algorithm properties
//!
//! ### Scoped API
//! - `scoped::DigestInit` - Trait for initializing digest operations (borrowed contexts)
//! - `scoped::DigestOp` - Trait for performing digest computations (borrowed contexts)
//! - `scoped::DigestCtrlReset` - Trait for resetting digest contexts
//!
//! ### Owned API (Typestate)
//! - `owned::DigestInit` - Trait for initializing digest operations (owned contexts)
//! - `owned::DigestOp` - Trait for performing digest computations (owned contexts)
//!
//! ## Supported Algorithms
//!
//! This module includes support for:
//! - SHA-2 family: SHA-256, SHA-384, SHA-512
//! - SHA-3 family: SHA3-224, SHA3-256, SHA3-384, SHA3-512
//!
//! ## Example Usage
//!
//! ### Scoped API (Traditional)
//! ```rust,no_run
//! # use openprot_hal_blocking::digest::*;
//! # use openprot_hal_blocking::digest::scoped::*;
//! # struct MyDigestImpl;
//! # impl ErrorType for MyDigestImpl { type Error = core::convert::Infallible; }
//! # impl DigestInit<Sha2_256> for MyDigestImpl {
//! #     type OpContext<'a> = MyContext<'a> where Self: 'a;
//! #     type Output = Digest<8>;
//! #     fn init<'a>(&'a mut self, _: Sha2_256) -> Result<Self::OpContext<'a>, Self::Error> { todo!() }
//! # }
//! # struct MyContext<'a>(&'a mut MyDigestImpl);
//! # impl ErrorType for MyContext<'_> { type Error = core::convert::Infallible; }
//! # impl DigestOp for MyContext<'_> {
//! #     type Output = Digest<8>;
//! #     fn update(&mut self, _: &[u8]) -> Result<(), Self::Error> { Ok(()) }
//! #     fn finalize(self) -> Result<Self::Output, Self::Error> {
//! #         Ok(Digest { value: [0u32; 8] })
//! #     }
//! # }
//! let mut hasher = MyDigestImpl;
//! let mut ctx = hasher.init(Sha2_256)?;
//! ctx.update(b"hello world")?;
//! let digest = ctx.finalize()?;
//! # Ok::<(), core::convert::Infallible>(())
//! ```
//!
//! ### Owned API (Move-based - for servers/sessions)
//! ```rust,no_run
//! # use openprot_hal_blocking::digest::*;
//! # use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};
//! # struct MyDigestController;
//! # impl ErrorType for MyDigestController { type Error = core::convert::Infallible; }
//! # impl DigestInit<Sha2_256> for MyDigestController {
//! #     type Context = MyOwnedContext;
//! #     type Output = Digest<8>;
//! #     fn init(self, _: Sha2_256) -> Result<Self::Context, Self::Error> { todo!() }
//! # }
//! # struct MyOwnedContext;
//! # impl ErrorType for MyOwnedContext { type Error = core::convert::Infallible; }
//! # impl DigestOp for MyOwnedContext {
//! #     type Output = Digest<8>;
//! #     type Controller = MyDigestController;
//! #     fn update(self, _: &[u8]) -> Result<Self, Self::Error> { todo!() }
//! #     fn finalize(self) -> Result<(Self::Output, Self::Controller), Self::Error> { todo!() }
//! #     fn cancel(self) -> Self::Controller { todo!() }
//! # }
//! let controller = MyDigestController;
//! let context = controller.init(Sha2_256)?;
//! let context = context.update(b"hello world")?;
//! let (digest, recovered_controller) = context.finalize()?;
//! // Controller can be reused for new operations
//! # Ok::<(), core::convert::Infallible>(())
//! ```

use core::fmt::Debug;
use core::result::Result;
use zerocopy::{FromBytes, Immutable, IntoBytes};

/// A generic digest output container.
///
/// This structure represents the output of a cryptographic digest operation.
/// It uses a const generic parameter `N` to specify the number of 32-bit words
/// in the digest output, allowing it to accommodate different digest sizes.
///
/// The structure is marked with `#[repr(C)]` to ensure a predictable memory layout,
/// making it suitable for zero-copy operations and hardware interfaces.
///
/// ## Integration Benefits
///
/// The `Digest<N>` type solves several critical integration challenges:
///
/// ### 1. Concrete vs Opaque Types
/// Unlike opaque associated types (`type Output: IntoBytes`), `Digest<N>` provides
/// a **concrete type** that generic code can work with directly:
///
/// ```rust
/// # use openprot_hal_blocking::digest::Digest;
/// // ✅ CONCRETE: We know exactly what this is
/// fn process_digest(digest: Digest<8>) -> [u32; 8] {
///     digest.into_array()  // Safe, direct conversion
/// }
///
/// // ❌ OPAQUE: We don't know what D::Output actually is  
/// // fn process_generic<D>(output: D::Output) -> /* Unknown type */ {
/// //     // Cannot convert to [u32; 8] safely
/// // }
/// ```
///
/// ### 2. Safe Type Conversions
/// Provides safe methods to access underlying data without unsafe code:
///
/// ```rust
/// # use openprot_hal_blocking::digest::Digest;
/// let digest = Digest::<8> { value: [1, 2, 3, 4, 5, 6, 7, 8] };
///
/// // Safe conversions - no unsafe code needed
/// let array: [u32; 8] = digest.into_array();      // Owned conversion
/// let array_ref: &[u32; 8] = digest.as_array();   // Borrowed conversion  
/// let bytes: &[u8] = digest.as_bytes();           // Byte slice access
/// ```
///
/// ### 3. IPC Integration
/// Designed specifically for Hubris IPC leased memory operations:
///
/// ```rust,no_run
/// # use openprot_hal_blocking::digest::Digest;
/// # struct Leased<T, U>(core::marker::PhantomData<(T, U)>);
/// # impl<T, U> Leased<T, U> { fn write(&self, data: U) -> Result<(), ()> { Ok(()) } }
/// # let digest_out: Leased<(), [u32; 8]> = Leased(core::marker::PhantomData);
/// # let digest = Digest::<8> { value: [0; 8] };
/// // Direct write to IPC lease - no conversion needed
/// digest_out.write(digest.into_array())?;
/// # Ok::<(), ()>(())
/// ```
///
/// ### 4. Server Application Support  
/// Enables servers to store and manipulate digest results safely:
///
/// ```rust
/// # use openprot_hal_blocking::digest::Digest;
/// struct DigestCache {
///     sha256_results: Vec<Digest<8>>,   // Can store concrete types
///     sha384_results: Vec<Digest<12>>,  // Different sizes supported
/// }
///
/// impl DigestCache {
///     fn store_sha256(&mut self, digest: Digest<8>) {
///         self.sha256_results.push(digest);  // Direct storage
///     }
///     
///     fn get_as_array(&self, index: usize) -> [u32; 8] {
///         self.sha256_results[index].into_array()  // Safe access
///     }
/// }
/// ```
///
/// ### 5. Zero-Copy Operations
/// Full zerocopy trait support enables efficient memory operations:
///
/// ```rust
/// # use openprot_hal_blocking::digest::Digest;
/// let digest = Digest::<8> { value: [1, 2, 3, 4, 5, 6, 7, 8] };
///
/// // Zero-copy byte access via zerocopy traits
/// let bytes: &[u8] = zerocopy::IntoBytes::as_bytes(&digest);
///
/// // Safe transmutation between compatible layouts
/// // (enabled by FromBytes + Immutable derives)
/// ```
///
/// ## Comparison with Opaque Output Types
///
/// | Feature | `Digest<N>` (Concrete) | `D::Output` (Opaque) |
/// |---------|-------------------------|----------------------|
/// | **Type Known at Compile Time** | ✅ Always `Digest<N>` | ❌ Unknown until runtime |
/// | **Safe Array Access** | ✅ `into_array()`, `as_array()` | ❌ Requires unsafe casting |
/// | **IPC Integration** | ✅ Direct `[u32; N]` conversion | ❌ Complex type bridging |
/// | **Server Storage** | ✅ Can store in structs | ❌ Difficult generic storage |
/// | **Zero-Copy Support** | ✅ Full zerocopy traits | ❌ Implementation dependent |
/// | **Embedded Friendly** | ✅ Known size, no allocation | ❌ Unknown size, complex |
///
/// # Type Parameters
///
/// * `N` - The number of 32-bit words in the digest output
///
/// # Examples
///
/// ```rust
/// # use openprot_hal_blocking::digest::Digest;
/// // A 256-bit digest (8 words of 32 bits each)
/// let sha256_digest = Digest::<8> {
///     value: [0x12345678, 0x9abcdef0, 0x11111111, 0x22222222,
///             0x33333333, 0x44444444, 0x55555555, 0x66666666],
/// };
///
/// // Safe conversion to array for IPC
/// let array = sha256_digest.into_array();
///
/// // Access as bytes for serialization  
/// let bytes = sha256_digest.as_bytes();
/// assert_eq!(bytes.len(), 32);
/// ```
#[derive(Copy, Clone, PartialEq, Eq, IntoBytes, FromBytes, Immutable)]
#[repr(C)]
pub struct Digest<const N: usize> {
    /// The digest value as an array of 32-bit words
    pub value: [u32; N],
}

impl<const N: usize> Digest<N> {
    /// Create a new digest from an array of words
    pub fn new(value: [u32; N]) -> Self {
        Self { value }
    }

    /// Get the digest as an array of words
    ///
    /// This provides safe access to the underlying array without any conversions.
    pub fn into_array(self) -> [u32; N] {
        self.value
    }

    /// Get a reference to the digest as an array of words
    pub fn as_array(&self) -> &[u32; N] {
        &self.value
    }

    /// Get the digest as a byte slice
    pub fn as_bytes(&self) -> &[u8] {
        zerocopy::IntoBytes::as_bytes(self)
    }
}

impl<const N: usize> AsRef<[u8]> for Digest<N> {
    fn as_ref(&self) -> &[u8] {
        zerocopy::IntoBytes::as_bytes(self)
    }
}

/// Trait defining the properties of a cryptographic digest algorithm.
///
/// This trait provides compile-time information about digest algorithms,
/// including their output size and associated digest type. It serves as
/// a type-level specification that can be used with generic digest operations.
///
/// # Requirements
///
/// Implementing types must be `Copy` and `Debug` to support easy cloning
/// and debugging capabilities.
///
/// # Examples
///
/// ```rust
/// # use openprot_hal_blocking::digest::{DigestAlgorithm, Digest};
/// # use core::fmt::Debug;
/// #[derive(Clone, Copy, Debug)]
/// struct MyCustomAlgorithm;
///
/// impl DigestAlgorithm for MyCustomAlgorithm {
///     const OUTPUT_BITS: usize = 256;
///     type Digest = Digest<8>; // 256 bits / 32 bits per word = 8 words
/// }
/// ```
pub trait DigestAlgorithm: Copy + Debug {
    /// The output size of the digest algorithm in bits.
    ///
    /// This constant defines the total number of bits in the digest output.
    /// For example, SHA-256 would have `OUTPUT_BITS = 256`.
    const OUTPUT_BITS: usize;

    /// The digest output type for this algorithm.
    ///
    /// This associated type specifies the concrete digest type that will be
    /// produced by this algorithm. Typically this will be a [`Digest<N>`]
    /// where `N` is calculated from `OUTPUT_BITS`.
    type Digest;
}

/// SHA-256 digest algorithm marker type.
///
/// This zero-sized type represents the SHA-256 cryptographic hash algorithm,
/// which produces a 256-bit (32-byte) digest output.
///
/// SHA-256 is part of the SHA-2 family and is widely used for cryptographic
/// applications requiring strong collision resistance.
#[derive(Clone, Copy, Debug)]
pub struct Sha2_256;
impl DigestAlgorithm for Sha2_256 {
    const OUTPUT_BITS: usize = 256usize;
    type Digest = Digest<{ Self::OUTPUT_BITS / 32 }>;
}

/// SHA-384 digest algorithm marker type.
///
/// This zero-sized type represents the SHA-384 cryptographic hash algorithm,
/// which produces a 384-bit (48-byte) digest output.
///
/// SHA-384 is part of the SHA-2 family and provides a larger output size
/// than SHA-256 for applications requiring additional security margin.
#[derive(Clone, Copy, Debug)]
pub struct Sha2_384;
impl DigestAlgorithm for Sha2_384 {
    const OUTPUT_BITS: usize = 384usize;
    type Digest = Digest<{ Self::OUTPUT_BITS / 32 }>;
}

/// SHA-512 digest algorithm marker type.
///
/// This zero-sized type represents the SHA-512 cryptographic hash algorithm,
/// which produces a 512-bit (64-byte) digest output.
///
/// SHA-512 is part of the SHA-2 family and provides the largest standard
/// output size, offering maximum collision resistance.
#[derive(Clone, Copy, Debug)]
pub struct Sha2_512;
impl DigestAlgorithm for Sha2_512 {
    const OUTPUT_BITS: usize = 512;
    type Digest = Digest<{ Self::OUTPUT_BITS / 32 }>;
}

/// SHA3-224 digest algorithm marker type.
///
/// This zero-sized type represents the SHA3-224 cryptographic hash algorithm,
/// which produces a 224-bit (28-byte) digest output.
///
/// SHA3-224 is part of the SHA-3 (Keccak) family and offers an alternative
/// to SHA-2 with different underlying mathematical foundations.
#[derive(Clone, Copy, Debug)]
pub struct Sha3_224;
impl DigestAlgorithm for Sha3_224 {
    const OUTPUT_BITS: usize = 224usize;
    type Digest = Digest<{ Self::OUTPUT_BITS / 32 }>;
}

/// SHA3-256 digest algorithm marker type.
///
/// This zero-sized type represents the SHA3-256 cryptographic hash algorithm,
/// which produces a 256-bit (32-byte) digest output.
///
/// SHA3-256 is part of the SHA-3 (Keccak) family and provides equivalent
/// security to SHA-256 with different algorithmic properties.
#[derive(Clone, Copy, Debug)]
pub struct Sha3_256;
impl DigestAlgorithm for Sha3_256 {
    const OUTPUT_BITS: usize = 256usize;
    type Digest = Digest<{ Self::OUTPUT_BITS / 32 }>;
}

/// SHA3-384 digest algorithm marker type.
///
/// This zero-sized type represents the SHA3-384 cryptographic hash algorithm,
/// which produces a 384-bit (48-byte) digest output.
///
/// SHA3-384 is part of the SHA-3 (Keccak) family and provides equivalent
/// security to SHA-384 with different algorithmic properties.
#[derive(Clone, Copy, Debug)]
pub struct Sha3_384;
impl DigestAlgorithm for Sha3_384 {
    const OUTPUT_BITS: usize = 384usize;
    type Digest = Digest<{ Self::OUTPUT_BITS / 32 }>;
}

/// SHA3-512 digest algorithm marker type.
///
/// This zero-sized type represents the SHA3-512 cryptographic hash algorithm,
/// which produces a 512-bit (64-byte) digest output.
///
/// SHA3-512 is part of the SHA-3 (Keccak) family and provides equivalent
/// security to SHA-512 with different algorithmic properties.
#[derive(Clone, Copy, Debug)]
pub struct Sha3_512;
impl DigestAlgorithm for Sha3_512 {
    const OUTPUT_BITS: usize = 512;
    type Digest = Digest<{ Self::OUTPUT_BITS / 32 }>;
}

/// Error kind.
///
/// This represents a common set of digest operation errors. Implementations are
/// free to define more specific or additional error types. However, by providing
/// a mapping to these common errors, generic code can still react to them.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The input data length is not valid for the hash function.
    InvalidInputLength,

    /// The specified hash algorithm is not supported by the hardware or software implementation.
    UnsupportedAlgorithm,

    /// Failed to allocate memory for the hash computation.
    MemoryAllocationFailure,

    /// Failed to initialize the hash computation context.
    InitializationError,

    /// Error occurred while updating the hash computation with new data.
    UpdateError,

    /// Error occurred while finalizing the hash computation.
    FinalizationError,

    /// The hardware accelerator is busy and cannot process the hash computation.
    Busy,

    /// General hardware failure during hash computation.
    HardwareFailure,

    /// The specified output size is not valid for the hash function.
    InvalidOutputSize,

    /// Insufficient permissions to access the hardware or perform the hash computation.
    PermissionDenied,

    /// The hash computation context has not been initialized.
    NotInitialized,
}

impl core::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidInputLength => write!(f, "invalid input data length"),
            Self::UnsupportedAlgorithm => write!(f, "unsupported hash algorithm"),
            Self::MemoryAllocationFailure => write!(f, "memory allocation failed"),
            Self::InitializationError => write!(f, "failed to initialize hash computation"),
            Self::UpdateError => write!(f, "error updating hash computation"),
            Self::FinalizationError => write!(f, "error finalizing hash computation"),
            Self::Busy => write!(f, "hardware accelerator is busy"),
            Self::HardwareFailure => write!(f, "hardware failure during hash computation"),
            Self::InvalidOutputSize => write!(f, "invalid output size for hash function"),
            Self::PermissionDenied => write!(f, "insufficient permissions to access hardware"),
            Self::NotInitialized => write!(f, "hash computation context not initialized"),
        }
    }
}

/// Trait for digest operation errors.
///
/// This trait provides a common interface for all error types that can occur
/// during digest operations. It allows for categorization of errors into
/// common types while still permitting implementation-specific error details.
///
/// All digest error types must implement `Debug` for debugging purposes and
/// provide a method to convert to a generic [`ErrorKind`].
///
/// # Examples
///
/// ```rust
/// # use openprot_hal_blocking::digest::{Error, ErrorKind};
/// # use core::fmt::Debug;
/// #[derive(Debug)]
/// struct MyDigestError {
///     message: &'static str,
/// }
///
/// impl Error for MyDigestError {
///     fn kind(&self) -> ErrorKind {
///         ErrorKind::HardwareFailure
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

/// Trait for types that have an associated error type.
///
/// This trait provides a standard way for digest-related types to specify
/// their error type. It's used throughout the digest HAL to maintain
/// type safety while allowing different implementations to use their own
/// specific error types.
///
/// # Examples
///
/// ```rust
/// # use openprot_hal_blocking::digest::{ErrorType, Error, ErrorKind};
/// # use core::fmt::Debug;
/// # #[derive(Debug)]
/// # struct MyError;
/// # impl Error for MyError {
/// #     fn kind(&self) -> ErrorKind { ErrorKind::HardwareFailure }
/// # }
/// struct MyDigestDevice;
///
/// impl ErrorType for MyDigestDevice {
///     type Error = MyError;
/// }
/// ```
pub trait ErrorType {
    /// Error type.
    type Error: Error;
}

/// Trait for initializing digest operations.
///
/// This trait provides the interface for creating new digest computation contexts.
/// It is parameterized by a [`DigestAlgorithm`] type to ensure type safety and
/// allow different algorithms to have different initialization parameters.
///
/// # Type Parameters
///
/// * `T` - The digest algorithm type that implements [`DigestAlgorithm`]
///
/// # Examples
///
/// ```rust,no_run
/// # use openprot_hal_blocking::digest::*;
/// # struct MyDigestImpl;
/// # impl ErrorType for MyDigestImpl { type Error = core::convert::Infallible; }
/// # impl DigestInit<Sha2_256> for MyDigestImpl {
/// #     type OpContext<'a> = MyContext<'a> where Self: 'a;
/// #     type Output = Digest<8>;
/// #     fn init<'a>(&'a mut self, _: Sha2_256) -> Result<Self::OpContext<'a>, Self::Error> { todo!() }
/// # }
/// # struct MyContext<'a>(&'a mut MyDigestImpl);
/// # impl ErrorType for MyContext<'_> { type Error = core::convert::Infallible; }
/// # impl DigestOp for MyContext<'_> {
/// #     type Output = Digest<8>;
/// #     fn update(&mut self, _: &[u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn finalize(self) -> Result<Self::Output, Self::Error> {
/// #         Ok(Digest { value: [0u32; 8] })
/// #     }
/// # }
/// let mut device = MyDigestImpl;
/// let context = device.init(Sha2_256)?;
/// # Ok::<(), core::convert::Infallible>(())
/// ```
pub trait DigestInit<T: DigestAlgorithm>: ErrorType {
    /// The operation context type that will handle the digest computation.
    ///
    /// This associated type represents the stateful context returned by [`init`](Self::init)
    /// that can be used to perform the actual digest operations via [`DigestOp`].
    /// The lifetime parameter ensures the context cannot outlive the device that created it.
    type OpContext<'a>: DigestOp<Output = Self::Output>
    where
        Self: 'a;

    /// The output type produced by this digest implementation.
    ///
    /// This type must implement [`IntoBytes`] to allow conversion to byte arrays
    /// for interoperability with other systems and zero-copy operations.
    type Output: IntoBytes;

    /// Init instance of the crypto function with the given context.
    ///
    /// # Parameters
    ///
    /// - `init_params`: The context or configuration parameters for the crypto function.
    ///
    /// # Returns
    ///
    /// A new instance of the hash function.
    fn init(&mut self, init_params: T) -> Result<Self::OpContext<'_>, Self::Error>;
}

/// Trait for resetting digest computation contexts.
///
/// This trait provides the ability to reset a digest device or context back to
/// its initial state, allowing it to be reused for new digest computations
/// without needing to create a new instance.
///
/// # Examples
///
/// ```rust,no_run
/// # use openprot_hal_blocking::digest::*;
/// # struct MyDigestImpl;
/// # impl ErrorType for MyDigestImpl { type Error = core::convert::Infallible; }
/// # impl DigestCtrlReset for MyDigestImpl {
/// #     fn reset(&mut self) -> Result<(), Self::Error> { Ok(()) }
/// # }
/// let mut device = MyDigestImpl;
/// device.reset()?;
/// # Ok::<(), core::convert::Infallible>(())
/// ```
pub trait DigestCtrlReset: ErrorType {
    /// Reset instance to its initial state.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure. On success, returns `Ok(())`. On failure, returns a `CryptoError`.
    fn reset(&mut self) -> Result<(), Self::Error>;
}

/// Trait for performing digest operations.
///
/// This trait provides the core interface for digest computation operations:
/// updating the digest state with input data and finalizing the computation
/// to produce the digest output.
///
/// This trait is typically implemented by context types returned from
/// [`DigestInit::init`] and represents an active digest computation.
///
/// # State Machine
///
/// Digest operations follow a simple state machine:
/// 1. **Update**: Call [`update`](Self::update) zero or more times with input data
/// 2. **Finalize**: Call [`finalize`](Self::finalize) once to produce the final digest
///
/// After finalization, the context is consumed and cannot be reused.
///
/// # Examples
///
/// ```rust,no_run
/// # use openprot_hal_blocking::digest::*;
/// # struct MyContext;
/// # impl ErrorType for MyContext { type Error = core::convert::Infallible; }
/// # impl DigestOp for MyContext {
/// #     type Output = Digest<8>;
/// #     fn update(&mut self, _: &[u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn finalize(self) -> Result<Self::Output, Self::Error> {
/// #         Ok(Digest { value: [0u32; 8] })
/// #     }
/// # }
/// let mut context = MyContext;
/// context.update(b"hello")?;
/// context.update(b" world")?;
/// let digest = context.finalize()?;
/// # Ok::<(), core::convert::Infallible>(())
/// ```
pub trait DigestOp: ErrorType {
    /// The digest output type.
    ///
    /// This type represents the final digest value produced by [`finalize`](Self::finalize).
    /// It must implement [`IntoBytes`] to enable zero-copy conversion to byte arrays.
    type Output: IntoBytes;

    /// Update state using provided input data.
    ///
    /// # Parameters
    ///
    /// - `input`: The input data to be hashed. This can be any type that implements `AsRef<[u8]>`.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure. On success, returns `Ok(())`. On failure, returns a `CryptoError`.
    fn update(&mut self, input: &[u8]) -> Result<(), Self::Error>;

    /// Finalize the computation and produce the output.
    ///
    /// # Parameters
    ///
    /// - `out`: A mutable slice to store the hash output. The length of the slice must be at least `MAX_OUTPUT_SIZE`.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure. On success, returns `Ok(())`. On failure, returns a `CryptoError`.
    fn finalize(self) -> Result<Self::Output, Self::Error>;
}

pub mod scoped {
    //! Scoped digest API with borrowed contexts (current)
    //!
    //! This module contains the original OpenPRoT HAL digest traits that use
    //! borrowed contexts with lifetime constraints. These traits are ideal for:
    //! - One-shot digest operations
    //! - Simple embedded applications  
    //! - Direct hardware mapping
    //! - Minimal memory overhead
    //!
    //! **Limitation**: Contexts cannot be stored or persist across function boundaries
    //! due to lifetime constraints.

    pub use super::{DigestAlgorithm, DigestCtrlReset, DigestInit, DigestOp, ErrorType};
}

pub mod owned {
    //! Owned digest API with move-based resource management
    //!
    //! This module provides a move-based digest API where contexts are owned
    //! rather than borrowed. This enables:
    //! - Persistent session storage
    //! - Multiple concurrent contexts
    //! - IPC boundary crossing
    //! - Resource recovery patterns
    //! - Compile-time prevention of use-after-finalize
    //!
    //! This API is specifically designed for server applications like Hubris
    //! digest servers that need to maintain long-lived sessions.

    use super::{DigestAlgorithm, ErrorType, IntoBytes};
    use core::result::Result;

    /// Trait for initializing digest operations with owned contexts.
    ///
    /// This trait takes ownership of the controller and returns an owned context
    /// that can be stored, moved, and persisted across function boundaries.
    /// Unlike the scoped API, there are no lifetime constraints.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The digest algorithm type that implements [`DigestAlgorithm`]
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use openprot_hal_blocking::digest::*;
    /// # use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};
    /// # struct MyController;
    /// # impl ErrorType for MyController { type Error = core::convert::Infallible; }
    /// # struct MyContext;
    /// # impl ErrorType for MyContext { type Error = core::convert::Infallible; }
    /// # impl DigestOp for MyContext {
    /// #     type Output = Digest<8>;
    /// #     type Controller = MyController;
    /// #     fn update(self, _: &[u8]) -> Result<Self, Self::Error> { Ok(self) }
    /// #     fn finalize(self) -> Result<(Self::Output, Self::Controller), Self::Error> { todo!() }
    /// #     fn cancel(self) -> Self::Controller { todo!() }
    /// # }
    /// # impl DigestInit<Sha2_256> for MyController {
    /// #     type Context = MyContext;
    /// #     type Output = Digest<8>;
    /// #     fn init(self, _: Sha2_256) -> Result<Self::Context, Self::Error> { todo!() }
    /// # }
    /// let controller = MyController;
    /// let context = controller.init(Sha2_256)?;
    /// // Context can be stored in structs, moved across functions, etc.
    /// # Ok::<(), core::convert::Infallible>(())
    /// ```
    pub trait DigestInit<T: DigestAlgorithm>: ErrorType + Sized {
        /// The owned context type that will handle the digest computation.
        ///
        /// This context has no lifetime constraints and can be stored in structs,
        /// moved between functions, and persisted across IPC boundaries.
        type Context: DigestOp<Output = Self::Output, Controller = Self>;

        /// The output type produced by this digest implementation.
        ///
        /// This type must implement [`IntoBytes`] to allow conversion to byte arrays
        /// for interoperability with other systems and zero-copy operations.
        type Output: IntoBytes;

        /// Initialize a new digest computation context.
        ///
        /// Takes ownership of the controller and returns an owned context.
        /// The controller will be returned when the context is finalized or cancelled.
        ///
        /// # Parameters
        ///
        /// - `init_params`: Algorithm-specific initialization parameters
        ///
        /// # Returns
        ///
        /// An owned context that can be used for digest operations.
        fn init(self, init_params: T) -> Result<Self::Context, Self::Error>;
    }

    /// Trait for performing digest operations with owned contexts.
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
    /// # use openprot_hal_blocking::digest::*;
    /// # use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};
    /// # struct MyContext;
    /// # impl ErrorType for MyContext { type Error = core::convert::Infallible; }
    /// # struct MyController;
    /// # impl DigestOp for MyContext {
    /// #     type Output = Digest<8>;
    /// #     type Controller = MyController;
    /// #     fn update(self, _: &[u8]) -> Result<Self, Self::Error> { Ok(self) }
    /// #     fn finalize(self) -> Result<(Self::Output, Self::Controller), Self::Error> { todo!() }
    /// #     fn cancel(self) -> Self::Controller { todo!() }
    /// # }
    /// # fn get_context() -> MyContext { todo!() }
    /// let context = get_context(); // MyContext
    /// let context = context.update(b"hello")?;
    /// let context = context.update(b" world")?;
    /// let (digest, controller) = context.finalize()?;
    /// // Controller recovered for reuse
    /// # Ok::<(), core::convert::Infallible>(())
    /// ```
    pub trait DigestOp: ErrorType + Sized {
        /// The digest output type.
        ///
        /// This type represents the final digest value produced by [`finalize`](Self::finalize).
        /// It must implement [`IntoBytes`] to enable zero-copy conversion to byte arrays.
        type Output: IntoBytes;

        /// The controller type that will be recovered after finalization or cancellation.
        ///
        /// This enables resource recovery and reuse patterns essential for server applications.
        type Controller;

        /// Update the digest state with input data.
        ///
        /// This method consumes the current context and returns a new context with
        /// the updated state. This prevents use-after-update bugs at compile time
        /// through move semantics.
        ///
        /// # Parameters
        ///
        /// - `data`: Input data to be processed by the digest algorithm
        ///
        /// # Returns
        ///
        /// A new context with updated state, or an error
        fn update(self, data: &[u8]) -> Result<Self, Self::Error>;

        /// Finalize the digest computation and recover the controller.
        ///
        /// This method consumes the context and returns both the final digest output
        /// and the original controller, enabling resource reuse.
        ///
        /// # Returns
        ///
        /// A tuple containing the digest output and the recovered controller
        fn finalize(self) -> Result<(Self::Output, Self::Controller), Self::Error>;

        /// Cancel the digest computation and recover the controller.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digest_output_conversions() {
        // Test safe conversion methods on Digest type
        let sha256_digest = Digest::<8> {
            value: [1, 2, 3, 4, 5, 6, 7, 8],
        };

        // Test into_array() method
        let array = sha256_digest.into_array();
        assert_eq!(array, [1, 2, 3, 4, 5, 6, 7, 8]);

        // Test as_array() method
        let sha256_digest = Digest::<8> {
            value: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        let array_ref = sha256_digest.as_array();
        assert_eq!(array_ref, &[1, 2, 3, 4, 5, 6, 7, 8]);

        // Test as_bytes() method
        let bytes = sha256_digest.as_bytes();
        assert_eq!(bytes.len(), 32); // 8 words * 4 bytes each

        // Verify the bytes match the expected layout (little endian)
        let expected_bytes = [
            1, 0, 0, 0, // word 1
            2, 0, 0, 0, // word 2
            3, 0, 0, 0, // word 3
            4, 0, 0, 0, // word 4
            5, 0, 0, 0, // word 5
            6, 0, 0, 0, // word 6
            7, 0, 0, 0, // word 7
            8, 0, 0, 0, // word 8
        ];
        assert_eq!(bytes, &expected_bytes);
    }

    #[test]
    fn test_output_type_sizes() {
        use core::mem;

        // Verify that digest output types have correct sizes for IPC
        assert_eq!(mem::size_of::<Digest<8>>(), 32); // SHA-256: 8 words * 4 bytes
        assert_eq!(mem::size_of::<Digest<12>>(), 48); // SHA-384: 12 words * 4 bytes
        assert_eq!(mem::size_of::<Digest<16>>(), 64); // SHA-512: 16 words * 4 bytes

        // Test alignment requirements
        assert_eq!(mem::align_of::<Digest<8>>(), 4); // Aligned to u32
    }

    #[test]
    fn test_digest_new_constructor() {
        let array = [
            0x12345678, 0x9abcdef0, 0x11111111, 0x22222222, 0x33333333, 0x44444444, 0x55555555,
            0x66666666,
        ];
        let digest = Digest::new(array);
        assert_eq!(digest.value, array);
        assert_eq!(digest.into_array(), array);
    }
}

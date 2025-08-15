// Licensed under the Apache-2.0 license

use crate::digest::Digest;
use core::fmt::Debug;
use zerocopy::IntoBytes;

/// Common error kinds for MAC operations (reused from digest operations).
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

    /// The type representing the key used for MAC computation.
    type Key;
}

/// Trait for initializing a MAC operation for a specific algorithm.
pub trait MacInit<A: MacAlgorithm>: ErrorType {
    /// The type representing the operational context for the MAC.
    type OpContext<'a>: MacOp<Output = A::MacOutput>
    where
        Self: 'a;

    /// Initializes the MAC operation with the specified algorithm and key.
    ///
    /// # Parameters
    ///
    /// - `algo`: A zero-sized type representing the MAC algorithm to use.
    /// - `key`: A reference to the key used for the MAC computation.
    ///
    /// # Returns
    ///
    /// A result containing the operational context for the MAC, or an error.
    fn init<'a>(&'a mut self, algo: A, key: &A::Key) -> Result<Self::OpContext<'a>, Self::Error>;
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
    type Key = [u8; 32];
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
    type Key = [u8; 48];
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
    type Key = [u8; 64];
}

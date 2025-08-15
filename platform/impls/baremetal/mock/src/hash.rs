// Licensed under the Apache-2.0 license

//! Mock Hash/Digest Accelerator Implementation
//!
//! Provides a stub implementation of digest operations that can be used
//! for testing when real hardware acceleration is not available.

use openprot_hal_blocking::digest::{
    DigestAlgorithm, DigestCtrlReset, DigestInit, DigestOp, Error, ErrorKind, ErrorType, Sha2_256,
    Sha2_384, Sha2_512,
};

/// Mock digest accelerator device
///
/// This is a software-only stub implementation of the digest hardware traits.
/// It provides working digest operations using simple algorithms or dummy outputs
/// for testing purposes.
pub struct MockDigestDevice;

impl Default for MockDigestDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl MockDigestDevice {
    /// Create a new mock digest device
    pub fn new() -> Self {
        Self
    }
}

/// Mock digest error type
#[derive(Debug, Clone, Copy)]
pub struct MockDigestError;

impl Error for MockDigestError {
    fn kind(&self) -> ErrorKind {
        // Mock implementation never fails, but we can simulate errors if needed
        ErrorKind::HardwareFailure
    }
}

impl ErrorType for MockDigestDevice {
    type Error = MockDigestError;
}

impl DigestCtrlReset for MockDigestDevice {
    fn reset(&mut self) -> Result<(), Self::Error> {
        // Mock reset always succeeds
        Ok(())
    }
}

/// Mock hasher context that tracks the algorithm type and lifetime of the device.
/// This mimics the pattern from the reference implementation where the hasher holds a reference
/// to the hardware device (even though we don't use it in the mock)
/// and the algorithm parameters for type safety.
pub struct MockHasher<'a, T> {
    #[allow(dead_code)] // Mock implementation doesn't need to use the device reference
    hw: &'a mut MockDigestDevice,
    _alg: T,
    data_processed: u64,
}

impl<T> ErrorType for MockHasher<'_, T> {
    type Error = MockDigestError;
}

/// Macro to implement digest traits for each algorithm, following the reference pattern
macro_rules! impl_sha2 {
    ($algo:ident) => {
        impl DigestInit<$algo> for MockDigestDevice {
            type OpContext<'a> = MockHasher<'a, $algo>;
            type Output = <$algo as DigestAlgorithm>::Digest;

            fn init(&mut self, init_params: $algo) -> Result<Self::OpContext<'_>, Self::Error> {
                // In a real implementation, we'd configure the hardware here
                Ok(Self::OpContext {
                    hw: self,
                    _alg: init_params,
                    data_processed: 0,
                })
            }
        }

        impl DigestOp for MockHasher<'_, $algo> {
            type Output = <$algo as DigestAlgorithm>::Digest;

            fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
                // Track the amount of data processed
                self.data_processed += input.len() as u64;
                // In a real implementation, we'd process the input data
                Ok(())
            }

            fn finalize(self) -> Result<Self::Output, Self::Error> {
                // Generate a deterministic but fake digest based on the data length and algorithm
                const OUTPUT_WORDS: usize = <$algo as DigestAlgorithm>::OUTPUT_BITS / 32;
                let mut value = [0u32; OUTPUT_WORDS];
                for (i, word) in value.iter_mut().enumerate() {
                    *word = 0x12345678u32
                        .wrapping_add(self.data_processed as u32)
                        .wrapping_add(i as u32);
                }
                Ok(Self::Output { value })
            }
        }
    };
}

impl_sha2!(Sha2_256);
impl_sha2!(Sha2_384);
impl_sha2!(Sha2_512);

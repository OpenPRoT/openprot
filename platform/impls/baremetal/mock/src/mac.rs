// Licensed under the Apache-2.0 license

//! Mock MAC (Message Authentication Code) Implementation
//!
//! Provides a stub implementation of MAC operations that can be used
//! for testing when real hardware acceleration is not available.

use openprot_hal_blocking::mac::{
    Error, ErrorKind, ErrorType, HmacSha2_256, HmacSha2_384, HmacSha2_512, MacAlgorithm,
    MacCtrlReset, MacInit, MacOp,
};

/// Mock MAC accelerator device
///
/// This is a software-only stub implementation of the MAC hardware traits.
/// It provides working MAC operations using simple algorithms or dummy outputs
/// for testing purposes.
pub struct MockMacDevice;

impl Default for MockMacDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMacDevice {
    /// Create a new mock MAC device
    pub fn new() -> Self {
        Self
    }
}

/// Mock MAC error type
#[derive(Debug, Clone, Copy)]
pub struct MockMacError;

impl Error for MockMacError {
    fn kind(&self) -> ErrorKind {
        // Mock implementation never fails, but we can simulate errors if needed
        ErrorKind::HardwareFailure
    }
}

impl ErrorType for MockMacDevice {
    type Error = MockMacError;
}

impl MacCtrlReset for MockMacDevice {
    fn reset(&mut self) -> Result<(), Self::Error> {
        // Mock reset always succeeds
        Ok(())
    }
}

/// Mock MAC context that tracks the algorithm type, key, and lifetime of the device.
/// This mimics the pattern from the reference implementation where the MAC context holds a reference
/// to the hardware device and the algorithm parameters for type safety.
pub struct MockMac<'a, A> {
    #[allow(dead_code)] // Mock implementation doesn't need to use the device reference
    hw: &'a mut MockMacDevice,
    _alg: A,
    #[allow(dead_code)] // Key is stored but not used in mock implementation
    key_hash: u64, // Simple hash of the key for deterministic output
    data_processed: u64,
}

impl<A> ErrorType for MockMac<'_, A> {
    type Error = MockMacError;
}

/// Helper function to create a simple hash of a byte slice for deterministic mock output
fn simple_hash(data: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64; // FNV offset basis
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    hash
}

/// Macro to implement MAC traits for each algorithm, following the reference pattern
macro_rules! impl_hmac {
    ($algo:ident) => {
        impl MacInit<$algo> for MockMacDevice {
            type OpContext<'a> = MockMac<'a, $algo>;

            fn init<'a>(
                &'a mut self,
                init_params: $algo,
                key: &<$algo as MacAlgorithm>::Key,
            ) -> Result<Self::OpContext<'a>, Self::Error> {
                // In a real implementation, we'd configure the hardware here
                let key_bytes: &[u8] = unsafe {
                    core::slice::from_raw_parts(
                        key.as_ptr() as *const u8,
                        core::mem::size_of_val(key),
                    )
                };
                Ok(Self::OpContext {
                    hw: self,
                    _alg: init_params,
                    key_hash: simple_hash(key_bytes),
                    data_processed: 0,
                })
            }
        }

        impl MacOp for MockMac<'_, $algo> {
            type Output = <$algo as MacAlgorithm>::MacOutput;

            fn update(&mut self, input: &[u8]) -> Result<(), Self::Error> {
                // Track the amount of data processed
                self.data_processed += input.len() as u64;
                // In a real implementation, we'd process the input data with the key
                Ok(())
            }

            fn finalize(self) -> Result<Self::Output, Self::Error> {
                // Generate a deterministic but fake MAC based on the data length, key hash, and algorithm
                const OUTPUT_WORDS: usize = <$algo as MacAlgorithm>::OUTPUT_BITS / 32;
                let mut value = [0u32; OUTPUT_WORDS];
                for (i, word) in value.iter_mut().enumerate() {
                    *word = 0x12345678u32
                        .wrapping_add(self.data_processed as u32)
                        .wrapping_add(self.key_hash as u32)
                        .wrapping_add((self.key_hash >> 32) as u32)
                        .wrapping_add(i as u32);
                }
                Ok(Self::Output { value })
            }
        }
    };
}

impl_hmac!(HmacSha2_256);
impl_hmac!(HmacSha2_384);
impl_hmac!(HmacSha2_512);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_mac_device_creation() {
        let device = MockMacDevice::new();
        let device_default = MockMacDevice::default();
        // Both should be valid instances
        assert_eq!(
            core::mem::size_of_val(&device),
            core::mem::size_of_val(&device_default)
        );
    }

    #[test]
    fn test_mock_mac_reset() {
        let mut device = MockMacDevice::new();
        assert!(device.reset().is_ok());
    }

    #[test]
    fn test_hmac_sha256_mock() {
        let mut device = MockMacDevice::new();
        let key = [0u8; 32];

        let mut mac_ctx = device
            .init(HmacSha2_256, &key)
            .expect("Failed to initialize MAC");
        mac_ctx.update(b"hello").expect("Failed to update MAC");
        mac_ctx.update(b" world").expect("Failed to update MAC");
        let result = mac_ctx.finalize().expect("Failed to finalize MAC");

        // The result should be deterministic for the same input and key
        assert_eq!(result.value.len(), 8); // 256 bits / 32 bits per word = 8 words
    }

    #[test]
    fn test_hmac_different_keys_different_output() {
        let mut device1 = MockMacDevice::new();
        let mut device2 = MockMacDevice::new();

        let key1 = [0u8; 32];
        let key2 = [1u8; 32];

        let mut mac_ctx1 = device1
            .init(HmacSha2_256, &key1)
            .expect("Failed to initialize MAC");
        mac_ctx1.update(b"test data").expect("Failed to update MAC");
        let result1 = mac_ctx1.finalize().expect("Failed to finalize MAC");

        let mut mac_ctx2 = device2
            .init(HmacSha2_256, &key2)
            .expect("Failed to initialize MAC");
        mac_ctx2.update(b"test data").expect("Failed to update MAC");
        let result2 = mac_ctx2.finalize().expect("Failed to finalize MAC");

        // Different keys should produce different outputs (in a real implementation)
        // Our mock should also produce different outputs due to key_hash
        assert_ne!(result1.value, result2.value);
    }
}

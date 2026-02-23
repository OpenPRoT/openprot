// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Mock Hash/Digest Accelerator Implementation
//!
//! Provides a stub implementation of digest operations that can be used
//! for testing when real hardware acceleration is not available.
//!
//! This module demonstrates both the scoped and owned digest APIs:
//! - **Scoped API**: Traditional lifetime-constrained contexts for simple use cases
//! - **Owned API**: Move-based resource management for server applications

// Allow security lints for mock/test code
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::arithmetic_side_effects)]

use openprot_hal_blocking::digest::{
    DigestAlgorithm, ErrorKind, ErrorType, Sha2_256, Sha2_384, Sha2_512,
};

// Import both API modules
use openprot_hal_blocking::digest::scoped::{DigestCtrlReset, DigestInit, DigestOp};

/// Mock digest accelerator device
///
/// This is a software-only stub implementation of the digest hardware traits.
/// It provides working digest operations using simple algorithms or dummy outputs
/// for testing purposes.
#[derive(Default)]
pub struct MockDigestDevice;

impl MockDigestDevice {
    /// Create a new mock digest device
    pub fn new() -> Self {
        Self
    }
}

/// Mock digest error type
#[derive(Debug, Clone, Copy)]
pub struct MockDigestError;

impl openprot_hal_blocking::digest::Error for MockDigestError {
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

//
// SCOPED API IMPLEMENTATION (Original)
//

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

/// Macro to implement scoped digest traits for each algorithm
macro_rules! impl_scoped_sha2 {
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

impl_scoped_sha2!(Sha2_256);
impl_scoped_sha2!(Sha2_384);
impl_scoped_sha2!(Sha2_512);

//
// OWNED API IMPLEMENTATION (Move-based Resource Management)
//

/// Demonstrates the new owned API with move-based resource management for persistent sessions
pub mod owned {
    use super::*;
    use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};

    /// Controller for owned digest operations
    ///
    /// This represents the hardware controller that can create owned contexts.
    /// Unlike the scoped API, this can be moved into and out of contexts.
    #[derive(Debug)]
    pub struct MockDigestController {
        // Hardware state could go here
        // For AST1060, this might track the single hardware context
        #[allow(dead_code)] // Mock implementation
        hardware_id: u32,
    }

    impl Default for MockDigestController {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MockDigestController {
        pub fn new() -> Self {
            Self {
                hardware_id: 0xDEADBEEF,
            }
        }
    }

    impl ErrorType for MockDigestController {
        type Error = MockDigestError;
    }

    /// Owned digest context for a specific algorithm
    ///
    /// This context owns the controller and can be stored in structs,
    /// moved across function boundaries, and persist across IPC calls.
    pub struct MockOwnedContext<T> {
        controller: MockDigestController,
        #[allow(dead_code)] // Algorithm type for type safety
        algorithm: T,
        data_processed: u64,
    }

    impl<T> ErrorType for MockOwnedContext<T> {
        type Error = MockDigestError;
    }

    /// Macro to implement owned digest traits for each algorithm
    macro_rules! impl_owned_sha2 {
        ($algo:ident) => {
            impl DigestInit<$algo> for MockDigestController {
                type Context = MockOwnedContext<$algo>;
                type Output = <$algo as DigestAlgorithm>::Digest;

                fn init(self, init_params: $algo) -> Result<Self::Context, Self::Error> {
                    // Controller moves into the context
                    // In hardware implementation, this might claim hardware resources
                    Ok(MockOwnedContext {
                        controller: self,
                        algorithm: init_params,
                        data_processed: 0,
                    })
                }
            }

            impl DigestOp for MockOwnedContext<$algo> {
                type Output = <$algo as DigestAlgorithm>::Digest;
                type Controller = MockDigestController;

                fn update(mut self, data: &[u8]) -> Result<Self, Self::Error> {
                    // Process data and return updated context
                    self.data_processed += data.len() as u64;
                    // In hardware implementation, this might feed data to hardware
                    Ok(self)
                }

                fn finalize(self) -> Result<(Self::Output, Self::Controller), Self::Error> {
                    // Generate digest and return both result and controller
                    const OUTPUT_WORDS: usize = <$algo as DigestAlgorithm>::OUTPUT_BITS / 32;
                    let mut value = [0u32; OUTPUT_WORDS];
                    for (i, word) in value.iter_mut().enumerate() {
                        *word = 0x87654321u32 // Different pattern to distinguish from scoped
                            .wrapping_add(self.data_processed as u32)
                            .wrapping_add(i as u32);
                    }

                    let result = Self::Output { value };
                    let controller = self.controller; // Move controller back

                    Ok((result, controller))
                }

                fn cancel(self) -> Self::Controller {
                    // Clean cancellation - return controller without producing output
                    // In hardware implementation, this might reset hardware state
                    self.controller
                }
            }
        };
    }

    impl_owned_sha2!(Sha2_256);
    impl_owned_sha2!(Sha2_384);
    impl_owned_sha2!(Sha2_512);
}

#[cfg(test)]
mod tests {
    use super::*;
    use openprot_hal_blocking::digest::{Digest, Sha2_256};

    #[test]
    fn test_scoped_api() {
        let mut device = MockDigestDevice::new();

        // Test the scoped API
        let mut ctx = device.init(Sha2_256).unwrap();
        ctx.update(b"hello").unwrap();
        ctx.update(b" world").unwrap();
        let digest = ctx.finalize().unwrap();

        // Verify we got a digest with the expected pattern
        assert_eq!(digest.value[0], 0x12345678 + 11); // 11 bytes processed
    }

    #[test]
    fn test_owned_api() {
        use crate::hash::owned::MockDigestController;
        use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};

        let controller = MockDigestController::new();

        // Test the owned API - contexts can be stored and moved
        let ctx = controller.init(Sha2_256).unwrap();
        let ctx = ctx.update(b"hello").unwrap();
        let ctx = ctx.update(b" world").unwrap();
        let (digest, recovered_controller) = ctx.finalize().unwrap();

        // Verify we got a digest with the expected pattern (different from scoped)
        assert_eq!(digest.value[0], 0x87654321 + 11); // 11 bytes processed

        // Controller is recovered and can be reused
        let _new_ctx = recovered_controller.init(Sha2_256).unwrap();
    }

    #[test]
    fn test_owned_api_cancel() {
        use crate::hash::owned::MockDigestController;
        use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};

        let controller = MockDigestController::new();

        let ctx = controller.init(Sha2_256).unwrap();
        let ctx = ctx.update(b"some data").unwrap();

        // Cancel the operation and recover controller
        let recovered_controller = ctx.cancel();

        // Controller can be reused after cancellation
        let _new_ctx = recovered_controller.init(Sha2_256).unwrap();
    }

    #[test]
    fn test_session_storage_pattern() {
        use crate::hash::owned::{MockDigestController, MockOwnedContext};
        use openprot_hal_blocking::digest::owned::{DigestInit, DigestOp};

        // Demonstrate session storage pattern (impossible with scoped API)
        // This simulates what a server would do to store contexts
        struct SimpleSessionManager {
            session: Option<MockOwnedContext<Sha2_256>>,
            controller: Option<MockDigestController>,
        }

        impl SimpleSessionManager {
            fn new() -> Self {
                Self {
                    session: None,
                    controller: Some(MockDigestController::new()),
                }
            }

            fn create_session(&mut self) -> Result<(), MockDigestError> {
                let controller = self.controller.take().unwrap();
                let context = controller.init(Sha2_256)?;
                self.session = Some(context);
                Ok(())
            }

            fn update_session(&mut self, data: &[u8]) -> Result<(), MockDigestError> {
                let context = self.session.take().unwrap();
                let updated_context = context.update(data)?;
                self.session = Some(updated_context);
                Ok(())
            }

            fn finalize_session(&mut self) -> Result<Digest<8>, MockDigestError> {
                let context = self.session.take().unwrap();
                let (result, controller) = context.finalize()?;
                self.controller = Some(controller); // Resource recovered
                Ok(result)
            }
        }

        let mut manager = SimpleSessionManager::new();

        // Create a session and process data across multiple calls
        manager.create_session().unwrap();
        manager.update_session(b"hello").unwrap();
        manager.update_session(b" world").unwrap();
        let result = manager.finalize_session().unwrap();

        // Verify we got a meaningful result
        assert_eq!(result.value[0], 0x87654321 + 11); // 11 bytes processed

        // Controller was recovered and can be reused
        manager.create_session().unwrap();
        let result2 = manager.finalize_session().unwrap();
        assert_eq!(result2.value[0], 0x87654321); // 0 bytes processed
    }
}

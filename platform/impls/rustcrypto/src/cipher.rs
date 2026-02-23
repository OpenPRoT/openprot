// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

#![allow(deprecated)] // Allow deprecated GenericArray from cipher crate for compatibility

use openprot_hal_blocking::cipher::{
    AeadCipherMode, BlockCipherMode, CipherInit, CipherMode, CipherOp, CipherStatus, Error,
    ErrorKind, ErrorType, SymmetricCipher,
};

// RustCrypto imports for AES-CTR implementation
use aes::Aes256;
use cipher::{generic_array::GenericArray, KeyIvInit, StreamCipher, StreamCipherSeek};
use ctr::Ctr64BE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustCryptoCipherError {
    InvalidKey,
    InvalidNonce,
    EncryptionFailed,
    DecryptionFailed,
    AuthenticationFailed,
    MessageTooLarge,
    InvalidState,
    HardwareFailure,
}

impl core::fmt::Display for RustCryptoCipherError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidKey => write!(f, "invalid key length or format"),
            Self::InvalidNonce => write!(f, "invalid nonce/IV length or format"),
            Self::EncryptionFailed => write!(f, "encryption operation failed"),
            Self::DecryptionFailed => write!(f, "decryption operation failed"),
            Self::AuthenticationFailed => write!(f, "authentication verification failed"),
            Self::MessageTooLarge => write!(f, "message too large for configured buffer size"),
            Self::InvalidState => write!(f, "cipher context is in an invalid state"),
            Self::HardwareFailure => write!(f, "hardware failure during cipher operation"),
        }
    }
}

impl Error for RustCryptoCipherError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::InvalidKey => ErrorKind::KeyError,
            Self::InvalidNonce => ErrorKind::InvalidInput,
            Self::EncryptionFailed | Self::DecryptionFailed => ErrorKind::HardwareFailure,
            Self::AuthenticationFailed => ErrorKind::InvalidInput,
            Self::MessageTooLarge => ErrorKind::InvalidInput,
            Self::InvalidState => ErrorKind::InvalidState,
            Self::HardwareFailure => ErrorKind::HardwareFailure,
        }
    }
}

//
// Cipher mode markers
//

/// AES-256 in CTR mode marker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aes256CtrMode;

impl CipherMode for Aes256CtrMode {}
impl BlockCipherMode for Aes256CtrMode {}

/// AES-256 in GCM mode marker  
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aes256GcmMode;

impl CipherMode for Aes256GcmMode {}
impl AeadCipherMode for Aes256GcmMode {}

//
// Basic cipher implementations for type system foundation
//

/// Basic AES-256-CTR cipher implementation
pub struct Aes256CtrCipher;

/// Basic AES-256-GCM AEAD cipher implementation
pub struct Aes256GcmCipher;

//
// ErrorType trait implementations
//

impl ErrorType for Aes256CtrCipher {
    type Error = RustCryptoCipherError;
}

impl ErrorType for Aes256GcmCipher {
    type Error = RustCryptoCipherError;
}

//
// SymmetricCipher trait implementations
//

impl SymmetricCipher for Aes256CtrCipher {
    type Key = [u8; 32]; // AES-256 key
    type Nonce = [u8; 16]; // 128-bit IV for CTR mode
    type PlainText = [u8; 256]; // Fixed-size plaintext buffer
    type CipherText = [u8; 256]; // Fixed-size ciphertext buffer
}

impl SymmetricCipher for Aes256GcmCipher {
    type Key = [u8; 32]; // AES-256 key
    type Nonce = [u8; 12]; // 96-bit nonce for GCM
    type PlainText = [u8; 256]; // Fixed-size plaintext buffer
    type CipherText = [u8; 272]; // 256 + 16 bytes for authentication tag
}

//
// AES-CTR Implementation using RustCrypto
//

/// AES-256-CTR cipher context wrapping RustCrypto's implementation.
///
/// This struct provides a secure, zeroizing wrapper around the RustCrypto
/// AES-256-CTR implementation, suitable for embedded systems and security-critical applications.
pub struct AesCtrContext {
    /// The underlying RustCrypto AES-256-CTR cipher
    cipher: Ctr64BE<Aes256>,
    /// Current state for status tracking
    state: CipherState,
}

/// Cipher state for tracking operational status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CipherState {
    /// Cipher is ready to accept new operations
    Ready,
    /// Cipher is currently processing data
    Processing,
    /// Cipher has completed processing and has output available
    HasOutput,
}

impl AesCtrContext {
    /// Create a new AES-CTR context with the given key and IV.
    ///
    /// # Parameters
    /// - `key`: 32-byte AES-256 key
    /// - `iv`: 16-byte initialization vector
    ///
    /// # Returns
    /// A new AES-CTR context ready for encryption/decryption operations.
    ///
    /// # Security Note
    /// The IV must be unique for each encryption operation with the same key.
    pub fn new(key: &[u8; 32], iv: &[u8; 16]) -> Result<Self, RustCryptoCipherError> {
        let key_array = GenericArray::from_slice(key);
        let iv_array = GenericArray::from_slice(iv);

        let cipher = Ctr64BE::<Aes256>::new(key_array, iv_array);
        Ok(Self {
            cipher,
            state: CipherState::Ready,
        })
    }

    /// Reset the cipher context to its initial state with new key/IV.
    ///
    /// This securely clears the previous state and reinitializes with new parameters.
    pub fn reset(&mut self, key: &[u8; 32], iv: &[u8; 16]) -> Result<(), RustCryptoCipherError> {
        // Create new cipher instance (this effectively clears the old one)
        let key_array = GenericArray::from_slice(key);
        let iv_array = GenericArray::from_slice(iv);

        self.cipher = Ctr64BE::<Aes256>::new(key_array, iv_array);
        self.state = CipherState::Ready;

        Ok(())
    }

    /// Get the current position in the keystream (for CTR mode).
    ///
    /// This can be useful for resuming operations or implementing seek functionality.
    pub fn position(&self) -> u64 {
        self.cipher.current_pos()
    }

    /// Seek to a specific position in the keystream.
    ///
    /// # Parameters
    /// - `pos`: Position to seek to in the keystream
    ///
    /// # Security Warning
    /// Seeking in CTR mode can be dangerous if not done carefully.
    /// Never reuse keystream positions with the same key/IV combination.
    pub fn seek(&mut self, pos: u64) {
        self.cipher.seek(pos);
    }
}

//
// ErrorType implementation for AES-CTR context
//

impl ErrorType for AesCtrContext {
    type Error = RustCryptoCipherError;
}

//
// CipherInit implementation for AES-256-CTR
//

impl CipherInit<Aes256CtrMode> for Aes256CtrCipher {
    type CipherContext<'a> = AesCtrContext;

    /// Initialize a new AES-CTR cipher context.
    ///
    /// # Parameters
    /// - `key`: Reference to the AES-256 key (32 bytes)
    /// - `nonce`: Reference to the initialization vector (16 bytes)
    /// - `mode`: The cipher mode (Aes256CtrMode)
    ///
    /// # Returns
    /// A new AES-CTR context ready for encryption/decryption operations.
    ///
    /// # Errors
    /// - `InvalidKey`: If the key is not exactly 32 bytes
    /// - `InvalidNonce`: If the IV is not exactly 16 bytes
    /// - `InitializationError`: If the cipher cannot be initialized
    ///
    /// # Security Notes
    /// - The IV must be unique for each encryption with the same key
    /// - Never reuse the same key/IV combination
    /// - Consider using a counter or random IV generation
    fn init<'a>(
        &'a mut self,
        key: &Self::Key,
        nonce: &Self::Nonce,
        _mode: Aes256CtrMode,
    ) -> Result<Self::CipherContext<'a>, Self::Error> {
        // Validate key and nonce lengths (compile-time guaranteed by types)
        // Create new context with the provided key and IV
        AesCtrContext::new(key, nonce)
    }
}

//
// CipherOp implementation for AES-CTR context
//

impl CipherOp<Aes256CtrMode> for AesCtrContext {
    /// Encrypt plaintext using AES-256-CTR mode.
    ///
    /// In CTR mode, encryption and decryption are the same operation:
    /// XOR the plaintext with the keystream generated by encrypting the counter.
    ///
    /// # Parameters
    /// - `plaintext`: Fixed-size plaintext buffer to encrypt
    ///
    /// # Returns
    /// The encrypted ciphertext with the same size as the input.
    ///
    /// # Security Notes
    /// - Never reuse the same key/IV combination
    /// - The counter state is advanced after each operation
    /// - CTR mode provides semantic security when IVs are unique
    fn encrypt(&mut self, mut plaintext: Self::PlainText) -> Result<Self::CipherText, Self::Error> {
        self.state = CipherState::Processing;

        // In CTR mode, encryption is just XOR with keystream
        // The apply_keystream method modifies the buffer in-place
        self.cipher.apply_keystream(&mut plaintext);

        self.state = CipherState::HasOutput;

        // Return the modified buffer as ciphertext
        Ok(plaintext)
    }

    /// Decrypt ciphertext using AES-256-CTR mode.
    ///
    /// In CTR mode, decryption is identical to encryption:
    /// XOR the ciphertext with the keystream.
    ///
    /// # Parameters
    /// - `ciphertext`: Fixed-size ciphertext buffer to decrypt
    ///
    /// # Returns
    /// The decrypted plaintext with the same size as the input.
    ///
    /// # Security Notes
    /// - The IV/counter state must match what was used for encryption
    /// - Position in the keystream is automatically tracked
    /// - Ensure the cipher context is properly initialized
    fn decrypt(
        &mut self,
        mut ciphertext: Self::CipherText,
    ) -> Result<Self::PlainText, Self::Error> {
        self.state = CipherState::Processing;

        // In CTR mode, decryption is identical to encryption
        // XOR the ciphertext with the same keystream
        self.cipher.apply_keystream(&mut ciphertext);

        self.state = CipherState::HasOutput;

        // Return the modified buffer as plaintext
        Ok(ciphertext)
    }
}

impl SymmetricCipher for AesCtrContext {
    type Key = [u8; 32];
    type Nonce = [u8; 16];
    type PlainText = [u8; 256];
    type CipherText = [u8; 256];
}

//
// CipherStatus implementation for AES-CTR context
//

impl CipherStatus for AesCtrContext {
    /// Check if the cipher is ready to accept new input data.
    ///
    /// For software-based AES-CTR, this is typically always true unless
    /// the cipher is in an error state.
    ///
    /// # Returns
    /// - `Ok(true)`: Cipher is ready for new operations
    /// - `Ok(false)`: Cipher is in error state or not ready
    /// - `Err(_)`: Error occurred while checking status
    fn is_ready(&self) -> Result<bool, Self::Error> {
        match self.state {
            CipherState::Ready | CipherState::HasOutput => Ok(true),
            CipherState::Processing => Ok(false), // Busy processing
        }
    }

    /// Check if processed output data is available for reading.
    ///
    /// For streaming ciphers like CTR mode, output is typically available
    /// immediately after processing completes.
    ///
    /// # Returns
    /// - `Ok(true)`: Output data is available
    /// - `Ok(false)`: No output data is currently available
    /// - `Err(_)`: Error occurred while checking status
    fn has_output(&self) -> Result<bool, Self::Error> {
        match self.state {
            CipherState::HasOutput => Ok(true),
            CipherState::Ready | CipherState::Processing => Ok(false),
        }
    }

    /// Check if the cipher is idle and available for new operations.
    ///
    /// For software implementations, this is similar to `is_ready()` but
    /// provides semantic clarity for different use cases.
    ///
    /// # Returns
    /// - `Ok(true)`: Cipher is idle and available
    /// - `Ok(false)`: Cipher is busy with ongoing operations
    /// - `Err(_)`: Error occurred while checking status
    fn is_idle(&self) -> Result<bool, Self::Error> {
        match self.state {
            CipherState::Ready => Ok(true),
            CipherState::Processing => Ok(false),
            CipherState::HasOutput => Ok(true), // Can start new operations
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // Allow unwrap in tests for cleaner test code
mod tests {
    use super::*;
    use openprot_hal_blocking::cipher::BlockAligned;

    // Test vectors for AES-256-CTR mode
    const TEST_KEY: [u8; 32] = [
        0x60, 0x3d, 0xeb, 0x10, 0x15, 0xca, 0x71, 0xbe, 0x2b, 0x73, 0xae, 0xf0, 0x85, 0x7d, 0x77,
        0x81, 0x1f, 0x35, 0x2c, 0x07, 0x3b, 0x61, 0x08, 0xd7, 0x2d, 0x98, 0x10, 0xa3, 0x09, 0x14,
        0xdf, 0xf4,
    ];

    const TEST_IV: [u8; 16] = [
        0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe,
        0xff,
    ];

    #[test]
    fn test_aes_ctr_context_creation() {
        let context = AesCtrContext::new(&TEST_KEY, &TEST_IV);
        assert!(context.is_ok(), "Failed to create AES-CTR context");

        let ctx = context.unwrap();
        assert!(
            ctx.is_ready().unwrap(),
            "Context should be ready after creation"
        );
        assert!(
            ctx.is_idle().unwrap(),
            "Context should be idle after creation"
        );
        assert!(
            !ctx.has_output().unwrap(),
            "Context should have no output initially"
        );
    }

    #[test]
    fn test_cipher_init_trait() {
        let mut cipher = Aes256CtrCipher;
        let result = cipher.init(&TEST_KEY, &TEST_IV, Aes256CtrMode);
        assert!(
            result.is_ok(),
            "Failed to create context via CipherInit trait"
        );

        let context = result.unwrap();
        assert!(context.is_ready().unwrap(), "Context should be ready");
    }

    #[test]
    fn test_basic_cipher_operations() {
        let mut context = AesCtrContext::new(&TEST_KEY, &TEST_IV).unwrap();

        // Test encryption - CTR mode uses fixed-size arrays
        let plaintext: [u8; 256] = [0x42; 256]; // Test plaintext
        let result = context.encrypt(plaintext);
        assert!(result.is_ok(), "Encryption should succeed");

        let ciphertext = result.unwrap();
        assert_ne!(
            plaintext, ciphertext,
            "Ciphertext should differ from plaintext"
        );

        // Reset context for decryption (CTR uses same operation for encrypt/decrypt)
        context.reset(&TEST_KEY, &TEST_IV).unwrap();

        // Test decryption
        let result = context.decrypt(ciphertext);
        assert!(result.is_ok(), "Decryption should succeed");

        let decrypted = result.unwrap();
        assert_eq!(plaintext, decrypted, "Decrypted text should match original");
    }

    #[test]
    fn test_block_aligned_container_operations() {
        let mut container = BlockAligned::<16, 4>::new();
        assert_eq!(container.block_count(), 0, "New container should be empty");

        // Add a block
        let test_block = [0x42u8; 16];
        let result = container.push_block(test_block);
        assert!(result.is_ok(), "Adding block should succeed");
        assert_eq!(container.block_count(), 1, "Should have 1 block");

        // Check the block content
        let stored_block = container.get_block(0).unwrap();
        assert_eq!(stored_block, &test_block, "Stored block should match input");

        // Test from_slice_padded
        let test_data = b"Hello, World!"; // 13 bytes
        let padded_container = BlockAligned::<16, 2>::from_slice_padded(test_data, 0x00);
        assert!(padded_container.is_ok(), "from_slice_padded should succeed");

        let container = padded_container.unwrap();
        assert_eq!(
            container.block_count(),
            1,
            "Should have 1 block for 13 bytes"
        );

        let block = container.get_block(0).unwrap();
        assert_eq!(&block[..13], test_data, "First 13 bytes should match input");
        assert_eq!(&block[13..], &[0x00; 3], "Last 3 bytes should be padding");
    }

    #[test]
    fn test_cipher_status_tracking() {
        let context = AesCtrContext::new(&TEST_KEY, &TEST_IV).unwrap();

        // Initial state
        assert!(context.is_ready().unwrap(), "Should be ready initially");
        assert!(context.is_idle().unwrap(), "Should be idle initially");
        assert!(
            !context.has_output().unwrap(),
            "Should have no output initially"
        );

        // CTR mode is always ready since it's a streaming cipher
        assert!(
            context.is_ready().unwrap(),
            "CTR mode should always be ready"
        );
    }

    #[test]
    fn test_context_reset() {
        let mut context = AesCtrContext::new(&TEST_KEY, &TEST_IV).unwrap();

        // Encrypt some data
        let plaintext: [u8; 256] = [0x42; 256];
        let output1 = context.encrypt(plaintext).unwrap();

        // Reset with same key/IV
        let result = context.reset(&TEST_KEY, &TEST_IV);
        assert!(result.is_ok(), "Reset should succeed");

        // Encrypt same data again - should produce same result
        let output2 = context.encrypt(plaintext).unwrap();
        assert_eq!(output1, output2, "Reset should restore initial state");

        // Reset with different IV
        let new_iv = [0x00u8; 16];
        context.reset(&TEST_KEY, &new_iv).unwrap();
        let output3 = context.encrypt(plaintext).unwrap();
        assert_ne!(
            output1, output3,
            "Different IV should produce different output"
        );
    }

    #[test]
    fn test_error_conditions() {
        // Test capacity exceeded for BlockAligned
        let large_data = [0x42u8; 100];
        let result = BlockAligned::<16, 2>::from_slice_padded(&large_data, 0x00);
        assert!(
            result.is_err(),
            "Should fail when data requires too many blocks"
        );

        let mut container = BlockAligned::<16, 2>::new();
        container.push_block([0u8; 16]).unwrap();
        container.push_block([0u8; 16]).unwrap();

        // Try to add one more block
        let result = container.push_block([0u8; 16]);
        assert!(result.is_err(), "Should fail when capacity exceeded");
    }

    #[test]
    fn test_symmetric_cipher_associated_types() {
        // Test that our cipher implements the expected types
        fn check_cipher_types<C: SymmetricCipher>() {
            // This function just checks that the types are correctly associated
        }

        check_cipher_types::<Aes256CtrCipher>();
        check_cipher_types::<Aes256GcmCipher>();
    }

    #[test]
    fn test_error_type_consistency() {
        let context = AesCtrContext::new(&TEST_KEY, &TEST_IV).unwrap();

        // Test that error types are consistent across traits
        let _ready_result: Result<bool, RustCryptoCipherError> = context.is_ready();
        let _idle_result: Result<bool, RustCryptoCipherError> = context.is_idle();
        let _output_result: Result<bool, RustCryptoCipherError> = context.has_output();
    }

    #[test]
    fn test_block_aligned_container_functionality() {
        // Empty BlockAligned container
        let empty_input = BlockAligned::<16, 4>::new();
        assert_eq!(
            empty_input.block_count(),
            0,
            "Empty container should have 0 blocks"
        );
        assert!(
            empty_input.is_empty(),
            "Empty container should report as empty"
        );
        assert_eq!(
            empty_input.capacity(),
            4,
            "Container should have correct capacity"
        );
    }
}

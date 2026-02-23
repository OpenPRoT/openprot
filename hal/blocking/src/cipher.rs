// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use core::fmt::Debug;
use zerocopy::{FromBytes, IntoBytes};

/// Marker trait for all cipher modes.
pub trait CipherMode: core::fmt::Debug + Clone + Copy {}

/// Marker trait for block cipher modes (e.g., CBC, CTR).
pub trait BlockCipherMode: CipherMode {}

/// Marker trait for AEAD modes (e.g., GCM, CCM).
pub trait AeadCipherMode: CipherMode {}

/// Marker trait for stream cipher modes (e.g., ChaCha20).
pub trait StreamCipherMode: CipherMode {}

/// Common error kinds for symmetric cipher operations.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Failed to initialize the cipher context.
    InitializationError,

    /// General hardware failure during cipher operation.
    HardwareFailure,

    /// Insufficient permissions to access hardware or perform the operation.
    PermissionDenied,

    /// The cipher context is in an invalid or uninitialized state.
    InvalidState,

    /// The input data is invalid (e.g., wrong length or format).
    InvalidInput,

    /// The specified algorithm or mode is not supported.
    UnsupportedAlgorithm,

    /// Key or IV is invalid or missing.
    KeyError,
}

/// Trait for converting implementation-specific errors into a generic [`ErrorKind`].
pub trait Error: Debug {
    /// Returns a generic error kind corresponding to the specific error.
    fn kind(&self) -> ErrorKind;
}

/// Trait for associating a type with an error type.
pub trait ErrorType {
    /// The associated error type.
    type Error: Error;
}

/// Trait for symmetric cipher algorithms.
///
/// This trait defines the core types and constraints for symmetric cipher implementations.
/// All types must support zero-copy serialization via the `zerocopy` crate traits,
/// enabling efficient operation with both software and hardware implementations.
///
/// # Zero-Copy Requirements
///
/// The `FromBytes` and `IntoBytes` trait bounds ensure that:
/// - Types can be safely constructed from byte arrays without validation
/// - Types can be converted to byte arrays for hardware or network transmission
/// - No unnecessary copying occurs during cryptographic operations
/// - Memory layout is well-defined and predictable
///
/// # Security Considerations
///
/// - Key types should implement `Zeroize` for secure memory cleanup
/// - Plaintext and ciphertext types should be handled securely in memory
/// - Consider using `secrecy` crate for sensitive data protection
///
/// # Example Implementation
///
/// ```ignore
/// impl SymmetricCipher for MyAesCipher {
///     type Key = [u8; 32];           // AES-256 key
///     type Nonce = [u8; 16];         // 128-bit nonce/IV
///     type PlainText = [u8; 64];     // Fixed-size plaintext buffer
///     type CipherText = [u8; 80];    // Fixed-size ciphertext buffer (with padding)
///     type Error = CryptoError;
/// }
/// ```
pub trait SymmetricCipher: ErrorType {
    /// The cryptographic key type.
    ///
    /// This type represents the secret key material used for encryption and decryption.
    /// The key can be provided through various mechanisms including software keys,
    /// key vault references, or hardware-managed keys.
    ///
    /// # Security Requirements
    ///
    /// - Should implement `Zeroize` for secure memory cleanup when stored in software
    /// - May reference keys stored in secure hardware or key vaults
    /// - Size should match the algorithm's key requirements (e.g., 32 bytes for AES-256)
    /// - Consider using masked key shares for side-channel protection in hardware
    ///
    /// # Key Sources
    ///
    /// - **Software Keys**: `[u8; 32]` for AES-256, `[u8; 16]` for AES-128
    /// - **Key Vault References**: `KeyVaultHandle`, `KeyId`, or similar abstract types
    /// - **Hardware Keys**: Sideloaded keys from key managers or crypto coprocessors
    /// - **Masked Keys**: Split key shares for side-channel attack resistance
    ///
    /// # Common Types
    ///
    /// - `[u8; 32]` for AES-256, ChaCha20 (software keys)
    /// - `[u8; 16]` for AES-128 (software keys)
    /// - `KeyVaultId` for key vault managed keys
    /// - `HardwareKeyHandle` for hardware-managed keys
    /// - Custom types for hardware-specific key formats or masked implementations
    type Key;

    /// The nonce or initialization vector type.
    ///
    /// This type represents the nonce (number used once) or initialization vector
    /// for the cipher operation. It must be unique for each encryption with the same key.
    ///
    /// # Security Requirements
    ///
    /// - Must never be reused with the same key (critical for CTR mode and stream ciphers)
    /// - Should be generated using cryptographically secure random number generation
    /// - Size must match the algorithm's requirements
    ///
    /// # Common Types
    ///
    /// - `[u8; 16]` for AES-CTR, AES-CBC
    /// - `[u8; 12]` for ChaCha20, AES-GCM
    /// - `[u8; 8]` for some legacy ciphers
    type Nonce: FromBytes + IntoBytes;

    /// The plaintext data type.
    ///
    /// This type represents the unencrypted data input to the cipher.
    /// It must support zero-copy operations for efficient processing.
    ///
    /// # Performance Considerations
    ///
    /// - Should minimize copying and allocation
    /// - Consider using slices or references where possible
    /// - Support both fixed-size and variable-length data
    ///
    /// # Common Types
    ///
    /// - `&[u8]` for read-only operations
    /// - `[u8; N]` for fixed-size owned data
    /// - Custom container types for variable-length data
    type PlainText: FromBytes + IntoBytes;

    /// The ciphertext data type.
    ///
    /// This type represents the encrypted data output from the cipher.
    /// It must support zero-copy operations for efficient processing.
    ///
    /// # Size Considerations
    ///
    /// - For stream ciphers: same size as plaintext
    /// - For block ciphers with padding: may be larger than plaintext
    /// - For AEAD modes: includes authentication tag
    ///
    /// # Common Types
    ///
    /// - `[u8; N]` for fixed-size encrypted blocks
    /// - Custom container types for variable-length encrypted data
    /// - Types that include metadata or authentication tags
    type CipherText: FromBytes + IntoBytes;
}

/// Trait for initializing a cipher with a specific mode.
pub trait CipherInit<M: CipherMode>: SymmetricCipher {
    /// The operational context for performing encryption/decryption.
    type CipherContext<'a>: CipherOp<M>
    where
        Self: 'a;

    /// Initializes the cipher with the given parameters.
    ///
    /// # Parameters
    ///
    /// - `key`: A reference to the key used for the cipher.
    /// - `nonce`: A reference to the nonce or IV used for the cipher.
    /// - `mode`: The cipher mode to use.
    ///
    /// # Returns
    ///
    /// A result containing the operational context or an error.
    fn init<'a>(
        &'a mut self,
        key: &Self::Key,
        nonce: &Self::Nonce,
        mode: M,
    ) -> Result<Self::CipherContext<'a>, Self::Error>;
}

/// Trait for basic encryption/decryption operations.
pub trait CipherOp<M: CipherMode>: SymmetricCipher + ErrorType {
    /// Encrypts the given plaintext.
    ///
    /// # Parameters
    ///
    /// - `plaintext`: The data to encrypt.
    ///
    /// # Returns
    ///
    /// A result containing the ciphertext or an error.
    fn encrypt(&mut self, plaintext: Self::PlainText) -> Result<Self::CipherText, Self::Error>;

    /// Decrypts the given ciphertext.
    ///
    /// # Parameters
    ///
    /// - `ciphertext`: The data to decrypt.
    ///
    /// # Returns
    ///
    /// A result containing the plaintext or an error.
    fn decrypt(&mut self, ciphertext: Self::CipherText) -> Result<Self::PlainText, Self::Error>;
}

/// Optional trait for cipher contexts that support resetting to their initial state.
pub trait ResettableCipherOp: ErrorType {
    /// Resets the cipher context.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    fn reset(&mut self) -> Result<(), Self::Error>;
}

/// Optional trait for cipher contexts that support rekeying.
pub trait CipherRekey<K>: ErrorType {
    /// Rekeys the cipher context with a new key.
    ///
    /// # Parameters
    ///
    /// - `new_key`: A reference to the new key.
    ///
    /// # Returns
    ///
    /// A result indicating success or failure.
    fn rekey(&mut self, new_key: &K) -> Result<(), Self::Error>;
}

/// Error type for block-aligned container operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockAlignedError {
    /// The container has reached its maximum capacity.
    CapacityExceeded,
    /// The input data is too large for the container.
    DataTooLarge,
}

impl core::fmt::Display for BlockAlignedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CapacityExceeded => write!(f, "block container has reached maximum capacity"),
            Self::DataTooLarge => write!(f, "input data exceeds container capacity"),
        }
    }
}

/// Block-aligned data container that guarantees correct block sizing at compile time.
///
/// This type wrapper ensures that data is always properly aligned to block boundaries,
/// preventing runtime errors from incorrectly sized cipher inputs. It uses fixed-size
/// arrays suitable for embedded systems without heap allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockAligned<const BLOCK_SIZE: usize, const MAX_BLOCKS: usize> {
    blocks: [[u8; BLOCK_SIZE]; MAX_BLOCKS],
    block_count: usize,
}

impl<const BLOCK_SIZE: usize, const MAX_BLOCKS: usize> Default
    for BlockAligned<BLOCK_SIZE, MAX_BLOCKS>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const BLOCK_SIZE: usize, const MAX_BLOCKS: usize> BlockAligned<BLOCK_SIZE, MAX_BLOCKS> {
    /// Create a new empty block-aligned container
    pub const fn new() -> Self {
        Self {
            blocks: [[0u8; BLOCK_SIZE]; MAX_BLOCKS],
            block_count: 0,
        }
    }

    /// Create block-aligned data from a byte slice, padding if necessary.
    ///
    /// # Parameters
    /// - `data`: Input data that will be padded to block boundaries
    /// - `padding_byte`: Byte value to use for padding (typically 0)
    ///
    /// # Returns
    /// - `Ok(BlockAligned)`: Successfully created block-aligned data
    /// - `Err(BlockAlignedError)`: Input data exceeds maximum capacity
    ///
    /// # Errors
    /// Returns an error if the input data would require more than `MAX_BLOCKS` blocks.
    pub fn from_slice_padded(data: &[u8], padding_byte: u8) -> Result<Self, BlockAlignedError> {
        let required_blocks = data.len().div_ceil(BLOCK_SIZE);

        if required_blocks > MAX_BLOCKS {
            return Err(BlockAlignedError::DataTooLarge);
        }

        let mut result = Self::new();
        result.block_count = required_blocks;

        // Fill complete blocks
        for (i, chunk) in data.chunks(BLOCK_SIZE).enumerate() {
            let block = result
                .blocks
                .get_mut(i)
                .ok_or(BlockAlignedError::DataTooLarge)?;
            block.fill(padding_byte);
            let slice = block
                .get_mut(..chunk.len())
                .ok_or(BlockAlignedError::DataTooLarge)?;
            slice.copy_from_slice(chunk);
        }

        Ok(result)
    }

    /// Add a complete block to the container.
    ///
    /// # Parameters
    /// - `block`: Block data to add
    ///
    /// # Returns
    /// - `Ok(())`: Block successfully added
    /// - `Err(BlockAlignedError)`: Container is at maximum capacity
    pub fn push_block(&mut self, block: [u8; BLOCK_SIZE]) -> Result<(), BlockAlignedError> {
        if self.block_count >= MAX_BLOCKS {
            return Err(BlockAlignedError::CapacityExceeded);
        }

        let block_slot = self
            .blocks
            .get_mut(self.block_count)
            .ok_or(BlockAlignedError::CapacityExceeded)?;
        *block_slot = block;
        self.block_count = self.block_count.saturating_add(1);
        Ok(())
    }

    /// Get the blocks as a slice containing only the valid blocks.
    pub fn blocks(&self) -> &[[u8; BLOCK_SIZE]] {
        &self.blocks[..self.block_count]
    }

    /// Get the total number of bytes in valid blocks.
    pub const fn len(&self) -> usize {
        self.block_count.saturating_mul(BLOCK_SIZE)
    }

    /// Check if the container is empty.
    pub const fn is_empty(&self) -> bool {
        self.block_count == 0
    }

    /// Get the number of blocks currently stored.
    pub const fn block_count(&self) -> usize {
        self.block_count
    }

    /// Get the maximum number of blocks this container can hold.
    pub const fn capacity(&self) -> usize {
        MAX_BLOCKS
    }

    /// Get a specific block by index.
    pub fn get_block(&self, index: usize) -> Option<&[u8; BLOCK_SIZE]> {
        if index < self.block_count {
            self.blocks.get(index)
        } else {
            None
        }
    }

    /// Iterate over all valid blocks.
    pub fn iter_blocks(&self) -> impl Iterator<Item = &[u8; BLOCK_SIZE]> {
        self.blocks[..self.block_count].iter()
    }
}

/// Trait for secure cipher operations and cleanup.
///
/// This trait provides security-focused operations that are orthogonal to basic
/// cipher functionality. It enables secure state management, cleanup, and
/// zeroization without requiring full cipher operation capabilities.
///
/// # Security Operations
///
/// - Secure state clearing and zeroization
/// - Emergency cleanup procedures
/// - Security policy enforcement
/// - Sensitive data lifecycle management
///
/// # Independence from CipherOp
///
/// This trait is deliberately independent of `CipherOp` to allow:
/// - Security managers that don't perform encryption/decryption
/// - Key stores and vaults with secure cleanup
/// - Flexible composition with other cipher traits
pub trait SecureCipherOp: ErrorType {
    /// Securely clear internal state and zeroize sensitive data.
    ///
    /// This method performs a secure cleanup of all internal state, including:
    /// - Zeroization of key material in memory
    /// - Clearing of intermediate computation values
    /// - Resetting hardware registers (for hardware implementations)
    /// - Invalidating cached data or contexts
    ///
    /// # Security Guarantees
    ///
    /// - All sensitive data must be cryptographically erased
    /// - Memory containing keys or intermediate values must be zeroized
    /// - Hardware registers must be cleared if applicable
    /// - The operation should be resistant to compiler optimizations
    ///
    /// # Returns
    ///
    /// A result indicating whether the secure cleanup was successful.
    ///
    /// # Errors
    ///
    /// - `HardwareFailure`: Hardware cleanup operations failed
    /// - `PermissionDenied`: Insufficient privileges for secure operations
    /// - `InvalidState`: Cipher is in a state that prevents secure cleanup
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut cipher = SecureAesCipher::new();
    /// // ... perform cipher operations ...
    /// cipher.clear_state()?; // Secure cleanup before dropping
    /// ```
    fn clear_state(&mut self) -> Result<(), Self::Error>;
}

/// Trait for querying cipher status and hardware state.
///
/// This trait provides status monitoring capabilities that are useful for
/// hardware implementations, performance optimization, and error detection.
/// It's independent of cipher operations to allow status monitoring without
/// requiring operation capabilities.
///
/// # Status Monitoring
///
/// - Hardware readiness and availability
/// - Output data availability
/// - Error and alert conditions
/// - Performance and state information
///
/// # Hardware Integration
///
/// - Allows polling-based operation models
/// - Supports interrupt-driven architectures
/// - Enables efficient resource utilization
/// - Provides visibility into hardware state
pub trait CipherStatus: ErrorType {
    /// Check if the cipher is ready to accept new input data.
    ///
    /// This method indicates whether the cipher implementation can accept
    /// new input for processing. For hardware implementations, this typically
    /// corresponds to input buffer availability.
    ///
    /// # Returns
    ///
    /// - `Ok(true)`: Cipher is ready for new input
    /// - `Ok(false)`: Cipher is busy and cannot accept input
    /// - `Err(_)`: Error occurred while checking status
    ///
    /// # Use Cases
    ///
    /// - Polling loops waiting for hardware readiness
    /// - Flow control in streaming operations
    /// - Performance optimization by avoiding blocking calls
    fn is_ready(&self) -> Result<bool, Self::Error>;

    /// Check if processed output data is available for reading.
    ///
    /// This method indicates whether the cipher has completed processing
    /// and has output data available. For hardware implementations, this
    /// typically corresponds to output buffer status.
    ///
    /// # Returns
    ///
    /// - `Ok(true)`: Output data is available
    /// - `Ok(false)`: No output data is currently available
    /// - `Err(_)`: Error occurred while checking status
    ///
    /// # Use Cases
    ///
    /// - Polling for completion of asynchronous operations
    /// - Avoiding blocking reads when no data is available
    /// - Implementing efficient producer-consumer patterns
    fn has_output(&self) -> Result<bool, Self::Error>;

    /// Check if the cipher is idle and available for new operations.
    ///
    /// This method indicates whether the cipher is in an idle state and
    /// can be used for new operations. This is useful for determining
    /// when to start new transactions or perform maintenance operations.
    ///
    /// # Returns
    ///
    /// - `Ok(true)`: Cipher is idle and available
    /// - `Ok(false)`: Cipher is busy with ongoing operations
    /// - `Err(_)`: Error occurred while checking status
    ///
    /// # Use Cases
    ///
    /// - Determining when to begin new cipher transactions
    /// - Resource management and scheduling
    /// - Power management decisions
    /// - Maintenance and diagnostic operations
    fn is_idle(&self) -> Result<bool, Self::Error>;
}

/// Trait for AEAD (Authenticated Encryption with Associated Data) operations.
///
/// This trait extends symmetric cipher operations to provide authenticated encryption,
/// which combines confidentiality (encryption) with authenticity and integrity
/// (authentication). AEAD modes like AES-GCM and ChaCha20-Poly1305 are the
/// recommended approach for modern cryptographic applications.
///
/// # AEAD Benefits
///
/// - **Confidentiality**: Data is encrypted and unreadable without the key
/// - **Authenticity**: Verifies the data came from the expected sender
/// - **Integrity**: Detects any tampering or corruption of the data
/// - **Associated Data**: Can authenticate additional data without encrypting it
///
/// # Security Guarantees
///
/// - Prevents chosen-ciphertext attacks
/// - Provides semantic security
/// - Detects message tampering
/// - Supports additional authenticated data (AAD) that remains in plaintext
///
/// # Common Algorithms
///
/// - **AES-GCM**: High performance, hardware acceleration available
/// - **ChaCha20-Poly1305**: Software-friendly, constant-time implementation
/// - **AES-CCM**: Suited for resource-constrained environments
///
/// # Example Usage
///
/// ```ignore
/// // Encrypt with associated data
/// let plaintext = b"secret message";
/// let aad = b"public header info";
/// let (ciphertext, tag) = cipher.encrypt_aead(plaintext, aad)?;
///
/// // Decrypt and verify
/// let decrypted = cipher.decrypt_aead(ciphertext, aad, tag)?;
/// ```
pub trait AeadCipherOp: SymmetricCipher + ErrorType {
    /// The associated data type for AEAD operations.
    ///
    /// Associated data (AAD) is additional information that is authenticated
    /// but not encrypted. It provides integrity protection for data that must
    /// remain in plaintext, such as packet headers or metadata.
    ///
    /// # Security Properties
    ///
    /// - **Authenticated but not encrypted**: AAD is included in authentication tag calculation
    /// - **Integrity protected**: Any modification to AAD will cause decryption to fail
    /// - **No confidentiality**: AAD remains visible in plaintext
    ///
    /// # Use Cases
    ///
    /// - Network packet headers that must be readable by intermediary devices
    /// - File metadata that must remain accessible
    /// - Protocol version information
    /// - Sequence numbers or timestamps
    ///
    /// # Common Types
    ///
    /// - `&[u8]` for read-only associated data
    /// - `[u8; N]` for fixed-size owned associated data
    /// - `()` or empty slice if no associated data is needed
    type AssociatedData: FromBytes + IntoBytes;

    /// The authentication tag type for AEAD operations.
    ///
    /// The authentication tag is a cryptographic checksum that provides
    /// integrity and authenticity verification for both the ciphertext
    /// and associated data.
    ///
    /// # Security Properties
    ///
    /// - **Unforgeable**: Cannot be created without the secret key
    /// - **Tamper-evident**: Any modification to protected data changes the tag
    /// - **Algorithm-specific size**: Fixed size determined by the AEAD mode
    ///
    /// # Tag Sizes
    ///
    /// - **AES-GCM**: 16 bytes (128 bits) recommended, can be truncated
    /// - **ChaCha20-Poly1305**: 16 bytes (128 bits) fixed
    /// - **AES-CCM**: Variable (4, 6, 8, 10, 12, 14, or 16 bytes)
    ///
    /// # Security Warning
    ///
    /// Tags must be compared in constant time to prevent timing attacks.
    /// Use cryptographic comparison functions, not standard equality operators.
    ///
    /// # Common Types
    ///
    /// - `[u8; 16]` for most AEAD modes
    /// - `[u8; N]` for variable-length tags
    /// - Custom types that include metadata
    type Tag: FromBytes + IntoBytes;

    /// Encrypts the given plaintext with associated data.
    ///
    /// # Parameters
    ///
    /// - `plaintext`: The data to encrypt.
    /// - `associated_data`: The associated data to authenticate.
    ///
    /// # Returns
    ///
    /// A result containing the ciphertext and authentication tag or an error.
    fn encrypt_aead(
        &mut self,
        plaintext: Self::PlainText,
        associated_data: Self::AssociatedData,
    ) -> Result<(Self::CipherText, Self::Tag), Self::Error>;

    /// Decrypts the given ciphertext with associated data and authentication tag.
    ///
    /// # Parameters
    ///
    /// - `ciphertext`: The data to decrypt.
    /// - `associated_data`: The associated data to authenticate.
    /// - `tag`: The authentication tag.
    ///
    /// # Returns
    ///
    /// A result containing the plaintext or an error.
    fn decrypt_aead(
        &mut self,
        ciphertext: Self::CipherText,
        associated_data: Self::AssociatedData,
        tag: Self::Tag,
    ) -> Result<Self::PlainText, Self::Error>;
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // Allow unwrap in tests for cleaner test code
mod tests {
    use super::*;

    #[test]
    fn test_block_aligned_creation() {
        let container = BlockAligned::<16, 4>::new();
        assert_eq!(container.block_count(), 0);
        assert_eq!(container.capacity(), 4);
        assert_eq!(container.len(), 0);
        assert!(container.is_empty());
    }

    #[test]
    fn test_block_aligned_default() {
        let container: BlockAligned<16, 4> = Default::default();
        assert_eq!(container.block_count(), 0);
        assert_eq!(container.capacity(), 4);
        assert!(container.is_empty());
    }

    #[test]
    fn test_push_block_success() {
        let mut container = BlockAligned::<16, 4>::new();

        let block1 = [0x42u8; 16];
        let result = container.push_block(block1);
        assert!(result.is_ok());
        assert_eq!(container.block_count(), 1);
        assert_eq!(container.len(), 16);
        assert!(!container.is_empty());

        let block2 = [0x33u8; 16];
        let result = container.push_block(block2);
        assert!(result.is_ok());
        assert_eq!(container.block_count(), 2);
        assert_eq!(container.len(), 32);
    }

    #[test]
    fn test_push_block_capacity_exceeded() {
        let mut container = BlockAligned::<16, 2>::new();

        // Fill to capacity
        container.push_block([0x01u8; 16]).unwrap();
        container.push_block([0x02u8; 16]).unwrap();
        assert_eq!(container.block_count(), 2);

        // Try to exceed capacity
        let result = container.push_block([0x03u8; 16]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), BlockAlignedError::CapacityExceeded);
        assert_eq!(container.block_count(), 2); // Should remain unchanged
    }

    #[test]
    fn test_get_block() {
        let mut container = BlockAligned::<16, 4>::new();

        let block1 = [0x42u8; 16];
        let block2 = [0x33u8; 16];
        container.push_block(block1).unwrap();
        container.push_block(block2).unwrap();

        // Test valid indices
        assert_eq!(container.get_block(0), Some(&block1));
        assert_eq!(container.get_block(1), Some(&block2));

        // Test invalid indices
        assert_eq!(container.get_block(2), None);
        assert_eq!(container.get_block(100), None);
    }

    #[test]
    fn test_blocks_slice() {
        let mut container = BlockAligned::<16, 4>::new();

        let block1 = [0x42u8; 16];
        let block2 = [0x33u8; 16];
        container.push_block(block1).unwrap();
        container.push_block(block2).unwrap();

        let blocks = container.blocks();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks.first().unwrap(), &block1);
        assert_eq!(blocks.get(1).unwrap(), &block2);
    }

    #[test]
    fn test_iter_blocks() {
        let mut container = BlockAligned::<16, 4>::new();

        let block1 = [0x42u8; 16];
        let block2 = [0x33u8; 16];
        let block3 = [0x11u8; 16];

        container.push_block(block1).unwrap();
        container.push_block(block2).unwrap();
        container.push_block(block3).unwrap();

        // Test iterator manually
        let mut iter = container.iter_blocks();
        assert_eq!(iter.next(), Some(&block1));
        assert_eq!(iter.next(), Some(&block2));
        assert_eq!(iter.next(), Some(&block3));
        assert_eq!(iter.next(), None);

        // Test that iterator only includes valid blocks
        let empty_container = BlockAligned::<16, 4>::new();
        let mut empty_iter = empty_container.iter_blocks();
        assert_eq!(empty_iter.next(), None);
    }

    #[test]
    fn test_from_slice_padded_exact_fit() {
        // Test data that exactly fits one block
        let data = [0x42u8; 16];
        let container = BlockAligned::<16, 4>::from_slice_padded(&data, 0x00).unwrap();

        assert_eq!(container.block_count(), 1);
        assert_eq!(container.get_block(0), Some(&data));
    }

    #[test]
    fn test_from_slice_padded_partial_block() {
        // Test data that requires padding
        let data = b"Hello, World!"; // 13 bytes
        let container = BlockAligned::<16, 4>::from_slice_padded(data, 0x00).unwrap();

        assert_eq!(container.block_count(), 1);

        let block = container.get_block(0).unwrap();
        // First 13 bytes should match input
        assert_eq!(&block[..13], data);
        // Last 3 bytes should be padding
        assert_eq!(&block[13..], &[0x00; 3]);
    }

    #[test]
    fn test_from_slice_padded_multiple_blocks() {
        // Test data that spans multiple blocks
        let data = [0x42u8; 33]; // 33 bytes = 3 blocks (16 + 16 + 1)
        let container = BlockAligned::<16, 4>::from_slice_padded(&data, 0xFF).unwrap();

        assert_eq!(container.block_count(), 3);

        // First two blocks should be all 0x42
        assert_eq!(container.get_block(0), Some(&[0x42u8; 16]));
        assert_eq!(container.get_block(1), Some(&[0x42u8; 16]));

        // Third block should have one byte of data and 15 bytes of padding
        let third_block = container.get_block(2).unwrap();
        assert_eq!(third_block.first().unwrap(), &0x42);
        assert_eq!(&third_block[1..], &[0xFF; 15]);
    }

    #[test]
    fn test_from_slice_padded_empty_data() {
        let data = &[];
        let container = BlockAligned::<16, 4>::from_slice_padded(data, 0x00).unwrap();

        assert_eq!(container.block_count(), 0);
        assert!(container.is_empty());
    }

    #[test]
    fn test_from_slice_padded_data_too_large() {
        // Test data that exceeds capacity
        let data = [0x42u8; 100]; // 100 bytes = 7 blocks (16*6 + 4), but capacity is only 4
        let result = BlockAligned::<16, 4>::from_slice_padded(&data, 0x00);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), BlockAlignedError::DataTooLarge);
    }

    #[test]
    fn test_from_slice_padded_different_padding() {
        let data = b"test"; // 4 bytes
        let container = BlockAligned::<8, 2>::from_slice_padded(data, 0xAA).unwrap();

        assert_eq!(container.block_count(), 1);

        let block = container.get_block(0).unwrap();
        assert_eq!(&block[..4], data);
        assert_eq!(&block[4..], &[0xAA; 4]);
    }

    #[test]
    fn test_different_block_sizes() {
        // Test with 8-byte blocks
        let mut container8 = BlockAligned::<8, 4>::new();
        container8.push_block([0x11u8; 8]).unwrap();
        assert_eq!(container8.len(), 8);

        // Test with 32-byte blocks
        let mut container32 = BlockAligned::<32, 2>::new();
        container32.push_block([0x22u8; 32]).unwrap();
        assert_eq!(container32.len(), 32);

        // Test with 1-byte blocks
        let mut container1 = BlockAligned::<1, 16>::new();
        container1.push_block([0x33]).unwrap();
        assert_eq!(container1.len(), 1);
    }

    #[test]
    fn test_clone_and_equality() {
        let mut container1 = BlockAligned::<16, 4>::new();
        let block = [0x42u8; 16];
        container1.push_block(block).unwrap();

        let container2 = container1.clone();
        assert_eq!(container1, container2);
        assert_eq!(container1.block_count(), container2.block_count());
        assert_eq!(container1.get_block(0), container2.get_block(0));

        // Test inequality
        let mut container3 = BlockAligned::<16, 4>::new();
        container3.push_block([0x33u8; 16]).unwrap();
        assert_ne!(container1, container3);
    }

    #[test]
    fn test_edge_cases() {
        // Test with maximum capacity
        let mut container = BlockAligned::<1, 8>::new();
        for i in 0..8 {
            container.push_block([i as u8]).unwrap();
        }
        assert_eq!(container.block_count(), 8);
        assert_eq!(container.len(), 8);

        // Verify all blocks are correct
        for i in 0..8 {
            assert_eq!(container.get_block(i), Some(&[i as u8]));
        }
    }

    #[test]
    fn test_error_display() {
        let capacity_error = BlockAlignedError::CapacityExceeded;
        let data_error = BlockAlignedError::DataTooLarge;

        // Test that the errors are created correctly
        assert_eq!(capacity_error, BlockAlignedError::CapacityExceeded);
        assert_eq!(data_error, BlockAlignedError::DataTooLarge);

        // Test that they are not equal to each other
        assert_ne!(capacity_error, data_error);
    }

    #[test]
    fn test_debug_formatting() {
        let mut container = BlockAligned::<4, 2>::new();
        container.push_block([1, 2, 3, 4]).unwrap();

        // Test that the container was created successfully
        assert_eq!(container.block_count(), 1);
        assert_eq!(container.get_block(0), Some(&[1, 2, 3, 4]));
    }
}

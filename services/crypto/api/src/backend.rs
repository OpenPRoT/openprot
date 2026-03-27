// Licensed under the Apache-2.0 license

//! Crypto Service Backend Traits
//!
//! Defines the abstraction layer between the crypto server and its pluggable
//! backends (RustCrypto, ASPEED HACE, etc.).
//!
//! # Architecture
//!
//! ```text
//!                    ┌──────────────────────────────┐
//!                    │         crypto-api            │
//!                    │  protocol (wire format)       │
//!                    │  backend  (Algorithm, OneShot)│
//!                    └───────────┬──────────────────┘
//!                                │
//!              ┌─────────────────┴─────────────────┐
//!              │                                   │
//!    ┌─────────▼──────┐                 ┌──────────▼─────────┐
//!    │  RustCrypto    │                 │  ASPEED HACE       │
//!    │  Backend       │                 │  Backend           │
//!    │ impl OneShot   │                 │ impl OneShot       │
//!    │   <Sha256> ..  │                 │   <Sha256> ..      │
//!    └────────────────┘                 └────────────────────┘
//! ```
//!
//! # Adding a new algorithm
//!
//! 1. Add a variant to [`CryptoOp`](crate::CryptoOp) in `protocol.rs`.
//! 2. Define a marker type here: `pub struct Blake3;`
//! 3. Implement [`Algorithm`] for it, returning the `CryptoOp` variant.
//! 4. Add a [`CryptoInput`] variant if the input shape is new.
//! 5. Implement `OneShot<Blake3>` on each backend.
//!
//! The server dispatch table gains one line — no other changes needed.

use crate::protocol::CryptoOp;

// ---------------------------------------------------------------------------
// Algorithm marker trait
// ---------------------------------------------------------------------------

/// Marker trait for cryptographic algorithms.
///
/// Each algorithm is a zero-sized type (ZST) that carries compile-time
/// metadata. The server uses [`Self::OP`] for dispatch; backends use the
/// marker as a type parameter for [`OneShot<A>`] / [`Streaming<A>`].
pub trait Algorithm {
    /// Size of the primary output in bytes.
    ///
    /// - Digest: hash length (32, 48, 64)
    /// - HMAC: tag length (32, 48, 64)
    /// - AEAD encrypt: 0 (output size = input_len + tag)
    /// - ECDSA sign: signature length (64, 96)
    /// - ECDSA verify: 1 (boolean result)
    const OUTPUT_SIZE: usize;

    /// The wire protocol operation code.
    ///
    /// This is the single source of truth — no duplicate `const OP_CODE: u8`
    /// that can drift out of sync with the protocol enum.
    const OP: CryptoOp;
}

// ---------------------------------------------------------------------------
// Digest algorithm markers
// ---------------------------------------------------------------------------

/// SHA-256 hash (32-byte output)
pub struct Sha256;
impl Algorithm for Sha256 {
    const OUTPUT_SIZE: usize = 32;
    const OP: CryptoOp = CryptoOp::Sha256Hash;
}

/// SHA-384 hash (48-byte output)
pub struct Sha384;
impl Algorithm for Sha384 {
    const OUTPUT_SIZE: usize = 48;
    const OP: CryptoOp = CryptoOp::Sha384Hash;
}

/// SHA-512 hash (64-byte output)
pub struct Sha512;
impl Algorithm for Sha512 {
    const OUTPUT_SIZE: usize = 64;
    const OP: CryptoOp = CryptoOp::Sha512Hash;
}

// ---------------------------------------------------------------------------
// MAC algorithm markers
// ---------------------------------------------------------------------------

/// HMAC-SHA-256 (32-byte tag)
pub struct HmacSha256;
impl Algorithm for HmacSha256 {
    const OUTPUT_SIZE: usize = 32;
    const OP: CryptoOp = CryptoOp::HmacSha256;
}

/// HMAC-SHA-384 (48-byte tag)
pub struct HmacSha384;
impl Algorithm for HmacSha384 {
    const OUTPUT_SIZE: usize = 48;
    const OP: CryptoOp = CryptoOp::HmacSha384;
}

/// HMAC-SHA-512 (64-byte tag)
pub struct HmacSha512;
impl Algorithm for HmacSha512 {
    const OUTPUT_SIZE: usize = 64;
    const OP: CryptoOp = CryptoOp::HmacSha512;
}

// ---------------------------------------------------------------------------
// AEAD algorithm markers
// ---------------------------------------------------------------------------

/// AES-256-GCM authenticated encryption
///
/// Output size is data-dependent: `plaintext_len + 16` (tag appended).
pub struct Aes256GcmEncrypt;
impl Algorithm for Aes256GcmEncrypt {
    const OUTPUT_SIZE: usize = 0; // variable
    const OP: CryptoOp = CryptoOp::Aes256GcmEncrypt;
}

/// AES-256-GCM authenticated decryption
///
/// Output size is data-dependent: `ciphertext_len - 16` (tag stripped).
pub struct Aes256GcmDecrypt;
impl Algorithm for Aes256GcmDecrypt {
    const OUTPUT_SIZE: usize = 0; // variable
    const OP: CryptoOp = CryptoOp::Aes256GcmDecrypt;
}

// ---------------------------------------------------------------------------
// Signature algorithm markers
// ---------------------------------------------------------------------------

/// ECDSA P-256 signing (64-byte fixed signature)
#[cfg(feature = "ecdsa")]
pub struct EcdsaP256Sign;
#[cfg(feature = "ecdsa")]
impl Algorithm for EcdsaP256Sign {
    const OUTPUT_SIZE: usize = 64;
    const OP: CryptoOp = CryptoOp::EcdsaP256Sign;
}

/// ECDSA P-256 verification (1-byte result: 0x01 = valid)
#[cfg(feature = "ecdsa")]
pub struct EcdsaP256Verify;
#[cfg(feature = "ecdsa")]
impl Algorithm for EcdsaP256Verify {
    const OUTPUT_SIZE: usize = 1;
    const OP: CryptoOp = CryptoOp::EcdsaP256Verify;
}

/// ECDSA P-384 signing (96-byte fixed signature)
#[cfg(feature = "ecdsa")]
pub struct EcdsaP384Sign;
#[cfg(feature = "ecdsa")]
impl Algorithm for EcdsaP384Sign {
    const OUTPUT_SIZE: usize = 96;
    const OP: CryptoOp = CryptoOp::EcdsaP384Sign;
}

/// ECDSA P-384 verification (1-byte result: 0x01 = valid)
#[cfg(feature = "ecdsa")]
pub struct EcdsaP384Verify;
#[cfg(feature = "ecdsa")]
impl Algorithm for EcdsaP384Verify {
    const OUTPUT_SIZE: usize = 1;
    const OP: CryptoOp = CryptoOp::EcdsaP384Verify;
}

/// Get random bytes (variable-length output)
pub struct GetRandomBytes;
impl Algorithm for GetRandomBytes {
    const OUTPUT_SIZE: usize = 0;  // Variable-length
    const OP: CryptoOp = CryptoOp::GetRandomBytes;
}

// ---------------------------------------------------------------------------
// Structured input type
// ---------------------------------------------------------------------------

/// Semantically typed crypto input.
///
/// Each variant carries exactly the fields its operation class requires —
/// no more stuffing signatures into "nonce" or guessing which byte ranges
/// of a flat buffer mean what.
///
/// The server constructs this from the parsed wire format via
/// [`CryptoInput::from_wire`]; the backend pattern-matches on it.
#[derive(Debug)]
pub enum CryptoInput<'a> {
    /// Hash operations (SHA-256/384/512): just the message data.
    Digest { data: &'a [u8] },

    /// MAC operations (HMAC-SHA-256/384/512): key + message data.
    Mac { key: &'a [u8], data: &'a [u8] },

    /// AEAD operations (AES-GCM): key + nonce + plaintext/ciphertext.
    /// For decrypt: `data = ciphertext || tag` (tag appended).
    Aead {
        key: &'a [u8],
        nonce: &'a [u8],
        data: &'a [u8],
    },

    /// Signing: private key + message.
    #[cfg(feature = "ecdsa")]
    Sign {
        private_key: &'a [u8],
        message: &'a [u8],
    },

    /// Verification: public key + message + signature.
    #[cfg(feature = "ecdsa")]
    Verify {
        public_key: &'a [u8],
        message: &'a [u8],
        signature: &'a [u8],
    },

    /// RNG operation: generate random bytes of specified length.
    Rng { length: usize },
}

impl<'a> CryptoInput<'a> {
    /// Construct from parsed wire format fields.
    ///
    /// This is the **only** place that maps the flat `key || nonce || data`
    /// wire encoding to semantically typed variants. The backend never
    /// sees raw wire bytes.
    pub fn from_wire(op: CryptoOp, key: &'a [u8], nonce: &'a [u8], data: &'a [u8]) -> Self {
        match op {
            CryptoOp::Sha256Hash | CryptoOp::Sha384Hash | CryptoOp::Sha512Hash |
            CryptoOp::Sha256Begin | CryptoOp::Sha256Update | CryptoOp::Sha256Finish |
            CryptoOp::Sha384Begin | CryptoOp::Sha384Update | CryptoOp::Sha384Finish |
            CryptoOp::Sha512Begin | CryptoOp::Sha512Update | CryptoOp::Sha512Finish => {
                CryptoInput::Digest { data }
            }
            CryptoOp::HmacSha256 | CryptoOp::HmacSha384 | CryptoOp::HmacSha512 => {
                CryptoInput::Mac { key, data }
            }
            CryptoOp::Aes256GcmEncrypt | CryptoOp::Aes256GcmDecrypt => {
                CryptoInput::Aead { key, nonce, data }
            }
            CryptoOp::GetRandomBytes => {
                // Length encoded in data_len field (already parsed in header)
                CryptoInput::Rng { length: data.len() }
            }
            #[cfg(feature = "ecdsa")]
            CryptoOp::EcdsaP256Sign | CryptoOp::EcdsaP384Sign => CryptoInput::Sign {
                private_key: key,
                message: data,
            },
            #[cfg(feature = "ecdsa")]
            CryptoOp::EcdsaP256Verify | CryptoOp::EcdsaP384Verify => CryptoInput::Verify {
                public_key: key,
                message: data,
                signature: nonce,
            },
            // When ECDSA feature is off, the enum variants still exist
            // (wire protocol is stable) but should never reach from_wire —
            // the server dispatch rejects them first.
            #[cfg(not(feature = "ecdsa"))]
            CryptoOp::EcdsaP256Sign | CryptoOp::EcdsaP256Verify |
            CryptoOp::EcdsaP384Sign | CryptoOp::EcdsaP384Verify => {
                panic!("ECDSA operations require the 'ecdsa' feature")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Backend error type
// ---------------------------------------------------------------------------

/// Backend crypto error.
///
/// Domain error type for backend operations — distinct from the wire
/// protocol's [`CryptoError`](crate::CryptoError) which is `repr(u8)`
/// for serialization. The server maps between them at the IPC boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendError {
    /// The operation code doesn't match the input variant.
    InvalidOperation,

    /// Key length is wrong for the algorithm.
    InvalidKeyLength,

    /// Nonce/IV length is wrong for the algorithm.
    InvalidNonceLength,

    /// Input data length is invalid or exceeds limits.
    InvalidDataLength,

    /// Output buffer is too small for the result.
    BufferTooSmall,

    /// AEAD authentication tag verification failed.
    AuthenticationFailed,

    /// Signing operation failed (e.g., invalid private key).
    SigningFailed,

    /// Signature verification failed.
    VerificationFailed,

    /// Unspecified backend failure.
    InternalError,
}

impl core::fmt::Display for BackendError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidOperation => write!(f, "invalid operation for input type"),
            Self::InvalidKeyLength => write!(f, "invalid key length"),
            Self::InvalidNonceLength => write!(f, "invalid nonce/IV length"),
            Self::InvalidDataLength => write!(f, "invalid data length"),
            Self::BufferTooSmall => write!(f, "output buffer too small"),
            Self::AuthenticationFailed => write!(f, "authentication failed"),
            Self::SigningFailed => write!(f, "signing failed"),
            Self::VerificationFailed => write!(f, "verification failed"),
            Self::InternalError => write!(f, "internal backend error"),
        }
    }
}

// ---------------------------------------------------------------------------
// Error abstraction traits
// ---------------------------------------------------------------------------

/// Trait for backend error types.
///
/// This trait provides a standard interface for all error types used in
/// crypto backend operations. It requires implementors to provide a mapping
/// to the common [`BackendError`] enumeration, enabling generic error handling
/// while preserving implementation-specific error details.
///
/// # Design
///
/// By using this pattern, backend implementations can define rich, context-specific
/// error types (e.g., containing hardware register values or debug info) while
/// still mapping them to common error kinds that the server can convert to wire
/// protocol errors.
///
/// # Example
///
/// ```ignore
/// #[derive(Debug)]
/// struct HaceError {
///     kind: BackendError,
///     register_status: u32,  // Hardware-specific debug info
/// }
///
/// impl Error for HaceError {
///     fn kind(&self) -> BackendError {
///         self.kind
///     }
/// }
/// ```
pub trait Error: core::fmt::Debug {
    /// Convert error to a generic backend error kind.
    ///
    /// By using this method, errors freely defined by backend implementations
    /// can be converted to a set of generic errors upon which the server
    /// can act and convert to wire protocol errors.
    fn kind(&self) -> BackendError;
}

impl Error for BackendError {
    /// BackendError trivially maps to itself.
    fn kind(&self) -> BackendError {
        *self
    }
}

impl Error for core::convert::Infallible {
    /// Since `core::convert::Infallible` represents an error that can never occur,
    /// this implementation uses pattern matching on the uninhabited type to
    /// ensure this method can never actually be called.
    fn kind(&self) -> BackendError {
        match *self {}
    }
}

/// Trait providing access to the associated error type.
///
/// This trait serves as a foundation for other traits that need to define
/// error handling. By separating error type definition from specific operations,
/// it enables composition and reuse across different trait implementations.
///
/// # Example
///
/// ```ignore
/// struct HaceBackend { /* ... */ }
///
/// impl ErrorType for HaceBackend {
///     type Error = HaceError;  // Rich error with hardware debug info
/// }
///
/// impl OneShot<Sha256> for HaceBackend {
///     // Can return HaceError, server converts via Error::kind()
/// }
/// ```
pub trait ErrorType {
    /// The error type used by this implementation.
    ///
    /// This associated type must implement the [`Error`] trait to ensure
    /// it can be converted to generic error kinds for interoperability
    /// with the server's error handling.
    type Error: Error;
}

/// Convert backend errors to wire protocol errors for IPC responses.
impl From<BackendError> for crate::CryptoError {
    fn from(e: BackendError) -> Self {
        match e {
            BackendError::InvalidOperation => crate::CryptoError::InvalidOperation,
            BackendError::InvalidKeyLength => crate::CryptoError::InvalidKeyLength,
            BackendError::InvalidNonceLength => crate::CryptoError::InvalidNonceLength,
            BackendError::InvalidDataLength => crate::CryptoError::InvalidDataLength,
            BackendError::BufferTooSmall => crate::CryptoError::BufferTooSmall,
            BackendError::AuthenticationFailed => crate::CryptoError::AuthenticationFailed,
            BackendError::SigningFailed => crate::CryptoError::SigningFailed,
            BackendError::VerificationFailed => crate::CryptoError::VerificationFailed,
            BackendError::InternalError => crate::CryptoError::InternalError,
        }
    }
}

// ---------------------------------------------------------------------------
// Backend traits
// ---------------------------------------------------------------------------

/// One-shot crypto operation trait.
///
/// One impl per `(Backend, Algorithm)` pair. The server dispatches to the
/// correct monomorphized `compute()` via the algorithm marker type.
///
/// `&self` (not consumed): software backends are stateless. Hardware
/// backends that need exclusive access should use internal `RefCell`
/// or be wrapped in `Option<HwController>` at the server level.
///
/// # Design Trade-off: `&mut [u8]` vs Typed Output
///
/// The `output: &mut [u8]` parameter accepts arbitrarily-sized buffers,
/// requiring **runtime validation** rather than compile-time guarantees:
///
/// | Approach | Type Safety | Flexibility | Constraint |
/// |----------|-------------|-------------|------------|
/// | `&mut [u8]` (current) | Runtime check | ✅ Variable outputs (AEAD) | N/A |
/// | `&mut [u8; A::OUTPUT_SIZE]` | Compile-time | ❌ Fixed only | Rust limitation¹ |
/// | Return `[u8; A::OUTPUT_SIZE]` | Compile-time | ❌ Fixed only | Rust limitation¹ |
///
/// ¹ Rust does not support `[u8; A::OUTPUT_SIZE]` where `OUTPUT_SIZE` is an
///   associated const used in a trait method signature.
///
/// **Why `&mut [u8]` was chosen:**
///
/// 1. **AEAD support**: AES-GCM output size = `data.len() + 16`, not compile-time known.
/// 2. **Zero-copy IPC**: Server can pass the response buffer directly — no intermediate copy.
/// 3. **Uniform API**: Same signature for all algorithms simplifies dispatch.
///
/// **Mitigations:**
///
/// - Implementations **must** check `output.len() >= A::OUTPUT_SIZE` and return
///   `BackendError::BufferTooSmall` if insufficient.
/// - The server allocates response buffers based on `A::OUTPUT_SIZE`, so this
///   error path rarely fires in production.
/// - The return value `usize` indicates actual bytes written, enabling callers
///   to slice the buffer correctly.
///
/// # Example
///
/// ```ignore
/// impl OneShot<Sha256> for RustCryptoBackend {
///     fn compute(&self, input: &CryptoInput, output: &mut [u8]) -> Result<usize, BackendError> {
///         let CryptoInput::Digest { data } = input else {
///             return Err(BackendError::InvalidOperation);
///         };
///         // Runtime check — required for safety
///         if output.len() < Sha256::OUTPUT_SIZE {
///             return Err(BackendError::BufferTooSmall);
///         }
///         let hash = sha2::Sha256::digest(data);
///         output[..32].copy_from_slice(&hash);
///         Ok(32)
///     }
/// }
/// ```
pub trait OneShot<A: Algorithm> {
    /// Execute a one-shot crypto operation.
    ///
    /// # Parameters
    ///
    /// - `input`: Structured crypto input matching the algorithm class.
    /// - `output`: Mutable buffer for the result.
    ///
    /// # Buffer Requirements
    ///
    /// The caller must provide a buffer of sufficient size:
    /// - **Fixed-output algorithms** (digest, MAC, signatures): `>= A::OUTPUT_SIZE` bytes
    /// - **AEAD encrypt**: `>= data.len() + 16` bytes (ciphertext + tag)
    /// - **AEAD decrypt**: `>= data.len() - 16` bytes (plaintext)
    ///
    /// If the buffer is too small, implementations must return `BackendError::BufferTooSmall`.
    ///
    /// # Returns
    ///
    /// `Ok(n)` where `n` is the number of bytes written to `output`, or an error.
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError>;
}

/// Session-based streaming crypto trait.
///
/// For processing data larger than a single IPC buffer (e.g., hashing a
/// firmware image in 1KB chunks). Optional — backends only implement this
/// for algorithms that benefit from streaming.
///
/// # Wire protocol integration
///
/// The request header `flags` byte encodes session semantics:
/// ```text
/// bit 0:    0 = one-shot, 1 = session operation
/// bits 1-2: 00 = begin, 01 = feed, 10 = finish, 11 = cancel
/// bits 3-7: reserved
/// ```
pub trait Streaming<A: Algorithm> {
    type Session;

    /// Begin a new streaming session.
    fn begin(&mut self) -> Result<Self::Session, BackendError>;

    /// Feed data into an active session.
    fn feed(&mut self, session: &mut Self::Session, data: &[u8]) -> Result<(), BackendError>;

    /// Finalize the session and write the result to `output`.
    ///
    /// Returns the number of bytes written. Consumes the session.
    fn finish(&mut self, session: Self::Session, output: &mut [u8])
        -> Result<usize, BackendError>;

    /// Cancel an active session without producing output.
    fn cancel(&mut self, session: Self::Session);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algorithm_output_sizes() {
        assert_eq!(Sha256::OUTPUT_SIZE, 32);
        assert_eq!(Sha384::OUTPUT_SIZE, 48);
        assert_eq!(Sha512::OUTPUT_SIZE, 64);
        assert_eq!(HmacSha256::OUTPUT_SIZE, 32);
        assert_eq!(HmacSha384::OUTPUT_SIZE, 48);
        assert_eq!(HmacSha512::OUTPUT_SIZE, 64);
        #[cfg(feature = "ecdsa")]
        {
            assert_eq!(EcdsaP256Sign::OUTPUT_SIZE, 64);
            assert_eq!(EcdsaP256Verify::OUTPUT_SIZE, 1);
            assert_eq!(EcdsaP384Sign::OUTPUT_SIZE, 96);
            assert_eq!(EcdsaP384Verify::OUTPUT_SIZE, 1);
        }
    }

    #[test]
    fn algorithm_ops_match_protocol() {
        // Algorithm::OP is CryptoOp — type-safe, can't drift
        assert_eq!(Sha256::OP, CryptoOp::Sha256Hash);
        assert_eq!(Sha384::OP, CryptoOp::Sha384Hash);
        assert_eq!(Sha512::OP, CryptoOp::Sha512Hash);
        assert_eq!(HmacSha256::OP, CryptoOp::HmacSha256);
        assert_eq!(HmacSha384::OP, CryptoOp::HmacSha384);
        assert_eq!(HmacSha512::OP, CryptoOp::HmacSha512);
        assert_eq!(Aes256GcmEncrypt::OP, CryptoOp::Aes256GcmEncrypt);
        assert_eq!(Aes256GcmDecrypt::OP, CryptoOp::Aes256GcmDecrypt);
        #[cfg(feature = "ecdsa")]
        {
            assert_eq!(EcdsaP256Sign::OP, CryptoOp::EcdsaP256Sign);
            assert_eq!(EcdsaP256Verify::OP, CryptoOp::EcdsaP256Verify);
            assert_eq!(EcdsaP384Sign::OP, CryptoOp::EcdsaP384Sign);
            assert_eq!(EcdsaP384Verify::OP, CryptoOp::EcdsaP384Verify);
        }
    }

    #[test]
    fn crypto_input_from_wire() {
        let key = b"secret";
        let nonce = b"123456789012";
        let data = b"hello";

        // Digest
        let input = CryptoInput::from_wire(CryptoOp::Sha256Hash, &[], &[], data);
        assert!(matches!(input, CryptoInput::Digest { .. }));

        // MAC
        let input = CryptoInput::from_wire(CryptoOp::HmacSha256, key, &[], data);
        assert!(matches!(input, CryptoInput::Mac { .. }));

        // AEAD
        let input = CryptoInput::from_wire(CryptoOp::Aes256GcmEncrypt, key, nonce, data);
        assert!(matches!(input, CryptoInput::Aead { .. }));

        // Sign
        #[cfg(feature = "ecdsa")]
        {
            let input = CryptoInput::from_wire(CryptoOp::EcdsaP256Sign, key, &[], data);
            assert!(matches!(input, CryptoInput::Sign { .. }));
        }

        // Verify
        #[cfg(feature = "ecdsa")]
        {
            let input = CryptoInput::from_wire(CryptoOp::EcdsaP256Verify, key, nonce, data);
            assert!(matches!(input, CryptoInput::Verify { .. }));
        }
    }

    #[test]
    fn backend_error_to_wire_error() {
        use crate::CryptoError;
        let wire: CryptoError = BackendError::InvalidKeyLength.into();
        assert_eq!(wire, CryptoError::InvalidKeyLength);

        let wire: CryptoError = BackendError::AuthenticationFailed.into();
        assert_eq!(wire, CryptoError::AuthenticationFailed);
    }

    #[test]
    fn backend_error_variants_are_distinct() {
        let variants = [
            BackendError::InvalidOperation,
            BackendError::InvalidKeyLength,
            BackendError::InvalidNonceLength,
            BackendError::InvalidDataLength,
            BackendError::BufferTooSmall,
            BackendError::AuthenticationFailed,
            BackendError::SigningFailed,
            BackendError::VerificationFailed,
            BackendError::InternalError,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    // Compile-time verification that a dummy backend can implement OneShot
    struct DummyBackend;

    impl OneShot<Sha256> for DummyBackend {
        fn compute(
            &self,
            input: &CryptoInput<'_>,
            output: &mut [u8],
        ) -> Result<usize, BackendError> {
            let CryptoInput::Digest { data: _ } = input else {
                return Err(BackendError::InvalidOperation);
            };
            if output.len() < Sha256::OUTPUT_SIZE {
                return Err(BackendError::BufferTooSmall);
            }
            output[..32].fill(0xAA);
            Ok(32)
        }
    }

    #[test]
    fn dummy_backend_oneshot() {
        let backend = DummyBackend;
        let input = CryptoInput::Digest { data: b"test" };
        let mut output = [0u8; 64];
        let len = backend.compute(&input, &mut output).unwrap();
        assert_eq!(len, 32);
        assert_eq!(&output[..32], &[0xAA; 32]);
    }

    #[test]
    fn dummy_backend_wrong_input_variant() {
        let backend = DummyBackend;
        let input = CryptoInput::Mac {
            key: b"key",
            data: b"data",
        };
        let mut output = [0u8; 64];
        let result = backend.compute(&input, &mut output);
        assert_eq!(result, Err(BackendError::InvalidOperation));
    }

    #[test]
    fn dummy_backend_buffer_too_small() {
        let backend = DummyBackend;
        let input = CryptoInput::Digest { data: b"test" };
        let mut output = [0u8; 16];
        let result = backend.compute(&input, &mut output);
        assert_eq!(result, Err(BackendError::BufferTooSmall));
    }
}

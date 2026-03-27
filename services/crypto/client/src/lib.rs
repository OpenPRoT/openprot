// Licensed under the Apache-2.0 license

//! Crypto Client Library
//!
//! Provides an ergonomic API for applications to call the crypto server over IPC.
//! The server implements cryptographic operations using RustCrypto crates.
//!
//! # Supported Algorithms
//!
//! | Category | Algorithm | Output |
//! |----------|-----------|--------|
//! | Hash | SHA-256, SHA-384, SHA-512 | 32/48/64 bytes |
//! | MAC | HMAC-SHA256, HMAC-SHA384, HMAC-SHA512 | 32/48/64 bytes |
//! | AEAD | AES-256-GCM | 16-byte tag |
//! | Signature | ECDSA P-256, P-384 (feature-gated) | 64/96 bytes |
//!
//! # Quick Start
//!
//! ```ignore
//! use crypto_client::CryptoClient;
//!
//! let crypto = CryptoClient::new(handle::CRYPTO);
//!
//! // Hash — returns fixed-size array directly
//! let hash: [u8; 32] = crypto.sha256(b"hello world")?;
//!
//! // HMAC — returns authentication tag
//! let tag: [u8; 32] = crypto.hmac_sha256(key, data)?;
//!
//! // AEAD — seal encrypts + authenticates, open decrypts + verifies
//! let ct_len = crypto.aes256_gcm_seal(&key, &nonce, plaintext, &mut ct)?;
//! let pt_len = crypto.aes256_gcm_open(&key, &nonce, &ct[..ct_len], &mut pt)?;
//!
//! // ECDSA (requires "ecdsa" feature)
//! let sig: [u8; 64] = crypto.ecdsa_p256_sign(&private_key, message)?;
//! crypto.ecdsa_p256_verify(&public_key, message, &sig)?; // Ok(()) = valid
//! ```
//!
//! # Error Handling
//!
//! All operations return `Result<T, ClientError>`. The [`ClientError`] type
//! distinguishes IPC failures from cryptographic errors and implements
//! [`Display`](core::fmt::Display) for logging.
//!
//! # Free Functions
//!
//! For compatibility, free functions are also exported (e.g., `sha256(handle, data)`).
//! Prefer [`CryptoClient`] for new code.

#![no_std]

use crypto_api::{
    CryptoError, CryptoOp, CryptoRequestHeader, CryptoResponseHeader,
    MAX_PAYLOAD_SIZE,
};
#[cfg(feature = "ecdsa")]
use crypto_api::{ECDSA_P256_SIGNATURE_SIZE, ECDSA_P384_SIGNATURE_SIZE};
use userspace::syscall;
use userspace::time::Instant;

/// Maximum buffer size for requests/responses
const MAX_BUF_SIZE: usize = 1024;

/// Error type for crypto client operations.
///
/// Distinguishes between IPC-level failures and cryptographic errors,
/// enabling appropriate error handling strategies.
///
/// # Variants
///
/// - [`IpcError`](Self::IpcError) — Channel communication failed (retry may help)
/// - [`ServerError`](Self::ServerError) — Cryptographic operation failed (check inputs)
/// - [`InvalidResponse`](Self::InvalidResponse) — Protocol mismatch (likely a bug)
/// - [`BufferTooSmall`](Self::BufferTooSmall) — Output buffer insufficient
/// - [`VerificationFailed`](Self::VerificationFailed) — Signature or MAC invalid
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientError {
    /// IPC syscall failed (channel closed, timeout, etc.)
    IpcError(pw_status::Error),
    /// Server returned an error (invalid key, bad nonce, etc.)
    ServerError(CryptoError),
    /// Response was malformed (internal error)
    InvalidResponse,
    /// Buffer too small for the requested operation
    BufferTooSmall,
    /// Signature or authentication tag verification failed
    VerificationFailed,
}

impl From<pw_status::Error> for ClientError {
    fn from(e: pw_status::Error) -> Self {
        ClientError::IpcError(e)
    }
}

impl core::fmt::Display for ClientError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IpcError(e) => write!(f, "IPC: {:?}", e),
            Self::ServerError(e) => write!(f, "server: {:?}", e),
            Self::InvalidResponse => write!(f, "malformed response"),
            Self::BufferTooSmall => write!(f, "buffer too small"),
            Self::VerificationFailed => write!(f, "verification failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// CryptoClient — typed handle to the crypto server
// ---------------------------------------------------------------------------

/// Typed handle to the crypto server IPC channel.
///
/// `CryptoClient` is the primary interface for cryptographic operations.
/// It wraps an IPC channel handle and provides ergonomic methods for
/// hashing, HMAC, AEAD, and digital signatures.
///
/// # Construction
///
/// Create a client using the channel handle from your application's generated
/// handle module:
///
/// ```ignore
/// use crypto_client::CryptoClient;
/// use app_my_app::handle;
///
/// let crypto = CryptoClient::new(handle::CRYPTO);
/// ```
///
/// # Thread Safety
///
/// `CryptoClient` is `Send` and `Sync`. The underlying IPC channel handles
/// concurrent requests correctly, but operations are serialized by the server.
///
/// # Performance
///
/// Zero-cost abstraction — the struct is a single `u32` that the compiler
/// inlines away. Each method performs one blocking IPC round-trip.
pub struct CryptoClient {
    handle: u32,
}

impl CryptoClient {
    /// Bind to the crypto server channel.
    ///
    /// `handle` is the IPC channel handle from the app's generated handle module
    /// (e.g., `handle::CRYPTO`).
    pub const fn new(handle: u32) -> Self {
        Self { handle }
    }

    // -- Digest operations --------------------------------------------------

    /// Compute SHA-256 hash of the input data.
    ///
    /// Returns the 32-byte digest directly as a fixed-size array.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let hash = crypto.sha256(b"hello world")?;
    /// assert_eq!(hash.len(), 32);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::BufferTooSmall`] if `data` exceeds the maximum
    /// payload size (~900 bytes).
    pub fn sha256(&self, data: &[u8]) -> Result<[u8; 32], ClientError> {
        self.hash_op(CryptoOp::Sha256Hash, data)
    }

    /// Compute SHA-384 hash of the input data.
    ///
    /// Returns the 48-byte digest directly as a fixed-size array.
    pub fn sha384(&self, data: &[u8]) -> Result<[u8; 48], ClientError> {
        self.hash_op(CryptoOp::Sha384Hash, data)
    }

    /// Compute SHA-512 hash of the input data.
    ///
    /// Returns the 64-byte digest directly as a fixed-size array.
    pub fn sha512(&self, data: &[u8]) -> Result<[u8; 64], ClientError> {
        self.hash_op(CryptoOp::Sha512Hash, data)
    }

    // -- Streaming digest operations ----------------------------------------

    /// Begin a streaming SHA-256 hash computation.
    ///
    /// Use this when the data to hash is too large for a single IPC buffer
    /// (>~900 bytes) or arrives in chunks. Returns a session that accumulates
    /// data via [`Sha256Session::update`] and produces the final hash via
    /// [`Sha256Session::finalize`].
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut session = crypto.sha256_begin()?;
    /// session.update(chunk1)?;
    /// session.update(chunk2)?;
    /// let hash: [u8; 32] = session.finalize()?;
    /// ```
    ///
    /// # Session Semantics
    ///
    /// Only one streaming session can be active at a time per server instance.
    /// Starting a new session while one is active returns `SessionBusy`.
    pub fn sha256_begin(&self) -> Result<Sha256Session, ClientError> {
        self.begin_hash_session(CryptoOp::Sha256Begin, Sha256Session { handle: self.handle })
    }

    /// Begin a streaming SHA-384 hash computation.
    pub fn sha384_begin(&self) -> Result<Sha384Session, ClientError> {
        self.begin_hash_session(CryptoOp::Sha384Begin, Sha384Session { handle: self.handle })
    }

    /// Begin a streaming SHA-512 hash computation.
    pub fn sha512_begin(&self) -> Result<Sha512Session, ClientError> {
        self.begin_hash_session(CryptoOp::Sha512Begin, Sha512Session { handle: self.handle })
    }

    fn begin_hash_session<S>(&self, op: CryptoOp, session: S) -> Result<S, ClientError> {
        let mut request = [0u8; MAX_BUF_SIZE];
        let mut response = [0u8; MAX_BUF_SIZE];

        let header = CryptoRequestHeader::new(op, 0, 0, 0);
        let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
        request[..CryptoRequestHeader::SIZE].copy_from_slice(header_bytes);

        let response_len = syscall::channel_transact(
            self.handle,
            &request[..CryptoRequestHeader::SIZE],
            &mut response,
            Instant::MAX,
        )?;

        // Check for success (empty body)
        if response_len < CryptoResponseHeader::SIZE {
            return Err(ClientError::InvalidResponse);
        }
        let header_bytes = &response[..CryptoResponseHeader::SIZE];
        let Some(resp_header) = zerocopy::Ref::<_, CryptoResponseHeader>::from_bytes(header_bytes).ok() else {
            return Err(ClientError::InvalidResponse);
        };
        if !resp_header.is_success() {
            return Err(ClientError::ServerError(resp_header.error_code()));
        }

        Ok(session)
    }

    // -- HMAC operations ----------------------------------------------------

    /// Compute HMAC-SHA256 authentication tag.
    ///
    /// Returns the 32-byte tag directly. HMAC provides both integrity and
    /// authenticity — use the same key to verify.
    ///
    /// # Arguments
    ///
    /// * `key` — Secret key (any length, but ≥32 bytes recommended)
    /// * `data` — Message to authenticate
    ///
    /// # Example
    ///
    /// ```ignore
    /// let key = b"my-secret-key-32-bytes-long!!!!!";
    /// let tag = crypto.hmac_sha256(key, b"message")?;
    ///
    /// // Verify by recomputing
    /// let tag2 = crypto.hmac_sha256(key, b"message")?;
    /// assert_eq!(tag, tag2);
    /// ```
    pub fn hmac_sha256(&self, key: &[u8], data: &[u8]) -> Result<[u8; 32], ClientError> {
        self.hmac_op(CryptoOp::HmacSha256, key, data)
    }

    /// Compute HMAC-SHA384 authentication tag. Returns the 48-byte tag.
    pub fn hmac_sha384(&self, key: &[u8], data: &[u8]) -> Result<[u8; 48], ClientError> {
        self.hmac_op(CryptoOp::HmacSha384, key, data)
    }

    /// Compute HMAC-SHA512. Returns the 64-byte tag.
    pub fn hmac_sha512(&self, key: &[u8], data: &[u8]) -> Result<[u8; 64], ClientError> {
        self.hmac_op(CryptoOp::HmacSha512, key, data)
    }

    // -- AEAD operations (seal/open) ----------------------------------------

    /// AES-256-GCM authenticated encryption (seal).
    ///
    /// Encrypts `plaintext` and appends a 16-byte authentication tag.
    /// The combination provides both confidentiality and integrity.
    ///
    /// # Arguments
    ///
    /// * `key` — 256-bit (32-byte) AES key
    /// * `nonce` — 96-bit (12-byte) nonce; **must be unique per key**
    /// * `plaintext` — Data to encrypt
    /// * `out` — Output buffer; must be at least `plaintext.len() + 16` bytes
    ///
    /// # Returns
    ///
    /// Number of bytes written to `out` (ciphertext length + 16-byte tag).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let key = [0u8; 32];  // Use a real key!
    /// let nonce = [0u8; 12]; // Must be unique per encryption
    /// let plaintext = b"secret message";
    ///
    /// let mut ciphertext = [0u8; 128];
    /// let ct_len = crypto.aes256_gcm_seal(&key, &nonce, plaintext, &mut ciphertext)?;
    /// // ciphertext[..ct_len] contains encrypted data + tag
    /// ```
    ///
    /// # Security
    ///
    /// Never reuse a nonce with the same key. Nonce reuse completely breaks
    /// AES-GCM security, allowing tag forgery and plaintext recovery.
    pub fn aes256_gcm_seal(
        &self,
        key: &[u8; 32],
        nonce: &[u8; 12],
        plaintext: &[u8],
        out: &mut [u8],
    ) -> Result<usize, ClientError> {
        self.cipher_op(CryptoOp::Aes256GcmEncrypt, key, nonce, plaintext, out)
    }

    /// AES-256-GCM authenticated decryption (open).
    ///
    /// Decrypts and verifies ciphertext produced by [`aes256_gcm_seal`](Self::aes256_gcm_seal).
    /// The ciphertext must include the 16-byte authentication tag.
    ///
    /// # Arguments
    ///
    /// * `key` — Same 256-bit key used for sealing
    /// * `nonce` — Same 96-bit nonce used for sealing
    /// * `ciphertext` — Data from `seal` (encrypted data + tag)
    /// * `out` — Output buffer; must be at least `ciphertext.len() - 16` bytes
    ///
    /// # Returns
    ///
    /// Number of bytes written to `out` (plaintext length).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::ServerError`] with `AuthenticationFailed` if
    /// the tag is invalid (tampered data, wrong key, or wrong nonce).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut plaintext = [0u8; 128];
    /// let pt_len = crypto.aes256_gcm_open(&key, &nonce, &ciphertext[..ct_len], &mut plaintext)?;
    /// assert_eq!(&plaintext[..pt_len], b"secret message");
    /// ```
    pub fn aes256_gcm_open(
        &self,
        key: &[u8; 32],
        nonce: &[u8; 12],
        ciphertext: &[u8],
        out: &mut [u8],
    ) -> Result<usize, ClientError> {
        self.cipher_op(CryptoOp::Aes256GcmDecrypt, key, nonce, ciphertext, out)
    }

    // -- RNG operations -----------------------------------------------------

    /// Generate cryptographically secure random bytes.
    ///
    /// Fills the provided buffer with random data from a CSPRNG
    /// (ChaCha20) seeded with system entropy.
    ///
    /// # Arguments
    ///
    /// * `buf` — Output buffer to fill with random bytes
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut nonce = [0u8; 32];
    /// crypto.get_random_bytes(&mut nonce)?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::BufferTooSmall`] if buffer exceeds max payload size.
    pub fn get_random_bytes(&self, buf: &mut [u8]) -> Result<(), ClientError> {
        let len = buf.len();

        if len == 0 {
            return Ok(());
        }

        if len > MAX_PAYLOAD_SIZE {
            return Err(ClientError::BufferTooSmall);
        }

        let mut request = [0u8; MAX_BUF_SIZE];
        let mut response = [0u8; MAX_BUF_SIZE];

        // Encode length in data_len field, no payload needed
        let header = CryptoRequestHeader::new(CryptoOp::GetRandomBytes, 0, 0, len as u16);
        let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
        request[..CryptoRequestHeader::SIZE].copy_from_slice(header_bytes);

        let response_len = syscall::channel_transact(
            self.handle,
            &request[..CryptoRequestHeader::SIZE],
            &mut response,
            Instant::MAX,
        )?;

        // Parse variable-length response
        parse_variable_response(&response[..response_len], buf)?;
        Ok(())
    }

    // -- ECDSA operations ---------------------------------------------------

    /// Sign a message with ECDSA P-256 (secp256r1).
    ///
    /// Returns the 64-byte signature (r || s, each 32 bytes).
    /// Uses RFC 6979 deterministic signatures with SHA-256 internally.
    ///
    /// # Arguments
    ///
    /// * `private_key` — 32-byte scalar (SEC1 format)
    /// * `message` — Message to sign (hashed internally)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let sig = crypto.ecdsa_p256_sign(&private_key, b"message to sign")?;
    /// assert_eq!(sig.len(), 64);
    /// ```
    #[cfg(feature = "ecdsa")]
    pub fn ecdsa_p256_sign(
        &self,
        private_key: &[u8; 32],
        message: &[u8],
    ) -> Result<[u8; 64], ClientError> {
        self.sign_op(CryptoOp::EcdsaP256Sign, private_key, message)
    }

    /// Verify an ECDSA P-256 signature.
    ///
    /// # Arguments
    ///
    /// * `public_key` — Uncompressed SEC1 point (65 bytes, starting with 0x04)
    /// * `message` — Original message (hashed internally)
    /// * `signature` — 64-byte signature from [`ecdsa_p256_sign`](Self::ecdsa_p256_sign)
    ///
    /// # Returns
    ///
    /// - `Ok(())` — Signature is valid
    /// - `Err(VerificationFailed)` — Signature is invalid
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Verify — Ok(()) means valid
    /// crypto.ecdsa_p256_verify(&public_key, message, &signature)?;
    ///
    /// // Check for invalid signature
    /// if crypto.ecdsa_p256_verify(&public_key, wrong_message, &signature).is_err() {
    ///     // Signature invalid for this message
    /// }
    /// ```
    #[cfg(feature = "ecdsa")]
    pub fn ecdsa_p256_verify(
        &self,
        public_key: &[u8],
        message: &[u8],
        signature: &[u8; 64],
    ) -> Result<(), ClientError> {
        self.verify_op(CryptoOp::EcdsaP256Verify, public_key, message, signature)
    }

    /// Sign a message with ECDSA P-384. Returns the 96-byte signature.
    #[cfg(feature = "ecdsa")]
    pub fn ecdsa_p384_sign(
        &self,
        private_key: &[u8; 48],
        message: &[u8],
    ) -> Result<[u8; 96], ClientError> {
        self.sign_op(CryptoOp::EcdsaP384Sign, private_key, message)
    }

    /// Verify an ECDSA P-384 signature.
    ///
    /// Returns `Ok(())` if the signature is valid.
    /// Returns `Err(VerificationFailed)` if the signature is invalid.
    #[cfg(feature = "ecdsa")]
    pub fn ecdsa_p384_verify(
        &self,
        public_key: &[u8],
        message: &[u8],
        signature: &[u8; 96],
    ) -> Result<(), ClientError> {
        self.verify_op(CryptoOp::EcdsaP384Verify, public_key, message, signature)
    }

    // -- Internal implementation --------------------------------------------

    fn hash_op<const N: usize>(
        &self,
        op: CryptoOp,
        data: &[u8],
    ) -> Result<[u8; N], ClientError> {
        if data.len() > MAX_PAYLOAD_SIZE {
            return Err(ClientError::BufferTooSmall);
        }

        let mut request = [0u8; MAX_BUF_SIZE];
        let mut response = [0u8; MAX_BUF_SIZE];

        let header = CryptoRequestHeader::new(op, 0, 0, data.len() as u16);
        let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
        request[..CryptoRequestHeader::SIZE].copy_from_slice(header_bytes);
        request[CryptoRequestHeader::SIZE..CryptoRequestHeader::SIZE + data.len()]
            .copy_from_slice(data);
        let request_len = CryptoRequestHeader::SIZE + data.len();

        let response_len = syscall::channel_transact(
            self.handle,
            &request[..request_len],
            &mut response,
            Instant::MAX,
        )?;

        parse_fixed_response(&response[..response_len])
    }

    fn hmac_op<const N: usize>(
        &self,
        op: CryptoOp,
        key: &[u8],
        data: &[u8],
    ) -> Result<[u8; N], ClientError> {
        if key.len() + data.len() > MAX_PAYLOAD_SIZE {
            return Err(ClientError::BufferTooSmall);
        }

        let mut request = [0u8; MAX_BUF_SIZE];
        let mut response = [0u8; MAX_BUF_SIZE];

        let header = CryptoRequestHeader::new(op, key.len() as u16, 0, data.len() as u16);
        let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
        request[..CryptoRequestHeader::SIZE].copy_from_slice(header_bytes);

        let mut offset = CryptoRequestHeader::SIZE;
        request[offset..offset + key.len()].copy_from_slice(key);
        offset += key.len();
        request[offset..offset + data.len()].copy_from_slice(data);
        offset += data.len();

        let response_len = syscall::channel_transact(
            self.handle,
            &request[..offset],
            &mut response,
            Instant::MAX,
        )?;

        parse_fixed_response(&response[..response_len])
    }

    fn cipher_op(
        &self,
        op: CryptoOp,
        key: &[u8; 32],
        nonce: &[u8; 12],
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, ClientError> {
        if 32 + 12 + input.len() > MAX_PAYLOAD_SIZE {
            return Err(ClientError::BufferTooSmall);
        }

        let mut request = [0u8; MAX_BUF_SIZE];
        let mut response = [0u8; MAX_BUF_SIZE];

        let header = CryptoRequestHeader::new(op, 32, 12, input.len() as u16);
        let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
        request[..CryptoRequestHeader::SIZE].copy_from_slice(header_bytes);

        let mut offset = CryptoRequestHeader::SIZE;
        request[offset..offset + 32].copy_from_slice(key);
        offset += 32;
        request[offset..offset + 12].copy_from_slice(nonce);
        offset += 12;
        request[offset..offset + input.len()].copy_from_slice(input);
        offset += input.len();

        let response_len = syscall::channel_transact(
            self.handle,
            &request[..offset],
            &mut response,
            Instant::MAX,
        )?;

        parse_variable_response(&response[..response_len], output)
    }

    #[cfg(feature = "ecdsa")]
    fn sign_op<const N: usize>(
        &self,
        op: CryptoOp,
        private_key: &[u8],
        message: &[u8],
    ) -> Result<[u8; N], ClientError> {
        let key_len = private_key.len();
        if message.len() + key_len > MAX_PAYLOAD_SIZE {
            return Err(ClientError::BufferTooSmall);
        }

        let mut request = [0u8; MAX_BUF_SIZE];
        let mut response = [0u8; MAX_BUF_SIZE];

        let header = CryptoRequestHeader::new(
            op,
            key_len as u16,
            0,
            message.len() as u16,
        );
        let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
        request[..CryptoRequestHeader::SIZE].copy_from_slice(header_bytes);

        let mut offset = CryptoRequestHeader::SIZE;
        request[offset..offset + key_len].copy_from_slice(private_key);
        offset += key_len;
        request[offset..offset + message.len()].copy_from_slice(message);
        offset += message.len();

        let response_len = syscall::channel_transact(
            self.handle,
            &request[..offset],
            &mut response,
            Instant::MAX,
        )?;

        parse_fixed_response(&response[..response_len])
    }

    #[cfg(feature = "ecdsa")]
    fn verify_op(
        &self,
        op: CryptoOp,
        public_key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), ClientError> {
        let sig_len = signature.len();
        if message.len() + public_key.len() + sig_len > MAX_PAYLOAD_SIZE {
            return Err(ClientError::BufferTooSmall);
        }

        let mut request = [0u8; MAX_BUF_SIZE];
        let mut response = [0u8; MAX_BUF_SIZE];

        // key=pubkey, nonce=signature, data=message
        let header = CryptoRequestHeader::new(
            op,
            public_key.len() as u16,
            sig_len as u16,
            message.len() as u16,
        );
        let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
        request[..CryptoRequestHeader::SIZE].copy_from_slice(header_bytes);

        let mut offset = CryptoRequestHeader::SIZE;
        request[offset..offset + public_key.len()].copy_from_slice(public_key);
        offset += public_key.len();
        request[offset..offset + sig_len].copy_from_slice(signature);
        offset += sig_len;
        request[offset..offset + message.len()].copy_from_slice(message);
        offset += message.len();

        let response_len = syscall::channel_transact(
            self.handle,
            &request[..offset],
            &mut response,
            Instant::MAX,
        )?;

        // Server returns success with empty body for valid, or error for invalid
        parse_verify_response(&response[..response_len])
    }
}

// ---------------------------------------------------------------------------
// Response parsing helpers
// ---------------------------------------------------------------------------

/// Parse a response with a fixed-size result, returning it by value.
fn parse_fixed_response<const N: usize>(
    response: &[u8],
) -> Result<[u8; N], ClientError> {
    if response.len() < CryptoResponseHeader::SIZE {
        return Err(ClientError::InvalidResponse);
    }

    let header_bytes = &response[..CryptoResponseHeader::SIZE];
    let Some(header) = zerocopy::Ref::<_, CryptoResponseHeader>::from_bytes(header_bytes).ok() else {
        return Err(ClientError::InvalidResponse);
    };
    let header: &CryptoResponseHeader = &*header;

    if !header.is_success() {
        return Err(ClientError::ServerError(header.error_code()));
    }

    let result_len = header.result_length();
    if result_len != N {
        return Err(ClientError::InvalidResponse);
    }

    let mut output = [0u8; N];
    output.copy_from_slice(
        &response[CryptoResponseHeader::SIZE..CryptoResponseHeader::SIZE + N],
    );
    Ok(output)
}

/// Parse a response with variable-size result into a caller-provided buffer.
fn parse_variable_response(
    response: &[u8],
    output: &mut [u8],
) -> Result<usize, ClientError> {
    if response.len() < CryptoResponseHeader::SIZE {
        return Err(ClientError::InvalidResponse);
    }

    let header_bytes = &response[..CryptoResponseHeader::SIZE];
    let Some(header) = zerocopy::Ref::<_, CryptoResponseHeader>::from_bytes(header_bytes).ok() else {
        return Err(ClientError::InvalidResponse);
    };
    let header: &CryptoResponseHeader = &*header;

    if !header.is_success() {
        return Err(ClientError::ServerError(header.error_code()));
    }

    let result_len = header.result_length();
    if result_len > output.len() {
        return Err(ClientError::BufferTooSmall);
    }

    output[..result_len].copy_from_slice(
        &response[CryptoResponseHeader::SIZE..CryptoResponseHeader::SIZE + result_len],
    );
    Ok(result_len)
}

/// Parse a verify response: success with empty body = valid, error = invalid/failed.
#[cfg(feature = "ecdsa")]
fn parse_verify_response(response: &[u8]) -> Result<(), ClientError> {
    if response.len() < CryptoResponseHeader::SIZE {
        return Err(ClientError::InvalidResponse);
    }

    let header_bytes = &response[..CryptoResponseHeader::SIZE];
    let Some(header) = zerocopy::Ref::<_, CryptoResponseHeader>::from_bytes(header_bytes).ok() else {
        return Err(ClientError::InvalidResponse);
    };
    let header: &CryptoResponseHeader = &*header;

    if !header.is_success() {
        let err = header.error_code();
        if err == CryptoError::VerificationFailed {
            return Err(ClientError::VerificationFailed);
        }
        return Err(ClientError::ServerError(err));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Streaming hash session types
// ---------------------------------------------------------------------------

/// Active SHA-256 streaming session.
///
/// Created by [`CryptoClient::sha256_begin`]. Call [`update`](Self::update)
/// to feed data, then [`finalize`](Self::finalize) to get the final hash.
pub struct Sha256Session {
    handle: u32,
}

impl Sha256Session {
    /// Feed data into the hash computation.
    ///
    /// Can be called multiple times. Each chunk is accumulated server-side.
    pub fn update(&mut self, data: &[u8]) -> Result<(), ClientError> {
        streaming_update(self.handle, CryptoOp::Sha256Update, data)
    }

    /// Finalize the hash and return the 32-byte digest.
    ///
    /// Consumes the session. The server clears its internal state.
    pub fn finalize(self) -> Result<[u8; 32], ClientError> {
        streaming_finish(self.handle, CryptoOp::Sha256Finish)
    }
}

/// Active SHA-384 streaming session.
pub struct Sha384Session {
    handle: u32,
}

impl Sha384Session {
    /// Feed data into the hash computation.
    pub fn update(&mut self, data: &[u8]) -> Result<(), ClientError> {
        streaming_update(self.handle, CryptoOp::Sha384Update, data)
    }

    /// Finalize the hash and return the 48-byte digest.
    pub fn finalize(self) -> Result<[u8; 48], ClientError> {
        streaming_finish(self.handle, CryptoOp::Sha384Finish)
    }
}

/// Active SHA-512 streaming session.
pub struct Sha512Session {
    handle: u32,
}

impl Sha512Session {
    /// Feed data into the hash computation.
    pub fn update(&mut self, data: &[u8]) -> Result<(), ClientError> {
        streaming_update(self.handle, CryptoOp::Sha512Update, data)
    }

    /// Finalize the hash and return the 64-byte digest.
    pub fn finalize(self) -> Result<[u8; 64], ClientError> {
        streaming_finish(self.handle, CryptoOp::Sha512Finish)
    }
}

/// Internal: send data to an active streaming session.
fn streaming_update(handle: u32, op: CryptoOp, data: &[u8]) -> Result<(), ClientError> {
    if data.len() > MAX_PAYLOAD_SIZE {
        return Err(ClientError::BufferTooSmall);
    }

    let mut request = [0u8; MAX_BUF_SIZE];
    let mut response = [0u8; MAX_BUF_SIZE];

    let header = CryptoRequestHeader::new(op, 0, 0, data.len() as u16);
    let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
    request[..CryptoRequestHeader::SIZE].copy_from_slice(header_bytes);
    request[CryptoRequestHeader::SIZE..CryptoRequestHeader::SIZE + data.len()]
        .copy_from_slice(data);
    let request_len = CryptoRequestHeader::SIZE + data.len();

    let response_len = syscall::channel_transact(
        handle,
        &request[..request_len],
        &mut response,
        Instant::MAX,
    )?;

    // Check for success
    if response_len < CryptoResponseHeader::SIZE {
        return Err(ClientError::InvalidResponse);
    }
    let header_bytes = &response[..CryptoResponseHeader::SIZE];
    let Some(resp_header) = zerocopy::Ref::<_, CryptoResponseHeader>::from_bytes(header_bytes).ok() else {
        return Err(ClientError::InvalidResponse);
    };
    if !resp_header.is_success() {
        return Err(ClientError::ServerError(resp_header.error_code()));
    }

    Ok(())
}

/// Internal: finalize a streaming session and return the hash.
fn streaming_finish<const N: usize>(handle: u32, op: CryptoOp) -> Result<[u8; N], ClientError> {
    let mut request = [0u8; MAX_BUF_SIZE];
    let mut response = [0u8; MAX_BUF_SIZE];

    let header = CryptoRequestHeader::new(op, 0, 0, 0);
    let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
    request[..CryptoRequestHeader::SIZE].copy_from_slice(header_bytes);

    let response_len = syscall::channel_transact(
        handle,
        &request[..CryptoRequestHeader::SIZE],
        &mut response,
        Instant::MAX,
    )?;

    parse_fixed_response(&response[..response_len])
}

// ---------------------------------------------------------------------------
// Free function wrappers (backward compatibility)
// ---------------------------------------------------------------------------

/// Convenience: compute SHA-256 without constructing a client.
pub fn sha256(handle: u32, data: &[u8]) -> Result<[u8; 32], ClientError> {
    CryptoClient::new(handle).sha256(data)
}

/// Convenience: compute SHA-384 without constructing a client.
pub fn sha384(handle: u32, data: &[u8]) -> Result<[u8; 48], ClientError> {
    CryptoClient::new(handle).sha384(data)
}

/// Convenience: compute SHA-512 without constructing a client.
pub fn sha512(handle: u32, data: &[u8]) -> Result<[u8; 64], ClientError> {
    CryptoClient::new(handle).sha512(data)
}

/// Convenience: compute HMAC-SHA256 without constructing a client.
pub fn hmac_sha256(handle: u32, key: &[u8], data: &[u8]) -> Result<[u8; 32], ClientError> {
    CryptoClient::new(handle).hmac_sha256(key, data)
}

/// Convenience: compute HMAC-SHA384 without constructing a client.
pub fn hmac_sha384(handle: u32, key: &[u8], data: &[u8]) -> Result<[u8; 48], ClientError> {
    CryptoClient::new(handle).hmac_sha384(key, data)
}

/// Convenience: compute HMAC-SHA512 without constructing a client.
pub fn hmac_sha512(handle: u32, key: &[u8], data: &[u8]) -> Result<[u8; 64], ClientError> {
    CryptoClient::new(handle).hmac_sha512(key, data)
}

/// Convenience: AES-256-GCM seal without constructing a client.
pub fn aes256_gcm_seal(
    handle: u32,
    key: &[u8; 32],
    nonce: &[u8; 12],
    plaintext: &[u8],
    out: &mut [u8],
) -> Result<usize, ClientError> {
    CryptoClient::new(handle).aes256_gcm_seal(key, nonce, plaintext, out)
}

/// Convenience: AES-256-GCM open without constructing a client.
pub fn aes256_gcm_open(
    handle: u32,
    key: &[u8; 32],
    nonce: &[u8; 12],
    ciphertext: &[u8],
    out: &mut [u8],
) -> Result<usize, ClientError> {
    CryptoClient::new(handle).aes256_gcm_open(key, nonce, ciphertext, out)
}

/// Convenience: get random bytes without constructing a client.
pub fn get_random_bytes(handle: u32, buf: &mut [u8]) -> Result<(), ClientError> {
    CryptoClient::new(handle).get_random_bytes(buf)
}

/// Convenience: ECDSA P-256 sign without constructing a client.
#[cfg(feature = "ecdsa")]
pub fn ecdsa_p256_sign(
    handle: u32,
    private_key: &[u8; 32],
    message: &[u8],
) -> Result<[u8; 64], ClientError> {
    CryptoClient::new(handle).ecdsa_p256_sign(private_key, message)
}

/// Convenience: ECDSA P-256 verify without constructing a client.
#[cfg(feature = "ecdsa")]
pub fn ecdsa_p256_verify(
    handle: u32,
    public_key: &[u8],
    message: &[u8],
    signature: &[u8; 64],
) -> Result<(), ClientError> {
    CryptoClient::new(handle).ecdsa_p256_verify(public_key, message, signature)
}

/// Convenience: ECDSA P-384 sign without constructing a client.
#[cfg(feature = "ecdsa")]
pub fn ecdsa_p384_sign(
    handle: u32,
    private_key: &[u8; 48],
    message: &[u8],
) -> Result<[u8; 96], ClientError> {
    CryptoClient::new(handle).ecdsa_p384_sign(private_key, message)
}

/// Convenience: ECDSA P-384 verify without constructing a client.
#[cfg(feature = "ecdsa")]
pub fn ecdsa_p384_verify(
    handle: u32,
    public_key: &[u8],
    message: &[u8],
    signature: &[u8; 96],
) -> Result<(), ClientError> {
    CryptoClient::new(handle).ecdsa_p384_verify(public_key, message, signature)
}

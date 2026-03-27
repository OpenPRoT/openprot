// Licensed under the Apache-2.0 license

//! Crypto IPC Protocol Definitions
//!
//! Wire format for crypto requests and responses between client and server.

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// Maximum payload size for crypto operations
pub const MAX_PAYLOAD_SIZE: usize = 512;

/// Maximum key size (256 bits = 32 bytes)
pub const MAX_KEY_SIZE: usize = 32;

/// Maximum nonce/IV size (96 bits for GCM)
pub const MAX_NONCE_SIZE: usize = 12;

/// Maximum hash output size (SHA-512 = 64 bytes)
pub const MAX_HASH_SIZE: usize = 64;

/// Maximum tag size for AEAD (128 bits = 16 bytes)
pub const MAX_TAG_SIZE: usize = 16;

/// ECDSA P-256 private key size (32 bytes)
pub const ECDSA_P256_PRIVATE_KEY_SIZE: usize = 32;

/// ECDSA P-256 public key size (uncompressed: 65 bytes, compressed: 33 bytes)
pub const ECDSA_P256_PUBLIC_KEY_SIZE: usize = 65;

/// ECDSA P-256 signature size (DER-encoded max ~72 bytes, fixed 64 bytes)
pub const ECDSA_P256_SIGNATURE_SIZE: usize = 64;

/// ECDSA P-384 private key size (48 bytes)
pub const ECDSA_P384_PRIVATE_KEY_SIZE: usize = 48;

/// ECDSA P-384 public key size (uncompressed: 97 bytes)
pub const ECDSA_P384_PUBLIC_KEY_SIZE: usize = 97;

/// ECDSA P-384 signature size (fixed 96 bytes)
pub const ECDSA_P384_SIGNATURE_SIZE: usize = 96;

/// Crypto operation codes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoOp {
    // Digest operations - one-shot (0x01-0x03)
    Sha256Hash = 0x01,
    Sha384Hash = 0x02,
    Sha512Hash = 0x03,

    // Digest operations - streaming (0x04-0x0C)
    Sha256Begin = 0x04,
    Sha256Update = 0x05,
    Sha256Finish = 0x06,
    Sha384Begin = 0x07,
    Sha384Update = 0x08,
    Sha384Finish = 0x09,
    Sha512Begin = 0x0A,
    Sha512Update = 0x0B,
    Sha512Finish = 0x0C,

    // MAC operations (0x10-0x1F)
    HmacSha256 = 0x10,
    HmacSha384 = 0x11,
    HmacSha512 = 0x12,

    // AEAD cipher operations (0x20-0x2F)
    Aes256GcmEncrypt = 0x20,
    Aes256GcmDecrypt = 0x21,

    // RNG operations (0x30-0x3F)
    GetRandomBytes = 0x30,

    // ECDSA operations (0x40-0x4F)
    EcdsaP256Sign = 0x40,
    EcdsaP256Verify = 0x41,
    EcdsaP384Sign = 0x42,
    EcdsaP384Verify = 0x43,
}

impl TryFrom<u8> for CryptoOp {
    type Error = CryptoError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(CryptoOp::Sha256Hash),
            0x02 => Ok(CryptoOp::Sha384Hash),
            0x03 => Ok(CryptoOp::Sha512Hash),
            0x04 => Ok(CryptoOp::Sha256Begin),
            0x05 => Ok(CryptoOp::Sha256Update),
            0x06 => Ok(CryptoOp::Sha256Finish),
            0x07 => Ok(CryptoOp::Sha384Begin),
            0x08 => Ok(CryptoOp::Sha384Update),
            0x09 => Ok(CryptoOp::Sha384Finish),
            0x0A => Ok(CryptoOp::Sha512Begin),
            0x0B => Ok(CryptoOp::Sha512Update),
            0x0C => Ok(CryptoOp::Sha512Finish),
            0x10 => Ok(CryptoOp::HmacSha256),
            0x11 => Ok(CryptoOp::HmacSha384),
            0x12 => Ok(CryptoOp::HmacSha512),
            0x20 => Ok(CryptoOp::Aes256GcmEncrypt),
            0x21 => Ok(CryptoOp::Aes256GcmDecrypt),
            0x30 => Ok(CryptoOp::GetRandomBytes),
            0x40 => Ok(CryptoOp::EcdsaP256Sign),
            0x41 => Ok(CryptoOp::EcdsaP256Verify),
            0x42 => Ok(CryptoOp::EcdsaP384Sign),
            0x43 => Ok(CryptoOp::EcdsaP384Verify),
            _ => Err(CryptoError::InvalidOperation),
        }
    }
}

/// Crypto error codes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    Success = 0x00,
    InvalidOperation = 0x01,
    InvalidKeyLength = 0x02,
    InvalidNonceLength = 0x03,
    InvalidDataLength = 0x04,
    AuthenticationFailed = 0x05,
    EncryptionFailed = 0x06,
    DecryptionFailed = 0x07,
    BufferTooSmall = 0x08,
    SigningFailed = 0x09,
    VerificationFailed = 0x0A,
    InvalidSignature = 0x0B,
    /// No active session for this client
    SessionNotFound = 0x0C,
    /// A session is already active (only one at a time per client)
    SessionBusy = 0x0D,
    InternalError = 0xFF,
}

impl From<u8> for CryptoError {
    fn from(value: u8) -> Self {
        match value {
            0x00 => CryptoError::Success,
            0x01 => CryptoError::InvalidOperation,
            0x02 => CryptoError::InvalidKeyLength,
            0x03 => CryptoError::InvalidNonceLength,
            0x04 => CryptoError::InvalidDataLength,
            0x05 => CryptoError::AuthenticationFailed,
            0x06 => CryptoError::EncryptionFailed,
            0x07 => CryptoError::DecryptionFailed,
            0x08 => CryptoError::BufferTooSmall,
            0x09 => CryptoError::SigningFailed,
            0x0A => CryptoError::VerificationFailed,
            0x0B => CryptoError::InvalidSignature,
            0x0C => CryptoError::SessionNotFound,
            0x0D => CryptoError::SessionBusy,
            _ => CryptoError::InternalError,
        }
    }
}

/// Request header (fixed 8 bytes)
///
/// Wire format:
/// ```text
/// +--------+--------+----------+------------+----------+
/// | op (1) | flags  | key_len  | nonce_len  | data_len |
/// +--------+--------+----------+------------+----------+
/// | u8     | u8     | u16 LE   | u16 LE     | u16 LE   |
/// +--------+--------+----------+------------+----------+
/// ```
///
/// Followed by: key (key_len bytes) || nonce (nonce_len bytes) || data (data_len bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct CryptoRequestHeader {
    pub op_code: u8,
    pub flags: u8,
    pub key_len: u16,
    pub nonce_len: u16,
    pub data_len: u16,
}

impl CryptoRequestHeader {
    pub const SIZE: usize = 8;

    pub fn new(op: CryptoOp, key_len: u16, nonce_len: u16, data_len: u16) -> Self {
        Self {
            op_code: op as u8,
            flags: 0,
            key_len: key_len.to_le(),
            nonce_len: nonce_len.to_le(),
            data_len: data_len.to_le(),
        }
    }

    pub fn operation(&self) -> Result<CryptoOp, CryptoError> {
        CryptoOp::try_from(self.op_code)
    }

    pub fn key_length(&self) -> usize {
        u16::from_le(self.key_len) as usize
    }

    pub fn nonce_length(&self) -> usize {
        u16::from_le(self.nonce_len) as usize
    }

    pub fn data_length(&self) -> usize {
        u16::from_le(self.data_len) as usize
    }

    pub fn total_payload_size(&self) -> usize {
        self.key_length() + self.nonce_length() + self.data_length()
    }
}

/// Response header (fixed 4 bytes)
///
/// Wire format:
/// ```text
/// +----------+----------+------------+
/// | status   | reserved | result_len |
/// +----------+----------+------------+
/// | u8       | u8       | u16 LE     |
/// +----------+----------+------------+
/// ```
///
/// Followed by: result (result_len bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct CryptoResponseHeader {
    pub status: u8,
    pub reserved: u8,
    pub result_len: u16,
}

impl CryptoResponseHeader {
    pub const SIZE: usize = 4;

    pub fn success(result_len: u16) -> Self {
        Self {
            status: CryptoError::Success as u8,
            reserved: 0,
            result_len: result_len.to_le(),
        }
    }

    pub fn error(err: CryptoError) -> Self {
        Self {
            status: err as u8,
            reserved: 0,
            result_len: 0,
        }
    }

    pub fn is_success(&self) -> bool {
        self.status == CryptoError::Success as u8
    }

    pub fn error_code(&self) -> CryptoError {
        CryptoError::from(self.status)
    }

    pub fn result_length(&self) -> usize {
        u16::from_le(self.result_len) as usize
    }
}

/// Hash output sizes by algorithm
pub const SHA256_OUTPUT_SIZE: usize = 32;
pub const SHA384_OUTPUT_SIZE: usize = 48;
pub const SHA512_OUTPUT_SIZE: usize = 64;

/// Get expected output size for a crypto operation
pub fn output_size_for_op(op: CryptoOp, input_len: usize) -> usize {
    match op {
        CryptoOp::Sha256Hash | CryptoOp::Sha256Finish => SHA256_OUTPUT_SIZE,
        CryptoOp::Sha384Hash | CryptoOp::Sha384Finish => SHA384_OUTPUT_SIZE,
        CryptoOp::Sha512Hash | CryptoOp::Sha512Finish => SHA512_OUTPUT_SIZE,
        // Streaming begin/update return empty success
        CryptoOp::Sha256Begin | CryptoOp::Sha256Update |
        CryptoOp::Sha384Begin | CryptoOp::Sha384Update |
        CryptoOp::Sha512Begin | CryptoOp::Sha512Update => 0,
        CryptoOp::HmacSha256 => SHA256_OUTPUT_SIZE,
        CryptoOp::HmacSha384 => SHA384_OUTPUT_SIZE,
        CryptoOp::HmacSha512 => SHA512_OUTPUT_SIZE,
        CryptoOp::Aes256GcmEncrypt => input_len + MAX_TAG_SIZE,
        CryptoOp::Aes256GcmDecrypt => input_len.saturating_sub(MAX_TAG_SIZE),
        CryptoOp::GetRandomBytes => input_len,
        CryptoOp::EcdsaP256Sign => ECDSA_P256_SIGNATURE_SIZE,
        CryptoOp::EcdsaP256Verify => 1, // returns 1 byte: 0x01 for valid, 0x00 for invalid
        CryptoOp::EcdsaP384Sign => ECDSA_P384_SIGNATURE_SIZE,
        CryptoOp::EcdsaP384Verify => 1,
    }
}

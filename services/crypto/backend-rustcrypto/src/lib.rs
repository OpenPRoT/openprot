// Licensed under the Apache-2.0 license

//! RustCrypto Backend
//!
//! Implements [`OneShot<A>`] for all supported algorithms using RustCrypto crates.
//! This backend is software-only and works on host, QEMU, and target.
//!
//! # Example
//!
//! ```ignore
//! use crypto_api::backend::{CryptoInput, OneShot, Sha256};
//! use crypto_backend_rustcrypto::RustCryptoBackend;
//!
//! let backend = RustCryptoBackend::new();
//! let input = CryptoInput::Digest { data: b"hello world" };
//! let mut output = [0u8; 32];
//! let len = backend.compute(&input, &mut output).unwrap();
//! ```

#![no_std]

use crypto_api::backend::{
    BackendError, CryptoInput, OneShot, Streaming,
    Sha256, Sha384, Sha512,
    HmacSha256, HmacSha384, HmacSha512,
    Aes256GcmEncrypt, Aes256GcmDecrypt,
    GetRandomBytes,
};

#[cfg(feature = "ecdsa")]
use crypto_api::backend::{EcdsaP256Sign, EcdsaP256Verify, EcdsaP384Sign, EcdsaP384Verify};

use rand_chacha::ChaCha20Rng;
use rand_core::{RngCore, SeedableRng};

use sha2::Digest as Sha2Digest;
use hmac::{Hmac, Mac};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce as GcmNonce, aead::AeadInPlace};

#[cfg(feature = "ecdsa")]
use p256::ecdsa::{
    SigningKey as P256SigningKey, 
    VerifyingKey as P256VerifyingKey, 
    Signature as P256Signature,
    signature::{Signer, Verifier},
};

#[cfg(feature = "ecdsa")]
use p384::ecdsa::{
    SigningKey as P384SigningKey, 
    VerifyingKey as P384VerifyingKey, 
    Signature as P384Signature,
};

// ---------------------------------------------------------------------------
// Backend type
// ---------------------------------------------------------------------------

/// RustCrypto-based software backend.
///
/// This is a zero-sized type — stateless, cheap to copy, can be freely cloned.
/// All state is local to each operation.
#[derive(Clone, Copy, Default, Debug)]
pub struct RustCryptoBackend;

impl RustCryptoBackend {
    /// Create a new RustCrypto backend instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

// ---------------------------------------------------------------------------
// Digest implementations (SHA-2)
// ---------------------------------------------------------------------------

/// Generic digest helper — reduces code duplication across SHA variants.
fn do_digest<D: Sha2Digest>(data: &[u8], output: &mut [u8]) -> Result<usize, BackendError> {
    let mut hasher = D::new();
    hasher.update(data);
    let result = hasher.finalize();
    let size = result.len();
    if output.len() < size {
        return Err(BackendError::BufferTooSmall);
    }
    output[..size].copy_from_slice(&result);
    Ok(size)
}

impl OneShot<Sha256> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Digest { data } = input else {
            return Err(BackendError::InvalidOperation);
        };
        do_digest::<sha2::Sha256>(data, output)
    }
}

impl OneShot<Sha384> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Digest { data } = input else {
            return Err(BackendError::InvalidOperation);
        };
        do_digest::<sha2::Sha384>(data, output)
    }
}

impl OneShot<Sha512> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Digest { data } = input else {
            return Err(BackendError::InvalidOperation);
        };
        do_digest::<sha2::Sha512>(data, output)
    }
}

// ---------------------------------------------------------------------------
// MAC implementations (HMAC-SHA-2)
// ---------------------------------------------------------------------------

type HmacSha2_256 = Hmac<sha2::Sha256>;
type HmacSha2_384 = Hmac<sha2::Sha384>;
type HmacSha2_512 = Hmac<sha2::Sha512>;

impl OneShot<HmacSha256> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Mac { key, data } = input else {
            return Err(BackendError::InvalidOperation);
        };
        let mut mac = <HmacSha2_256 as Mac>::new_from_slice(key)
            .map_err(|_| BackendError::InvalidKeyLength)?;
        mac.update(data);
        let result = mac.finalize().into_bytes();
        if output.len() < 32 {
            return Err(BackendError::BufferTooSmall);
        }
        output[..32].copy_from_slice(&result);
        Ok(32)
    }
}

impl OneShot<HmacSha384> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Mac { key, data } = input else {
            return Err(BackendError::InvalidOperation);
        };
        let mut mac = <HmacSha2_384 as Mac>::new_from_slice(key)
            .map_err(|_| BackendError::InvalidKeyLength)?;
        mac.update(data);
        let result = mac.finalize().into_bytes();
        if output.len() < 48 {
            return Err(BackendError::BufferTooSmall);
        }
        output[..48].copy_from_slice(&result);
        Ok(48)
    }
}

impl OneShot<HmacSha512> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Mac { key, data } = input else {
            return Err(BackendError::InvalidOperation);
        };
        let mut mac = <HmacSha2_512 as Mac>::new_from_slice(key)
            .map_err(|_| BackendError::InvalidKeyLength)?;
        mac.update(data);
        let result = mac.finalize().into_bytes();
        if output.len() < 64 {
            return Err(BackendError::BufferTooSmall);
        }
        output[..64].copy_from_slice(&result);
        Ok(64)
    }
}

// ---------------------------------------------------------------------------
// AEAD implementations (AES-256-GCM)
// ---------------------------------------------------------------------------

const AES_KEY_SIZE: usize = 32;
const GCM_NONCE_SIZE: usize = 12;
const GCM_TAG_SIZE: usize = 16;

impl OneShot<Aes256GcmEncrypt> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Aead { key, nonce, data } = input else {
            return Err(BackendError::InvalidOperation);
        };
        
        if key.len() != AES_KEY_SIZE {
            return Err(BackendError::InvalidKeyLength);
        }
        if nonce.len() != GCM_NONCE_SIZE {
            return Err(BackendError::InvalidNonceLength);
        }
        
        let output_len = data.len() + GCM_TAG_SIZE;
        if output.len() < output_len {
            return Err(BackendError::BufferTooSmall);
        }
        
        let key_array: [u8; 32] = (*key).try_into()
            .map_err(|_| BackendError::InvalidKeyLength)?;
        let cipher = Aes256Gcm::new(&key_array.into());
        let gcm_nonce = GcmNonce::from_slice(nonce);
        
        // Copy plaintext to output buffer for in-place encryption
        output[..data.len()].copy_from_slice(data);
        
        // Encrypt in place, get the tag
        let tag = cipher
            .encrypt_in_place_detached(gcm_nonce, &[], &mut output[..data.len()])
            .map_err(|_| BackendError::InternalError)?;
        
        // Append tag after ciphertext
        output[data.len()..output_len].copy_from_slice(&tag);
        
        Ok(output_len)
    }
}

impl OneShot<Aes256GcmDecrypt> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Aead { key, nonce, data } = input else {
            return Err(BackendError::InvalidOperation);
        };
        
        if key.len() != AES_KEY_SIZE {
            return Err(BackendError::InvalidKeyLength);
        }
        if nonce.len() != GCM_NONCE_SIZE {
            return Err(BackendError::InvalidNonceLength);
        }
        if data.len() < GCM_TAG_SIZE {
            return Err(BackendError::InvalidDataLength);
        }
        
        let ciphertext_len = data.len() - GCM_TAG_SIZE;
        if output.len() < ciphertext_len {
            return Err(BackendError::BufferTooSmall);
        }
        
        let key_array: [u8; 32] = (*key).try_into()
            .map_err(|_| BackendError::InvalidKeyLength)?;
        let cipher = Aes256Gcm::new(&key_array.into());
        let gcm_nonce = GcmNonce::from_slice(nonce);
        
        // Split ciphertext and tag
        let ciphertext = &data[..ciphertext_len];
        let tag = &data[ciphertext_len..];
        
        // Copy ciphertext to output for in-place decryption
        output[..ciphertext_len].copy_from_slice(ciphertext);
        
        // Decrypt in place
        cipher
            .decrypt_in_place_detached(gcm_nonce, &[], &mut output[..ciphertext_len], tag.into())
            .map_err(|_| BackendError::AuthenticationFailed)?;
        
        Ok(ciphertext_len)
    }
}

// ---------------------------------------------------------------------------
// ECDSA implementations (P-256, P-384)
// ---------------------------------------------------------------------------

#[cfg(feature = "ecdsa")]
impl OneShot<EcdsaP256Sign> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Sign { private_key, message } = input else {
            return Err(BackendError::InvalidOperation);
        };
        
        if private_key.len() != 32 {
            return Err(BackendError::InvalidKeyLength);
        }
        if output.len() < 64 {
            return Err(BackendError::BufferTooSmall);
        }
        
        let key = P256SigningKey::from_slice(private_key)
            .map_err(|_| BackendError::InvalidKeyLength)?;
        let sig: P256Signature = key.sign(message);
        output[..64].copy_from_slice(&sig.to_bytes());
        
        Ok(64)
    }
}

#[cfg(feature = "ecdsa")]
impl OneShot<EcdsaP256Verify> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Verify { public_key, message, signature } = input else {
            return Err(BackendError::InvalidOperation);
        };
        
        if signature.len() != 64 {
            return Err(BackendError::VerificationFailed);
        }
        if output.is_empty() {
            return Err(BackendError::BufferTooSmall);
        }
        
        let key = P256VerifyingKey::from_sec1_bytes(public_key)
            .map_err(|_| BackendError::InvalidKeyLength)?;
        let sig = P256Signature::from_slice(signature)
            .map_err(|_| BackendError::VerificationFailed)?;
        
        match key.verify(message, &sig) {
            Ok(()) => {
                output[0] = 1; // verified
                Ok(1)
            }
            Err(_) => {
                output[0] = 0; // failed
                Ok(1)
            }
        }
    }
}

#[cfg(feature = "ecdsa")]
impl OneShot<EcdsaP384Sign> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Sign { private_key, message } = input else {
            return Err(BackendError::InvalidOperation);
        };
        
        if private_key.len() != 48 {
            return Err(BackendError::InvalidKeyLength);
        }
        if output.len() < 96 {
            return Err(BackendError::BufferTooSmall);
        }
        
        let key = P384SigningKey::from_slice(private_key)
            .map_err(|_| BackendError::InvalidKeyLength)?;
        let sig: P384Signature = Signer::sign(&key, message);
        output[..96].copy_from_slice(&sig.to_bytes());
        
        Ok(96)
    }
}

#[cfg(feature = "ecdsa")]
impl OneShot<EcdsaP384Verify> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Verify { public_key, message, signature } = input else {
            return Err(BackendError::InvalidOperation);
        };
        
        if signature.len() != 96 {
            return Err(BackendError::VerificationFailed);
        }
        if output.is_empty() {
            return Err(BackendError::BufferTooSmall);
        }
        
        let key = P384VerifyingKey::from_sec1_bytes(public_key)
            .map_err(|_| BackendError::InvalidKeyLength)?;
        let sig = P384Signature::from_slice(signature)
            .map_err(|_| BackendError::VerificationFailed)?;
        
        match Verifier::verify(&key, message, &sig) {
            Ok(()) => {
                output[0] = 1;
                Ok(1)
            }
            Err(_) => {
                output[0] = 0;
                Ok(1)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// RNG implementation
// ---------------------------------------------------------------------------

impl OneShot<GetRandomBytes> for RustCryptoBackend {
    fn compute(&self, input: &CryptoInput<'_>, output: &mut [u8]) -> Result<usize, BackendError> {
        let CryptoInput::Rng { length } = input else {
            return Err(BackendError::InvalidOperation);
        };

        if *length == 0 {
            return Err(BackendError::InvalidDataLength);
        }

        if *length > output.len() {
            return Err(BackendError::BufferTooSmall);
        }

        // Use ChaCha20Rng with a fixed seed for MVP
        // TODO: Integrate with hardware RNG when available
        // Fixed seed derived from "OpenPRoT-SPDM-RNG-Seed-v1" SHA-256
        let seed: [u8; 32] = [
            0x3c, 0xf8, 0x91, 0x42, 0x7e, 0x5a, 0x1f, 0xd3,
            0x9b, 0x2e, 0x8c, 0x67, 0xa4, 0x0f, 0xb5, 0x28,
            0x71, 0xc6, 0x3d, 0x94, 0xe2, 0x58, 0x0b, 0xfa,
            0x6d, 0xb1, 0x45, 0xc9, 0x87, 0x3a, 0xf6, 0x29,
        ];
        let mut rng = ChaCha20Rng::from_seed(seed);
        rng.fill_bytes(&mut output[..*length]);

        Ok(*length)
    }
}

// ---------------------------------------------------------------------------
// Streaming implementations (SHA-2)
// ---------------------------------------------------------------------------

/// SHA-256 streaming session state.
pub struct Sha256Session(sha2::Sha256);

/// SHA-384 streaming session state.
pub struct Sha384Session(sha2::Sha384);

/// SHA-512 streaming session state.
pub struct Sha512Session(sha2::Sha512);

impl Streaming<Sha256> for RustCryptoBackend {
    type Session = Sha256Session;
    
    fn begin(&mut self) -> Result<Self::Session, BackendError> {
        Ok(Sha256Session(sha2::Sha256::new()))
    }
    
    fn feed(&mut self, session: &mut Self::Session, data: &[u8]) -> Result<(), BackendError> {
        session.0.update(data);
        Ok(())
    }
    
    fn finish(&mut self, session: Self::Session, output: &mut [u8]) -> Result<usize, BackendError> {
        if output.len() < 32 {
            return Err(BackendError::BufferTooSmall);
        }
        let result = session.0.finalize();
        output[..32].copy_from_slice(&result);
        Ok(32)
    }
    
    fn cancel(&mut self, _session: Self::Session) {
        // Session dropped, nothing to clean up for software impl
    }
}

impl Streaming<Sha384> for RustCryptoBackend {
    type Session = Sha384Session;
    
    fn begin(&mut self) -> Result<Self::Session, BackendError> {
        Ok(Sha384Session(sha2::Sha384::new()))
    }
    
    fn feed(&mut self, session: &mut Self::Session, data: &[u8]) -> Result<(), BackendError> {
        session.0.update(data);
        Ok(())
    }
    
    fn finish(&mut self, session: Self::Session, output: &mut [u8]) -> Result<usize, BackendError> {
        if output.len() < 48 {
            return Err(BackendError::BufferTooSmall);
        }
        let result = session.0.finalize();
        output[..48].copy_from_slice(&result);
        Ok(48)
    }
    
    fn cancel(&mut self, _session: Self::Session) {}
}

impl Streaming<Sha512> for RustCryptoBackend {
    type Session = Sha512Session;
    
    fn begin(&mut self) -> Result<Self::Session, BackendError> {
        Ok(Sha512Session(sha2::Sha512::new()))
    }
    
    fn feed(&mut self, session: &mut Self::Session, data: &[u8]) -> Result<(), BackendError> {
        session.0.update(data);
        Ok(())
    }
    
    fn finish(&mut self, session: Self::Session, output: &mut [u8]) -> Result<usize, BackendError> {
        if output.len() < 64 {
            return Err(BackendError::BufferTooSmall);
        }
        let result = session.0.finalize();
        output[..64].copy_from_slice(&result);
        Ok(64)
    }
    
    fn cancel(&mut self, _session: Self::Session) {}
}

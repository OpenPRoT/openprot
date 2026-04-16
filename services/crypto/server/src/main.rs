// Licensed under the Apache-2.0 license

//! Crypto Server Application
//!
//! A userspace crypto service that handles IPC requests from clients.
//! Uses trait-based backend abstraction for pluggable crypto implementations.

#![no_main]
#![no_std]

use crypto_api::{
    CryptoError, CryptoOp, CryptoRequestHeader, CryptoResponseHeader,
};
use crypto_api::backend::{
    OneShot, Streaming,
    Sha256 as Sha256Marker, Sha384 as Sha384Marker, Sha512 as Sha512Marker,
    HmacSha256 as HmacSha256Marker, HmacSha384 as HmacSha384Marker, HmacSha512 as HmacSha512Marker,
    Aes256GcmEncrypt, Aes256GcmDecrypt,
    GetRandomBytes,
};
use crypto_backend_rustcrypto::{
    RustCryptoBackend,
    Sha256Session, Sha384Session, Sha512Session,
};
use pw_status::Result;

use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

use app_crypto_server::handle;

const MAX_REQUEST_SIZE: usize = 1024;
const MAX_RESPONSE_SIZE: usize = 1024;

/// Per-channel streaming hash session state.
enum HashSession {
    None,
    Sha256(Sha256Session),
    Sha384(Sha384Session),
    Sha512(Sha512Session),
}

impl HashSession {
    fn is_none(&self) -> bool {
        matches!(self, HashSession::None)
    }
}

/// All channel handles served by this crypto server instance.
/// Each entry: (channel_handle, user_data for WG)
const CHANNELS: [(u32, usize); 6] = [
    (handle::CRYPTO_REQ,     0),
    (handle::CRYPTO_REQ_M1,  1),
    (handle::CRYPTO_REQ_L1,  2),
    (handle::CRYPTO_RESP,    3),
    (handle::CRYPTO_RESP_M1, 4),
    (handle::CRYPTO_RESP_L1, 5),
];

fn crypto_server_loop() -> Result<()> {
    pw_log::info!("Crypto server starting");

    let mut request_buf = [0u8; MAX_REQUEST_SIZE];
    let mut response_buf = [0u8; MAX_RESPONSE_SIZE];

    // Independent session state per channel
    let mut sessions: [HashSession; 6] = [
        HashSession::None,
        HashSession::None,
        HashSession::None,
        HashSession::None,
        HashSession::None,
        HashSession::None,
    ];
    let mut backend = RustCryptoBackend::new();

    // Register all channels in the WaitGroup
    for (chan, ud) in CHANNELS {
        syscall::wait_group_add(handle::WG, chan, Signals::READABLE, ud)?;
    }

    loop {
        let ev = syscall::object_wait(handle::WG, Signals::READABLE, Instant::MAX)?;
        let idx = ev.user_data;
        let chan = CHANNELS[idx].0;

        let len = syscall::channel_read(chan, 0, &mut request_buf)?;

        if len < CryptoRequestHeader::SIZE {
            let header = CryptoResponseHeader::error(CryptoError::InvalidDataLength);
            let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
            response_buf[..CryptoResponseHeader::SIZE].copy_from_slice(header_bytes);
            syscall::channel_respond(chan, &response_buf[..CryptoResponseHeader::SIZE])?;
            continue;
        }

        let response_len = dispatch_crypto_op(
            &request_buf[..len],
            &mut response_buf,
            &mut sessions[idx],
            &mut backend,
        );
        syscall::channel_respond(chan, &response_buf[..response_len])?;
    }
}

fn dispatch_crypto_op(
    request: &[u8],
    response: &mut [u8],
    session: &mut HashSession,
    backend: &mut RustCryptoBackend,
) -> usize {
    // Parse header
    let header_bytes = &request[..CryptoRequestHeader::SIZE];
    let Some(header) = zerocopy::Ref::<_, CryptoRequestHeader>::from_bytes(header_bytes).ok() else {
        return encode_error(response, CryptoError::InvalidDataLength);
    };
    let header: &CryptoRequestHeader = &*header;

    let op = match header.operation() {
        Ok(op) => op,
        Err(e) => return encode_error(response, e),
    };

    // Extract key, nonce, and data from payload
    let payload = &request[CryptoRequestHeader::SIZE..];
    let key_len = header.key_length();
    let nonce_len = header.nonce_length();
    let data_len = header.data_length();

    if payload.len() < key_len + nonce_len + data_len {
        return encode_error(response, CryptoError::InvalidDataLength);
    }

    let key = &payload[..key_len];
    let nonce = &payload[key_len..key_len + nonce_len];
    let data = &payload[key_len + nonce_len..key_len + nonce_len + data_len];

    match op {
        // One-shot hash via backend traits
        CryptoOp::Sha256Hash => do_oneshot::<Sha256Marker>(backend, op, key, nonce, data, response),
        CryptoOp::Sha384Hash => do_oneshot::<Sha384Marker>(backend, op, key, nonce, data, response),
        CryptoOp::Sha512Hash => do_oneshot::<Sha512Marker>(backend, op, key, nonce, data, response),

        // Streaming hash - SHA-256 via backend Streaming trait
        CryptoOp::Sha256Begin => do_sha256_begin(backend, session, response),
        CryptoOp::Sha256Update => do_sha256_update(backend, session, data, response),
        CryptoOp::Sha256Finish => do_sha256_finish(backend, session, response),

        // Streaming hash - SHA-384 via backend Streaming trait
        CryptoOp::Sha384Begin => do_sha384_begin(backend, session, response),
        CryptoOp::Sha384Update => do_sha384_update(backend, session, data, response),
        CryptoOp::Sha384Finish => do_sha384_finish(backend, session, response),

        // Streaming hash - SHA-512 via backend Streaming trait
        CryptoOp::Sha512Begin => do_sha512_begin(backend, session, response),
        CryptoOp::Sha512Update => do_sha512_update(backend, session, data, response),
        CryptoOp::Sha512Finish => do_sha512_finish(backend, session, response),

        // HMAC via backend traits
        CryptoOp::HmacSha256 => do_oneshot::<HmacSha256Marker>(backend, op, key, nonce, data, response),
        CryptoOp::HmacSha384 => do_oneshot::<HmacSha384Marker>(backend, op, key, nonce, data, response),
        CryptoOp::HmacSha512 => do_oneshot::<HmacSha512Marker>(backend, op, key, nonce, data, response),

        // AES-GCM via backend traits
        CryptoOp::Aes256GcmEncrypt => do_oneshot::<Aes256GcmEncrypt>(backend, op, key, nonce, data, response),
        CryptoOp::Aes256GcmDecrypt => do_oneshot::<Aes256GcmDecrypt>(backend, op, key, nonce, data, response),

        // RNG via backend traits
        CryptoOp::GetRandomBytes => do_oneshot::<GetRandomBytes>(backend, op, key, nonce, data, response),

        // ECDSA not yet migrated to backend traits
        CryptoOp::EcdsaP256Sign | CryptoOp::EcdsaP256Verify |
        CryptoOp::EcdsaP384Sign | CryptoOp::EcdsaP384Verify => {
            encode_error(response, CryptoError::InvalidOperation)
        }
    }
}

/// Generic one-shot operation via backend trait.
fn do_oneshot<A>(
    backend: &RustCryptoBackend,
    op: CryptoOp,
    key: &[u8],
    nonce: &[u8],
    data: &[u8],
    response: &mut [u8],
) -> usize
where
    A: crypto_api::backend::Algorithm,
    RustCryptoBackend: OneShot<A>,
{
    use crypto_api::backend::CryptoInput;
    
    let input = CryptoInput::from_wire(op, key, nonce, data);
    let output = &mut response[CryptoResponseHeader::SIZE..];
    
    match backend.compute(&input, output) {
        Ok(len) => {
            let header = CryptoResponseHeader::success(len as u16);
            let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
            response[..CryptoResponseHeader::SIZE].copy_from_slice(header_bytes);
            CryptoResponseHeader::SIZE + len
        }
        Err(e) => encode_error(response, e.into()),
    }
}

fn encode_error(response: &mut [u8], err: CryptoError) -> usize {
    let header = CryptoResponseHeader::error(err);
    let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
    response[..CryptoResponseHeader::SIZE].copy_from_slice(header_bytes);
    CryptoResponseHeader::SIZE
}

fn encode_success(response: &mut [u8], result: &[u8]) -> usize {
    let header = CryptoResponseHeader::success(result.len() as u16);
    let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
    response[..CryptoResponseHeader::SIZE].copy_from_slice(header_bytes);
    response[CryptoResponseHeader::SIZE..CryptoResponseHeader::SIZE + result.len()]
        .copy_from_slice(result);
    CryptoResponseHeader::SIZE + result.len()
}

// ---------------------------------------------------------------------------
// Streaming hash operations via backend Streaming<A> trait
// ---------------------------------------------------------------------------

// SHA-256 streaming
fn do_sha256_begin(
    backend: &mut RustCryptoBackend,
    session: &mut HashSession,
    response: &mut [u8],
) -> usize {
    if !session.is_none() {
        return encode_error(response, CryptoError::SessionBusy);
    }
    match <RustCryptoBackend as Streaming<Sha256Marker>>::begin(backend) {
        Ok(s) => {
            *session = HashSession::Sha256(s);
            encode_success(response, &[])
        }
        Err(e) => encode_error(response, e.into()),
    }
}

fn do_sha256_update(
    backend: &mut RustCryptoBackend,
    session: &mut HashSession,
    data: &[u8],
    response: &mut [u8],
) -> usize {
    match session {
        HashSession::Sha256(s) => {
            match <RustCryptoBackend as Streaming<Sha256Marker>>::feed(backend, s, data) {
                Ok(()) => encode_success(response, &[]),
                Err(e) => encode_error(response, e.into()),
            }
        }
        _ => encode_error(response, CryptoError::SessionNotFound),
    }
}

fn do_sha256_finish(
    backend: &mut RustCryptoBackend,
    session: &mut HashSession,
    response: &mut [u8],
) -> usize {
    let HashSession::Sha256(s) = core::mem::replace(session, HashSession::None) else {
        return encode_error(response, CryptoError::SessionNotFound);
    };
    let output = &mut response[CryptoResponseHeader::SIZE..];
    match <RustCryptoBackend as Streaming<Sha256Marker>>::finish(backend, s, output) {
        Ok(len) => {
            let header = CryptoResponseHeader::success(len as u16);
            let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
            response[..CryptoResponseHeader::SIZE].copy_from_slice(header_bytes);
            CryptoResponseHeader::SIZE + len
        }
        Err(e) => encode_error(response, e.into()),
    }
}

// SHA-384 streaming
fn do_sha384_begin(
    backend: &mut RustCryptoBackend,
    session: &mut HashSession,
    response: &mut [u8],
) -> usize {
    if !session.is_none() {
        return encode_error(response, CryptoError::SessionBusy);
    }
    match <RustCryptoBackend as Streaming<Sha384Marker>>::begin(backend) {
        Ok(s) => {
            *session = HashSession::Sha384(s);
            encode_success(response, &[])
        }
        Err(e) => encode_error(response, e.into()),
    }
}

fn do_sha384_update(
    backend: &mut RustCryptoBackend,
    session: &mut HashSession,
    data: &[u8],
    response: &mut [u8],
) -> usize {
    match session {
        HashSession::Sha384(s) => {
            match <RustCryptoBackend as Streaming<Sha384Marker>>::feed(backend, s, data) {
                Ok(()) => encode_success(response, &[]),
                Err(e) => encode_error(response, e.into()),
            }
        }
        _ => encode_error(response, CryptoError::SessionNotFound),
    }
}

fn do_sha384_finish(
    backend: &mut RustCryptoBackend,
    session: &mut HashSession,
    response: &mut [u8],
) -> usize {
    let HashSession::Sha384(s) = core::mem::replace(session, HashSession::None) else {
        return encode_error(response, CryptoError::SessionNotFound);
    };
    let output = &mut response[CryptoResponseHeader::SIZE..];
    match <RustCryptoBackend as Streaming<Sha384Marker>>::finish(backend, s, output) {
        Ok(len) => {
            let header = CryptoResponseHeader::success(len as u16);
            let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
            response[..CryptoResponseHeader::SIZE].copy_from_slice(header_bytes);
            CryptoResponseHeader::SIZE + len
        }
        Err(e) => encode_error(response, e.into()),
    }
}

// SHA-512 streaming
fn do_sha512_begin(
    backend: &mut RustCryptoBackend,
    session: &mut HashSession,
    response: &mut [u8],
) -> usize {
    if !session.is_none() {
        return encode_error(response, CryptoError::SessionBusy);
    }
    match <RustCryptoBackend as Streaming<Sha512Marker>>::begin(backend) {
        Ok(s) => {
            *session = HashSession::Sha512(s);
            encode_success(response, &[])
        }
        Err(e) => encode_error(response, e.into()),
    }
}

fn do_sha512_update(
    backend: &mut RustCryptoBackend,
    session: &mut HashSession,
    data: &[u8],
    response: &mut [u8],
) -> usize {
    match session {
        HashSession::Sha512(s) => {
            match <RustCryptoBackend as Streaming<Sha512Marker>>::feed(backend, s, data) {
                Ok(()) => encode_success(response, &[]),
                Err(e) => encode_error(response, e.into()),
            }
        }
        _ => encode_error(response, CryptoError::SessionNotFound),
    }
}

fn do_sha512_finish(
    backend: &mut RustCryptoBackend,
    session: &mut HashSession,
    response: &mut [u8],
) -> usize {
    let HashSession::Sha512(s) = core::mem::replace(session, HashSession::None) else {
        return encode_error(response, CryptoError::SessionNotFound);
    };
    let output = &mut response[CryptoResponseHeader::SIZE..];
    match <RustCryptoBackend as Streaming<Sha512Marker>>::finish(backend, s, output) {
        Ok(len) => {
            let header = CryptoResponseHeader::success(len as u16);
            let header_bytes = zerocopy::IntoBytes::as_bytes(&header);
            response[..CryptoResponseHeader::SIZE].copy_from_slice(header_bytes);
            CryptoResponseHeader::SIZE + len
        }
        Err(e) => encode_error(response, e.into()),
    }
}

#[entry]
fn entry() -> ! {
    if let Err(e) = crypto_server_loop() {
        pw_log::error!("Crypto server error: {}", e as u32);
        let _ = syscall::debug_shutdown(Err(e));
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

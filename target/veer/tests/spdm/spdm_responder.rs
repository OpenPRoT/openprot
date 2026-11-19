// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! SPDM responder: answers SPDM requests as a client of the MCTP server.
//!
//! The responder runs the `//services/spdm/responder` service over MCTP. It
//! has one certificate chain and no measurements, and it does not sign. It
//! supports `GET_VERSION`, `GET_CAPABILITIES`, `NEGOTIATE_ALGORITHMS`,
//! `GET_DIGESTS` and `GET_CERTIFICATE`. `platform.rs` has the hash, random
//! number generator, certificate store and evidence source that the service
//! needs.

#![no_main]
#![no_std]

mod platform;

use kernel_config::{MBOX_BASE, SOC_IFC_BASE};
use openprot_hal_blocking::certificate::Concat;
use openprot_mctp_api::Stack;
use openprot_mctp_client_ipc::IpcMctpClient;
use openprot_spdm_responder::{ResponderConfig, SpdmResponder};
use openprot_spdm_transport_mctp::MctpSpdmTransport;
use platform::{CaliptraHash, CaliptraRandom, NoEvidence, OneSlot, SpdmCertificates};
use spdm_lib::codec::MessageBuf;
use spdm_lib::error::SpdmError;
use spdm_lib::platform::transport::SpdmTransport as _;
use spdm_lib::protocol::CapabilityFlags;
use spdm_responder_codegen::handle;
use userspace::process_entry;
use userspace::syscall;
use veer_dpe::CaliptraDpeChain;
use veer_mailbox::Mailbox;

/// Size in bytes of the buffer for one SPDM message.
const MESSAGE_BUF: usize = 2048;

/// The largest SPDM message that the responder sends or receives, which
/// fits in the 1032 bytes that the MCTP server accepts.
const MAX_SPDM_MESSAGE: u32 = 1024;

/// Size in bytes of the buffer for the leaf certificate.
const LEAF_BUF: usize = 2048;

/// Logs `reason` and stops the test with a failure.
fn fail(reason: &'static str) -> ! {
    pw_log::error!("SPDM responder failed: {}", reason as &str);
    let _ = syscall::debug_shutdown(Err(pw_status::Error::Internal));
    #[expect(clippy::empty_loop)]
    loop {}
}

#[process_entry("spdm_responder")]
fn entry() {
    // SAFETY: the system config maps the registers for this process, and
    // this image has no other user of the mailbox.
    let mailbox = unsafe { Mailbox::new(MBOX_BASE, SOC_IFC_BASE) };

    let stack = Stack::new(IpcMctpClient::new(handle::MCTP));
    let mut transport = MctpSpdmTransport::new_responder(&stack);
    if transport.init_sequence().is_err() {
        fail("no MCTP listener");
    }

    // The chain of slot 0: the chain of Caliptra's DPE, which is read from
    // Caliptra in parts, then the leaf certificate, which is kept in memory.
    let mut leaf = [0u8; LEAF_BUF];
    let Some(leaf) = platform::certify_leaf(&mailbox, &mut leaf) else {
        fail("no leaf certificate");
    };
    let Ok(dpe_chain) = CaliptraDpeChain::new(&mailbox) else {
        fail("no DPE certificate chain");
    };
    let store = OneSlot::new(Concat::new(dpe_chain, leaf));
    let Some(mut cert_store) = SpdmCertificates::new(store, &mailbox) else {
        fail("no root certificate hash");
    };
    pw_log::info!(
        "spdm responder: certificate chain of {} bytes, leaf of {} bytes",
        cert_store.chain_len().unwrap_or(0) as u32,
        leaf.len() as u32
    );

    let mut hash = CaliptraHash::new(&mailbox);
    let mut m1_hash = CaliptraHash::new(&mailbox);
    let mut l1_hash = CaliptraHash::new(&mailbox);
    let mut rng = CaliptraRandom::new(&mailbox);
    let evidence = NoEvidence;

    // The responder has certificates. It does not sign and has no
    // measurements.
    let mut capabilities = ResponderConfig::default_capabilities();
    capabilities.flags = CapabilityFlags::default();
    capabilities.flags.set_cert_cap(1);
    capabilities.data_transfer_size = MAX_SPDM_MESSAGE;
    capabilities.max_spdm_msg_size = MAX_SPDM_MESSAGE;
    let config = ResponderConfig {
        capabilities: Some(capabilities),
        algorithms: None,
    };

    let mut responder = match SpdmResponder::new(
        &mut transport,
        &mut cert_store,
        &mut hash,
        &mut m1_hash,
        &mut l1_hash,
        &mut rng,
        &evidence,
        Some(config),
    ) {
        Ok(responder) => responder,
        Err(_) => fail("no SPDM context"),
    };

    let mut buffer = [0u8; MESSAGE_BUF];
    let mut message = MessageBuf::new(&mut buffer);

    pw_log::info!("spdm responder: waiting for request");
    loop {
        message.reset();
        match responder
            .context_mut()
            .responder_process_message(&mut message)
        {
            Ok(()) => {}
            // The service did not accept the request. It has sent an SPDM
            // error response if the request allows one.
            Err(SpdmError::Command(_)) => {
                pw_log::warn!("spdm responder: request not accepted")
            }
            Err(_) => fail("request not processed"),
        }
    }
}

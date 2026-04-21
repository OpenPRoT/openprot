// Licensed under the Apache-2.0 license

//! MCTP Server — IPC Dispatch Loop
//!
//! Userspace service that receives MCTP requests over a Pigweed IPC channel,
//! dispatches them to the MCTP server core, and responds with results.
//!
//! # Build Modes
//!
//! Two mutually exclusive event loops are compiled depending on feature flags:
//!
//! | Features | Mode | IPC served | SPDM in-process |
//! |---|---|---|---|
//! | _(none)_ | Notification (WaitGroup + IRQ) | Yes | No |
//! | `i2c-polling` | Polling | No | No |
//! | `i2c-polling` + `direct-client` | Polling + SPDM | No | Yes |
//!
//! # Notification Mode Architecture (default)
//!
//! ```text
//! ┌─ Client ──────────────────────────┐
//! │ channel_transact(request)         │
//! └──────────────┬────────────────────┘
//!                │ IPC channel
//!                ▼
//! ┌─ This Server ─────────────────────┐
//! │ object_wait(WG, READABLE)         │
//! │  ├─ user_data=0: IPC client       │
//! │  │   channel_read / dispatch /    │
//! │  │   channel_respond              │
//! │  └─ user_data=1: I2C slave IRQ    │
//! │      get_pending_messages         │
//! │      receiver.decode → inbound()  │
//! └──────────────┬────────────────────┘
//!                │ mctp-stack Router
//!                ▼
//! ┌─ I2C Transport ──────────────────┐
//! │ I2cSender → I2C Server IPC      │
//! └──────────────────────────────────┘
//! ```
//!
//! # In-Process SPDM Responder Architecture (`i2c-polling` + `direct-client`)
//!
//! When both features are enabled the SPDM responder runs inside this process.
//! `DirectMctpClient` replaces the IPC channel — it calls `Server` methods
//! directly via a `RefCell`. No separate `spdm_responder` app process is needed.
//!
//! ```text
//! ┌─ I2C Hardware ───────────────────────────────────────┐
//! │  Slave-mode frames arrive on bus 2                   │
//! └──────────────┬───────────────────────────────────────┘
//!                │ wait_for_messages() → TargetMessage
//!                ▼
//! ┌─ This Process ───────────────────────────────────────┐
//! │  Phase 1: receiver.decode() → raw MCTP packet        │
//! │           server_cell.borrow_mut().inbound(pkt)      │
//! │               └─ Router reassembles fragments        │
//! │                                                      │
//! │  Phase 2: spdm_ctx.responder_process_message()       │
//! │    │  MctpSpdmTransport::recv_request()              │
//! │    │    DirectMctpClient::recv()                     │
//! │    │      server_cell.borrow_mut().try_recv()        │
//! │    │        └─ returns Ok(msg) or Err(TimedOut)      │
//! │    └─ on TimedOut: no-op, loop continues             │
//! │    └─ on Ok: SPDM state machine processes request    │
//! │      MctpSpdmTransport::send_response()              │
//! │        DirectMctpClient::send()                      │
//! │          server_cell.borrow_mut().send()             │
//! │            I2cSender → I2C Server IPC → wire        │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! # IPC Pattern (notification mode)
//!
//! 1. `object_wait(WG, READABLE)` — block until an event fires
//! 2. `channel_read(MCTP, ...)` — read raw request bytes from IPC client
//! 3. `dispatch_mctp_op(...)` — decode and execute the MCTP operation
//! 4. `channel_respond(MCTP, ...)` — send response back to client
//!
//! # Handle Binding
//!
//! Handles are provided by the `app_package` Bazel rule, generated from
//! `system.json5` into `app_mctp_server::handle::{I2C, MCTP, WG}`.


#![no_main]
#![no_std]

// Imports shared by both modes.
use i2c_api::{BusIndex, I2cAddress, I2cTargetClient, TargetMessage};
use i2c_client::IpcI2cClient;
use openprot_mctp_transport_i2c::{I2cSender, MctpI2cReceiver};
use openprot_mctp_api::wire::MAX_PAYLOAD_SIZE;
use pw_status::Result;
use userspace::entry;
use userspace::syscall;
use app_mctp_server::handle;

// Imports used only by the in-process SPDM responder (i2c-polling + direct-client).
#[cfg(feature = "i2c-polling")]
use core::cell::RefCell;
#[cfg(all(feature = "i2c-polling", feature = "direct-client"))]
use mock_platform::{MockCertStore, MockEvidence, MockHash, MockRng};
#[cfg(all(feature = "i2c-polling", feature = "direct-client"))]
use openprot_mctp_server::direct_client::DirectMctpClient;
#[cfg(all(feature = "i2c-polling", feature = "direct-client"))]
use openprot_spdm_transport_mctp::MctpSpdmTransport;
#[cfg(all(feature = "i2c-polling", feature = "direct-client"))]
use spdm_lib::codec::MessageBuf;
#[cfg(all(feature = "i2c-polling", feature = "direct-client"))]
use spdm_lib::context::SpdmContext;
#[cfg(all(feature = "i2c-polling", feature = "direct-client"))]
use spdm_lib::platform::transport::SpdmTransport;
#[cfg(all(feature = "i2c-polling", feature = "direct-client"))]
use spdm_lib::protocol::{
    AeadCipherSuite, AlgorithmPriorityTable, BaseAsymAlgo, BaseHashAlgo, CapabilityFlags,
    DeviceAlgorithms, DeviceCapabilities, DheNamedGroup, KeySchedule, LocalDeviceAlgorithms,
    MeasurementHashAlgo, MeasurementSpecification, MelSpecification, OtherParamSupport,
    ReqBaseAsymAlg, SpdmVersion,
};

// Imports used only by the notification (WaitGroup + IRQ) loop.
#[cfg(not(feature = "i2c-polling"))]
use openprot_mctp_api::wire::{MctpRequestHeader, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE};
#[cfg(not(feature = "i2c-polling"))]
use openprot_mctp_api::ResponseCode;
#[cfg(not(feature = "i2c-polling"))]
use openprot_mctp_server::dispatch;
#[cfg(not(feature = "i2c-polling"))]
use userspace::syscall::Signals;
#[cfg(not(feature = "i2c-polling"))]
use userspace::time::Instant;

const OWN_EID: u8 = 8;
const OWN_I2C_ADDR: u8 = 0x10;

// ---------------------------------------------------------------------------
// Polling mode — blocks on wait_for_messages(); no WaitGroup or IRQ needed.
// When built with the "direct-client" feature, an SPDM responder runs
// in-process using DirectMctpClient (no IPC channel to a separate process).
// Enable with crate_features = ["i2c-polling", "direct-client"] in Bazel.
// ---------------------------------------------------------------------------

#[cfg(feature = "i2c-polling")]
fn mctp_loop() -> Result<()> {
    pw_log::info!("MCTP server starting (I2C polling mode, eid=0x{:02x} addr=0x{:02x})",
        OWN_EID as u32, OWN_I2C_ADDR as u32);

    let mut i2c = IpcI2cClient::new(handle::I2C);

    pw_log::info!("MCTP server: configuring I2C target address 0x{:02x} on bus 2",
        OWN_I2C_ADDR as u32);
    let addr = I2cAddress::new(OWN_I2C_ADDR).map_err(|_| {
        pw_log::error!("MCTP server: invalid I2C address 0x{:02x}", OWN_I2C_ADDR as u32);
        pw_status::Error::InvalidArgument
    })?;
    i2c.configure_target_address(BusIndex::BUS_2, addr).map_err(|_| {
        pw_log::error!("MCTP server: configure_target_address failed");
        pw_status::Error::Internal
    })?;
    pw_log::info!("MCTP server: enabling I2C receive on bus 2");
    i2c.enable_receive(BusIndex::BUS_2).map_err(|_| {
        pw_log::error!("MCTP server: enable_receive failed");
        pw_status::Error::Internal
    })?;

    let sender = I2cSender::new(IpcI2cClient::new(handle::I2C), BusIndex::BUS_2, OWN_I2C_ADDR);
    let receiver = MctpI2cReceiver::new(OWN_I2C_ADDR);

    // RefCell lets DirectMctpClient borrow `server` alongside the I2C path in
    // the same polling loop.  A single name is used in both cfg variants so
    // every call site is uniform.
    let server = RefCell::new(openprot_mctp_server::Server::<_, 16>::new(
        mctp::Eid(OWN_EID),
        0,
        sender,
    ));

    // ---------------------------------------------------------------------------
    // SPDM responder setup (only when direct-client feature is enabled)
    // ---------------------------------------------------------------------------
    #[cfg(feature = "direct-client")]
    let mut transport = {
        let client = DirectMctpClient::new(&server);
        MctpSpdmTransport::new_responder(client)
    };

    #[cfg(feature = "direct-client")]
    {
        pw_log::info!("MCTP server: registering SPDM listener (msg_type=0x05)");
        if transport.init_sequence().is_err() {
            pw_log::error!("MCTP server: S  PDM transport init_sequence failed — \
                listener(0x05) rejected; router listener table may be full");
            return Err(pw_status::Error::Internal);
        }
        pw_log::info!("MCTP server: SPDM listener registered");
    }

    #[cfg(feature = "direct-client")]
    let mut cert_store = MockCertStore::new();
    #[cfg(feature = "direct-client")]
    let mut hash = MockHash::new();
    #[cfg(feature = "direct-client")]
    let mut m1_hash = MockHash::new();
    #[cfg(feature = "direct-client")]
    let mut l1_hash = MockHash::new();
    #[cfg(feature = "direct-client")]
    let mut rng = MockRng::new();
    #[cfg(feature = "direct-client")]
    let evidence = MockEvidence::new();

    #[cfg(feature = "direct-client")]
    let mut flags = CapabilityFlags::default();
    #[cfg(feature = "direct-client")]
    {
        flags.set_cert_cap(1);
        flags.set_chal_cap(1);
        flags.set_meas_cap(2);
        flags.set_meas_fresh_cap(1);
        flags.set_chunk_cap(1);
    }

    #[cfg(feature = "direct-client")]
    let capabilities = DeviceCapabilities {
        ct_exponent: 0,
        flags,
        data_transfer_size: 1024,
        max_spdm_msg_size: 4096,
        include_supported_algorithms: true,
    };

    #[cfg(feature = "direct-client")]
    static SUPPORTED_VERSIONS: [SpdmVersion; 2] = [SpdmVersion::V12, SpdmVersion::V13];

    #[cfg(feature = "direct-client")]
    let algorithms = {
        let mut measurement_spec = MeasurementSpecification::default();
        measurement_spec.set_dmtf_measurement_spec(1);
        let mut measurement_hash_algo = MeasurementHashAlgo::default();
        measurement_hash_algo.set_tpm_alg_sha_384(1);
        let mut base_asym_algo = BaseAsymAlgo::default();
        base_asym_algo.set_tpm_alg_ecdsa_ecc_nist_p384(1);
        let mut base_hash_algo = BaseHashAlgo::default();
        base_hash_algo.set_tpm_alg_sha_384(1);
        let device_algorithms = DeviceAlgorithms {
            measurement_spec,
            other_param_support: OtherParamSupport::default(),
            measurement_hash_algo,
            base_asym_algo,
            base_hash_algo,
            mel_specification: MelSpecification::default(),
            dhe_group: DheNamedGroup::default(),
            aead_cipher_suite: AeadCipherSuite::default(),
            req_base_asym_algo: ReqBaseAsymAlg::default(),
            key_schedule: KeySchedule::default(),
        };
        LocalDeviceAlgorithms {
            device_algorithms,
            algorithm_priority_table: AlgorithmPriorityTable {
                measurement_specification: None,
                opaque_data_format: None,
                base_asym_algo: None,
                base_hash_algo: None,
                mel_specification: None,
                dhe_group: None,
                aead_cipher_suite: None,
                req_base_asym_algo: None,
                key_schedule: None,
            },
        }
    };

    #[cfg(feature = "direct-client")]
    let mut spdm_ctx = {
        pw_log::info!("MCTP server: creating SPDM context (v1.2+v1.3, ECDSA-P384, SHA-384)");
        match SpdmContext::new(
            &SUPPORTED_VERSIONS,
            &mut transport,
            capabilities,
            algorithms,
            &mut cert_store,
            None,
            &mut hash,
            &mut m1_hash,
            &mut l1_hash,
            &mut rng,
            &evidence,
        ) {
            Ok(ctx) => {
                pw_log::info!("MCTP server: SPDM context ready");
                ctx
            }
            Err(_) => {
                pw_log::error!("MCTP server: SpdmContext::new failed — \
                    check cert_store, hash, rng platform impls");
                return Err(pw_status::Error::Internal);
            }
        }
    };

    #[cfg(feature = "direct-client")]
    let mut spdm_buf = [0u8; MAX_PAYLOAD_SIZE];
    // MessageBuf is created once outside the loop; reset() is called each iteration.
    // Creating it inside the loop would leave the mutable borrow of spdm_buf live
    // across loop iterations, which the borrow checker rejects (E0499).
    #[cfg(feature = "direct-client")]
    let mut msg_buf = MessageBuf::new(&mut spdm_buf);

    // Fault-isolation counters — logged on first error and at every 16th recurrence.
    let mut i2c_recv_err: u32 = 0;
    let mut decode_err: u32 = 0;
    let mut inbound_err: u32 = 0;
    #[cfg(feature = "direct-client")]
    let mut spdm_ok: u32 = 0;
    #[cfg(feature = "direct-client")]
    let mut spdm_err: u32 = 0;
    let mut i2c_pkt: u32 = 0;
    let mut idle_polls: u32 = 0;

    pw_log::info!("MCTP server ready, polling for I2C packets");

    loop {
        // Phase 1: drain inbound I2C packets into the MCTP router.
        let mut msgs = [TargetMessage::default(); 1];

        match i2c.wait_for_messages(BusIndex::BUS_2, &mut msgs, None) {
            Ok(n) => {
                for msg in msgs.get(..n).unwrap_or(&[]) {
                    let raw = msg.data();

                    // WORKAROUND: Manually prepend destination address to I2C frame.
                    // The I2C hardware currently does NOT prepend the destination address,
                    // but mctp-lib's I2C decoder expects it. This will be fixed when the
                    // I2C driver is updated to prepend the destination address automatically.
                    // TODO: Remove this workaround once I2C driver change is in place.
                    let mut frame_with_dest = [0u8; 256];
                    if raw.len() + 1 > frame_with_dest.len() {
                        pw_log::warn!(
                            "I2C frame too large ({} bytes), skipping",
                            raw.len() as u32,
                        );
                        continue;
                    }

                    // Prepend destination address (OWN_I2C_ADDR << 1 | 0 for write)
                    frame_with_dest[0] = OWN_I2C_ADDR << 1;
                    frame_with_dest[1..raw.len() + 1].copy_from_slice(raw);
                    let frame_len = raw.len() + 1;

                    // Log the frame AFTER prepending destination
                    // Format: dest_addr cmd byte_count src_addr | MCTP hdr[0..3]
                    if frame_len >= 9 {
                        pw_log::info!(
                            "I2C frame (with prepended dest): dest=0x{:02x} cmd=0x{:02x} bc={} src=0x{:02x} \
                            | mctp: ver=0x{:02x} deid=0x{:02x} seid=0x{:02x} flags=0x{:02x} \
                            len={}",
                            frame_with_dest[0] as u32, frame_with_dest[1] as u32,
                            frame_with_dest[2] as u32, frame_with_dest[3] as u32,
                            frame_with_dest[4] as u32, frame_with_dest[5] as u32,
                            frame_with_dest[6] as u32, frame_with_dest[7] as u32,
                            frame_len as u32,
                        );
                    } else {
                        pw_log::warn!(
                            "I2C frame too short ({} bytes) to contain MCTP header",
                            frame_len as u32,
                        );
                    }

                    // Create temporary TargetMessage with prepended destination
                    let msg_with_dest = i2c_api::TargetMessage::from_data(
                        msg.source_address,
                        &frame_with_dest[..frame_len]
                    );

                    match receiver.decode(&msg_with_dest) {
                        Ok((pkt, src_addr)) => {
                            i2c_pkt = i2c_pkt.wrapping_add(1);
                            // Log MCTP packet fields: SOM/EOM from flags byte (pkt[3])
                            let som = pkt.get(3).map_or(0u8, |b| (b >> 7) & 1);
                            let eom = pkt.get(3).map_or(0u8, |b| (b >> 6) & 1);
                            let seq = pkt.get(3).map_or(0u8, |b| (b >> 4) & 0x3);
                            let msg_type = pkt.get(4).map_or(0u8, |b| b & 0x7f);
                            pw_log::info!(
                                "MCTP pkt #{}: src_i2c=0x{:02x} dest_i2c=0x{:02x} bc={} len={} \
                                SOM={} EOM={} seq={} msg_type=0x{:02x}",
                                i2c_pkt as u32,
                                src_addr.source as u32,
                                src_addr.dest as u32,
                                src_addr.byte_count as u32,
                                pkt.len() as u32,
                                som as u32, eom as u32, seq as u32, msg_type as u32,
                            );
                            match server.borrow_mut().inbound(pkt) {
                                Ok(Some(cookie)) => {
                                    pw_log::debug!(
                                        "MCTP pkt #{} associated with cookie {}",
                                        i2c_pkt as u32,
                                        cookie.0 as u32,
                                    );
                                }
                                Ok(None) => {
                                    pw_log::debug!("MCTP pkt #{} discarded (no matching handler)", i2c_pkt as u32);
                                }
                                Err(e) => {
                                    inbound_err = inbound_err.wrapping_add(1);
                                    if inbound_err == 1 || inbound_err & 0xf == 0 {
                                        pw_log::error!(
                                            "MCTP server: inbound() error code={} \
                                            total_inbound_errors={}",
                                            e.code as u32,
                                            inbound_err as u32,
                                        );
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            decode_err = decode_err.wrapping_add(1);
                            if decode_err == 1 || decode_err & 0xf == 0 {
                                pw_log::error!(
                                    "MCTP server: I2C frame decode failed \
                                    total_decode_errors={}",
                                    decode_err as u32,
                                );
                            }
                        }
                    }
                }
            }
            Err(e) if e.is_timeout() => {
                // Timeout is the normal "no frame yet" result from the backend's
                // poll-budget loop — not a real error.  Just proceed to Phase 2
                // (SPDM responder poll) and loop again.
                idle_polls = idle_polls.wrapping_add(1);
                if idle_polls & 0xfff == 0 {
                    pw_log::info!(
                        "MCTP alive: idle_polls={} pkts={}",
                        idle_polls as u32,
                        i2c_pkt as u32,
                    );
                }
            }
            Err(_) => {
                i2c_recv_err = i2c_recv_err.wrapping_add(1);
                if i2c_recv_err == 1 || i2c_recv_err & 0xf == 0 {
                    pw_log::error!(
                        "MCTP server: wait_for_messages failed \
                        total_i2c_recv_errors={}",
                        i2c_recv_err as u32,
                    );
                }
            }
        }

        // Phase 2: poll the SPDM responder for an assembled message.
        // Transport returning TimedOut means no SPDM message is assembled yet
        // — this is the normal steady-state; log only genuine protocol errors.
        #[cfg(feature = "direct-client")]
        {
            msg_buf.reset();
            match spdm_ctx.responder_process_message(&mut msg_buf) {
                Ok(_) => {
                    spdm_ok = spdm_ok.wrapping_add(1);
                    pw_log::info!(
                        "MCTP server: SPDM request processed ok total_spdm_ok={}",
                        spdm_ok as u32,
                    );
                }
                Err(_) => {
                    // Most errors here are TransportError::NoRequestInFlight
                    // (TimedOut from DirectMctpClient::recv — no message ready).
                    // Count and log only on first occurrence and every 256th after,
                    // to avoid flooding the log during idle polling.
                    spdm_err = spdm_err.wrapping_add(1);
                    if spdm_err == 1 || spdm_err & 0xff == 0 {
                        pw_log::debug!(
                            "MCTP server: responder_process_message returned Err \
                            (likely no message yet) total_spdm_err={}",
                            spdm_err as u32,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Notification mode — WaitGroup multiplexes IPC (MCTP channel) and the I2C
// slave interrupt (USER signal).  Serves both local IPC MCTP clients and
// inbound I2C transport.  Default when "i2c-polling" feature is not set.
// Requires WG and I2C2_IRQ objects in system.json5.
// ---------------------------------------------------------------------------

#[cfg(not(feature = "i2c-polling"))]
fn mctp_loop() -> Result<()> {
    pw_log::info!("MCTP server starting");

    // I2C notification client: receives slave-mode interrupts via Signals::USER.
    let mut i2c_notify = IpcI2cClient::new(handle::I2C);

    // Configure I2C bus 2 as slave at our address
    let addr = I2cAddress::new(OWN_I2C_ADDR).map_err(|_| pw_status::Error::InvalidArgument)?;
    i2c_notify
        .configure_target_address(BusIndex::BUS_2, addr)
        .map_err(|_| pw_status::Error::Internal)?;

    // Enable slave receive mode
    i2c_notify
        .enable_receive(BusIndex::BUS_2)
        .map_err(|_| pw_status::Error::Internal)?;

    // Register for notifications
    i2c_notify
        .register_notification(BusIndex::BUS_2, 0)
        .map_err(|_| pw_status::Error::Internal)?;

    // Separate handle for the sender — I2cSender takes ownership.
    let sender = I2cSender::new(IpcI2cClient::new(handle::I2C), BusIndex::BUS_2, OWN_I2C_ADDR);
    let receiver = MctpI2cReceiver::new(OWN_I2C_ADDR);
    let mut server = openprot_mctp_server::Server::<_, 16>::new(
        mctp::Eid(OWN_EID),
        0,
        sender,
    );

    let mut request_buf = [0u8; MAX_REQUEST_SIZE];
    let mut response_buf = [0u8; MAX_RESPONSE_SIZE];
    let mut recv_buf = [0u8; MAX_PAYLOAD_SIZE];

    // Register both event sources with the WaitGroup.
    // user_data=0 → IPC from a client  (MCTP channel READABLE)
    // user_data=1 → I2C slave notification (I2C channel USER)
    syscall::wait_group_add(handle::WG, handle::MCTP, Signals::READABLE, 0usize)?;
    syscall::wait_group_add(handle::WG, handle::I2C,  Signals::USER,     1usize)?;

    pw_log::info!("MCTP server ready, entering event loop");

    loop {
        let ev = syscall::object_wait(handle::WG, Signals::READABLE, Instant::MAX)?;

        if ev.user_data == 1 {
            // Inbound I2C data: drain pending messages, decode I2C framing,
            // feed raw MCTP packets into the router.
            let mut msgs = [TargetMessage::default(); 1];
            if let Ok(n) = i2c_notify.get_pending_messages(BusIndex::BUS_2, &mut msgs) {
                for msg in &msgs[..n] {
                    if let Ok((pkt, _src_addr)) = receiver.decode(msg) {
                        // Feed packet to server; ignore result (no detailed logging in this path)
                        let _ = server.inbound(pkt);
                    }
                }
            }
        } else {
            // IPC from a client — channel_read is non-blocking here because
            // the WaitGroup only fires after READABLE is set.
            let len = syscall::channel_read(handle::MCTP, 0, &mut request_buf)?;

            if len < MctpRequestHeader::SIZE {
                // Truncated request — respond with error
                let resp = openprot_mctp_api::wire::MctpResponseHeader::error(ResponseCode::BadArgument);
                response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE]
                    .copy_from_slice(&resp.to_bytes());
                syscall::channel_respond(
                    handle::MCTP,
                    &response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE],
                )?;
                continue;
            }

            // Dispatch and respond
            let response_len = dispatch::dispatch_mctp_op(
                &request_buf[..len],
                &mut response_buf,
                &mut server,
                &mut recv_buf,
            );
            syscall::channel_respond(handle::MCTP, &response_buf[..response_len])?;
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[entry]
fn entry() -> ! {
    if let Err(e) = mctp_loop() {
        pw_log::error!("MCTP server error: {}", e as u32);
        let _ = syscall::debug_shutdown(Err(e));
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

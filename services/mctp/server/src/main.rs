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
//! | Features | Mode | IPC served | SPDM role in-process |
//! |---|---|---|---|
//! | _(none)_ | Notification (WaitGroup + IRQ) | Yes | none |
//! | `i2c-polling` | Polling | No | none |
//! | `i2c-polling` + `in-process-responder` | Polling + SPDM | No | responder |
//! | `i2c-polling` + `in-process-requester` | Polling + SPDM | No | requester (added in Phase 2+) |
//!
//! `in-process-requester` and `in-process-responder` are mutually
//! exclusive and enforced with a `compile_error!` below.
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
//! # In-Process SPDM Responder Architecture (`i2c-polling` + `in-process-responder`)
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

// Role-feature mutual exclusion: the in-process requester and responder
// cannot both be instantiated in the same binary.
#[cfg(all(feature = "in-process-requester", feature = "in-process-responder"))]
compile_error!(
    "features `in-process-requester` and `in-process-responder` are mutually exclusive"
);

// Imports shared by both modes.
use i2c_api::{BusIndex, I2cAddress, I2cTargetClient, TargetMessage};
use i2c_client::IpcI2cClient;
use openprot_mctp_transport_i2c::{I2cSender, MctpI2cReceiver};
use openprot_mctp_api::wire::MAX_PAYLOAD_SIZE;
use pw_status::Result;
use userspace::entry;
use userspace::syscall;

// Each Bazel `rust_app` target generates its own handle-table crate named
// after `codegen_crate_name`.  Pick the right one for this build.
#[cfg(not(feature = "in-process-requester"))]
use app_mctp_server::handle;
#[cfg(feature = "in-process-requester")]
use app_mctp_server_requester::handle;

// Imports shared by every in-process SPDM role (requester or responder).
// Gated on `direct-client` because both role features imply it.
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

// Imports used only by the in-process SPDM requester.
#[cfg(all(feature = "i2c-polling", feature = "in-process-requester"))]
use mock_platform::DemoPeerCertStore;
#[cfg(all(feature = "i2c-polling", feature = "in-process-requester"))]
use spdm_lib::commands::algorithms::request::generate_negotiate_algorithms_request;
#[cfg(all(feature = "i2c-polling", feature = "in-process-requester"))]
use spdm_lib::commands::capabilities::request::generate_capabilities_request_local;
#[cfg(all(feature = "i2c-polling", feature = "in-process-requester"))]
use spdm_lib::commands::version::VersionReqPayload;
#[cfg(all(feature = "i2c-polling", feature = "in-process-requester"))]
use spdm_lib::commands::version::request::generate_get_version;

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

// Address configuration — mirrored between requester and responder builds.
// Requester's OWN values match Responder's REMOTE values, and vice versa.
//
// in-process-requester:  OWN_EID=0x08, OWN_I2C_ADDR=0x10, REMOTE_EID=0x42, REMOTE_I2C_ADDR=0x13
// in-process-responder:  OWN_EID=0x42, OWN_I2C_ADDR=0x13, REMOTE_EID=0x08, REMOTE_I2C_ADDR=0x10

#[cfg(feature = "in-process-requester")]
const OWN_EID: u8 = 0x08;
#[cfg(feature = "in-process-requester")]
const OWN_I2C_ADDR: u8 = 0x10;
#[cfg(feature = "in-process-requester")]
const REMOTE_EID: u8 = 0x42;
#[cfg(feature = "in-process-requester")]
const REMOTE_I2C_ADDR: u8 = 0x13;

#[cfg(feature = "in-process-responder")]
const OWN_EID: u8 = 0x42;
#[cfg(feature = "in-process-responder")]
const OWN_I2C_ADDR: u8 = 0x13;
#[cfg(feature = "in-process-responder")]
const REMOTE_EID: u8 = 0x08;
#[cfg(feature = "in-process-responder")]
const REMOTE_I2C_ADDR: u8 = 0x10;

/// Max number of Phase-2 steps spent in a single `Await*` state before the
/// requester gives up and transitions to `Failed`.  Generous default — see
/// design §6.4 and IMPLEMENTATION_PLAN.md Appendix §A; tune once on-target
/// measurements exist.  10k at the observed idle-poll rate gives a
/// comfortable wall-clock ceiling while still bounding genuine hangs.
#[cfg(feature = "in-process-requester")]
#[allow(dead_code)]
const AWAIT_STEP_BUDGET: u32 = 10_000;

/// Requester FSM state (Phase 2 of the polling loop when
/// `in-process-requester` is enabled).  Stepped once per loop iteration;
/// each `Await*` tick either advances on `Ok` or stays (counted) on `Err`.
#[cfg(feature = "in-process-requester")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
enum ReqState {
    SendVersion = 0,
    AwaitVersion = 1,
    SendCapabilities = 2,
    AwaitCapabilities = 3,
    SendAlgorithms = 4,
    AwaitAlgorithms = 5,
    Done = 6,
    Failed = 7,
}

// ---------------------------------------------------------------------------
// Polling mode — blocks on wait_for_messages(); no WaitGroup or IRQ needed.
// When built with the "in-process-responder" feature, an SPDM responder
// runs in-process using DirectMctpClient (no IPC channel to a separate
// process).  Enable with crate_features = ["i2c-polling",
// "in-process-responder"] in Bazel.
// ---------------------------------------------------------------------------

#[cfg(feature = "i2c-polling")]
fn mctp_loop() -> Result<()> {
    #[cfg(feature = "in-process-requester")]
    pw_log::info!("MCTP server starting (I2C polling mode, REQUESTER, own: eid=0x{:02x} i2c=0x{:02x}, remote: eid=0x{:02x} i2c=0x{:02x})",
        OWN_EID as u32, OWN_I2C_ADDR as u32, REMOTE_EID as u32, REMOTE_I2C_ADDR as u32);

    #[cfg(feature = "in-process-responder")]
    pw_log::info!("MCTP server starting (I2C polling mode, RESPONDER, own: eid=0x{:02x} i2c=0x{:02x}, remote: eid=0x{:02x} i2c=0x{:02x})",
        OWN_EID as u32, OWN_I2C_ADDR as u32, REMOTE_EID as u32, REMOTE_I2C_ADDR as u32);

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

    let sender = I2cSender::new(
        IpcI2cClient::new(handle::I2C),
        BusIndex::BUS_2,
        OWN_I2C_ADDR,
        REMOTE_I2C_ADDR,
    );
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
    #[cfg(feature = "in-process-responder")]
    let mut transport = {
        let client = DirectMctpClient::new(&server);
        MctpSpdmTransport::new_responder(client)
    };

    #[cfg(feature = "in-process-responder")]
    {
        pw_log::info!(
            "MCTP server: registering SPDM responder listener (msg_type=0x05, expecting from eid=0x{:02x}, i2c_addr=0x{:02x})",
            REMOTE_EID as u32, REMOTE_I2C_ADDR as u32
        );
        if transport.init_sequence().is_err() {
            pw_log::error!("MCTP server: SPDM transport init_sequence failed — \
                listener(0x05) rejected; router listener table may be full");
            return Err(pw_status::Error::Internal);
        }
        pw_log::info!("MCTP server: SPDM responder listener registered");
    }

    #[cfg(feature = "in-process-responder")]
    let mut cert_store = MockCertStore::new();
    #[cfg(feature = "in-process-responder")]
    let mut hash = MockHash::new();
    #[cfg(feature = "in-process-responder")]
    let mut m1_hash = MockHash::new();
    #[cfg(feature = "in-process-responder")]
    let mut l1_hash = MockHash::new();
    #[cfg(feature = "in-process-responder")]
    let mut rng = MockRng::new();
    #[cfg(feature = "in-process-responder")]
    let evidence = MockEvidence::new();

    #[cfg(feature = "in-process-responder")]
    let mut flags = CapabilityFlags::default();
    #[cfg(feature = "in-process-responder")]
    {
        flags.set_cert_cap(1);
        flags.set_chal_cap(1);
        flags.set_meas_cap(2);
        flags.set_meas_fresh_cap(1);
        flags.set_chunk_cap(1);
    }

    #[cfg(feature = "in-process-responder")]
    let capabilities = DeviceCapabilities {
        ct_exponent: 0,
        flags,
        data_transfer_size: 1024,
        max_spdm_msg_size: 4096,
        include_supported_algorithms: true,
    };

    #[cfg(feature = "in-process-responder")]
    static SUPPORTED_VERSIONS: [SpdmVersion; 2] = [SpdmVersion::V12, SpdmVersion::V13];

    #[cfg(feature = "in-process-responder")]
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

    #[cfg(feature = "in-process-responder")]
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

    // ---------------------------------------------------------------------------
    // SPDM requester setup (only when in-process-requester feature is enabled).
    // Mirrors the responder block above; role-specific differences:
    //   • transport created with new_requester(client, REMOTE_EID)
    //   • capabilities: meas_cap=0, no meas_fresh_cap, include_supported_algorithms=false
    //   • peer_cert_store: Some(&mut DemoPeerCertStore) (required by VCA+)
    // ---------------------------------------------------------------------------
    #[cfg(feature = "in-process-requester")]
    let mut transport = {
        let client = DirectMctpClient::new(&server);
        MctpSpdmTransport::new_requester(client, REMOTE_EID)
    };

    #[cfg(feature = "in-process-requester")]
    {
        pw_log::info!(
            "MCTP server: initializing SPDM requester transport (target eid=0x{:02x}, i2c_addr=0x{:02x})",
            REMOTE_EID as u32, REMOTE_I2C_ADDR as u32
        );
        if transport.init_sequence().is_err() {
            pw_log::error!("MCTP server: SPDM transport init_sequence failed — \
                req(eid=0x{:02x}) rejected; router request table may be full",
                REMOTE_EID as u32);
            return Err(pw_status::Error::Internal);
        }
        pw_log::info!("MCTP server: SPDM requester transport ready");
    }

    #[cfg(feature = "in-process-requester")]
    let mut cert_store = MockCertStore::new();
    #[cfg(feature = "in-process-requester")]
    let mut hash = MockHash::new();
    #[cfg(feature = "in-process-requester")]
    let mut m1_hash = MockHash::new();
    #[cfg(feature = "in-process-requester")]
    let mut l1_hash = MockHash::new();
    #[cfg(feature = "in-process-requester")]
    let mut rng = MockRng::new();
    #[cfg(feature = "in-process-requester")]
    let evidence = MockEvidence::new();
    #[cfg(feature = "in-process-requester")]
    let mut peer_cert_store = DemoPeerCertStore::default();

    #[cfg(feature = "in-process-requester")]
    let mut flags = CapabilityFlags::default();
    #[cfg(feature = "in-process-requester")]
    {
        flags.set_cert_cap(1);
        flags.set_chal_cap(1);
        flags.set_meas_cap(0);
        flags.set_chunk_cap(1);
    }

    #[cfg(feature = "in-process-requester")]
    let capabilities = DeviceCapabilities {
        ct_exponent: 0,
        flags,
        data_transfer_size: 1024,
        max_spdm_msg_size: 4096,
        // Setting true at V1.3 encodes param1 bit 2 of GET_CAPABILITIES,
        // which the responder currently rejects as an unexpected reserved
        // field.  Algorithm negotiation goes through NEGOTIATE_ALGORITHMS.
        include_supported_algorithms: false,
    };

    #[cfg(feature = "in-process-requester")]
    static SUPPORTED_VERSIONS: [SpdmVersion; 2] = [SpdmVersion::V12, SpdmVersion::V13];

    #[cfg(feature = "in-process-requester")]
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

    #[cfg(feature = "in-process-requester")]
    let mut spdm_ctx = {
        pw_log::info!("MCTP server: creating SPDM requester context (v1.2+v1.3, ECDSA-P384, SHA-384)");
        match SpdmContext::new(
            &SUPPORTED_VERSIONS,
            &mut transport,
            capabilities,
            algorithms,
            &mut cert_store,
            Some(&mut peer_cert_store),
            &mut hash,
            &mut m1_hash,
            &mut l1_hash,
            &mut rng,
            &evidence,
        ) {
            Ok(ctx) => {
                pw_log::info!("MCTP server: SPDM requester context ready");
                ctx
            }
            Err(_) => {
                pw_log::error!("MCTP server: SpdmContext::new failed (requester) — \
                    check cert_store, hash, rng platform impls");
                return Err(pw_status::Error::Internal);
            }
        }
    };

    // Shared between responder and requester — both need a MessageBuf for
    // SPDM encode/decode.  Gated on `direct-client` (implied by either role).
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
    #[cfg(feature = "in-process-responder")]
    let mut spdm_ok: u32 = 0;
    #[cfg(feature = "in-process-responder")]
    let mut spdm_err: u32 = 0;
    let mut i2c_pkt: u32 = 0;
    let mut idle_polls: u32 = 0;

    // Requester FSM: starts at SendVersion, walks the VCA flow, ends at
    // Done or Failed.  Stepped once per loop iteration in Phase 2 below.
    #[cfg(feature = "in-process-requester")]
    let mut req_state = ReqState::SendVersion;
    // Observability counters — see design §8.  All u32 wrapping; log volume
    // is bounded by rate-limiting inside the FSM step.
    #[cfg(feature = "in-process-requester")]
    let mut req_send_ok: u32 = 0;
    #[cfg(feature = "in-process-requester")]
    let mut req_recv_ok: u32 = 0;
    #[cfg(feature = "in-process-requester")]
    let mut req_recv_pending: u32 = 0;
    // Number of consecutive steps spent in the current Await state.  Reset
    // to zero on every state transition; exhaustion of AWAIT_STEP_BUDGET
    // transitions the FSM to Failed so the device does not hang.
    #[cfg(feature = "in-process-requester")]
    let mut await_steps: u32 = 0;

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
                        Ok((pkt, hdr)) => {
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
                                hdr.source as u32,
                                hdr.dest as u32,
                                hdr.byte_count as u32,
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
        #[cfg(feature = "in-process-responder")]
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

        // Phase 2 (requester): one FSM step per loop iteration.  Send
        // states emit a request and advance; Await states try to consume
        // a response and either advance on Ok or stay on Err (which is
        // the normal "router has not assembled the response yet" case
        // given DirectMctpClient::recv returns TimedOut immediately).
        // Done/Failed are terminal no-ops; Phase 1 keeps draining I2C.
        #[cfg(feature = "in-process-requester")]
        {
            // Design follow-ups (see DESIGN_IN_PROCESS_REQUESTER.md):
            //   §6.3 option 2: plumb TransportError through
            //     `requester_process_message` so genuine protocol errors
            //     can be distinguished from TimedOut; today the Err arm
            //     relies on AWAIT_STEP_BUDGET alone.
            //   §9.1: pumping-`DirectMctpClient` (Shape B) would let the
            //     FSM call a synchronous API and delete the state machine.
            //   §9.3: external trigger — today the requester fires VCA
            //     at boot; a real caller will want IPC/GPIO/policy control.
            //
            // Await* states share a retry pattern: try to process one
            // message; on Ok advance, on Err count it as pending and
            // check the step budget.  `await_try!` centralizes that
            // logic and emits the state-timeout log if the budget
            // runs out.
            macro_rules! await_try {
                ($next:expr, $state_code:expr) => {{
                    msg_buf.reset();
                    match spdm_ctx.requester_process_message(&mut msg_buf) {
                        Ok(_) => {
                            req_recv_ok = req_recv_ok.wrapping_add(1);
                            await_steps = 0;
                            req_state = $next;
                        }
                        Err(_) => {
                            req_recv_pending = req_recv_pending.wrapping_add(1);
                            await_steps = await_steps.wrapping_add(1);
                            if await_steps >= AWAIT_STEP_BUDGET {
                                pw_log::error!(
                                    "SPDM requester: AWAIT_STEP_BUDGET exhausted in \
                                    state={} (steps={} pending_total={}) — giving up",
                                    $state_code as u32,
                                    await_steps as u32,
                                    req_recv_pending as u32,
                                );
                                req_state = ReqState::Failed;
                            }
                        }
                    }
                }};
            }

            match req_state {
                ReqState::SendVersion => {
                    msg_buf.reset();
                    let ok = generate_get_version(
                        &mut spdm_ctx,
                        &mut msg_buf,
                        VersionReqPayload::new(0, 0),
                    )
                    .is_ok()
                        && spdm_ctx
                            .requester_send_request(&mut msg_buf, REMOTE_EID)
                            .is_ok();
                    if ok {
                        req_send_ok = req_send_ok.wrapping_add(1);
                        await_steps = 0;
                        pw_log::info!("SPDM requester: sent GET_VERSION, awaiting VERSION");
                        req_state = ReqState::AwaitVersion;
                    } else {
                        pw_log::error!("SPDM requester: GET_VERSION send failed");
                        req_state = ReqState::Failed;
                    }
                }
                ReqState::AwaitVersion => {
                    await_try!(ReqState::SendCapabilities, ReqState::AwaitVersion);
                    if req_state == ReqState::SendCapabilities {
                        pw_log::info!("SPDM requester: VERSION ok, sending GET_CAPABILITIES");
                    }
                }
                ReqState::SendCapabilities => {
                    msg_buf.reset();
                    let ok = generate_capabilities_request_local(&mut spdm_ctx, &mut msg_buf)
                        .is_ok()
                        && spdm_ctx
                            .requester_send_request(&mut msg_buf, REMOTE_EID)
                            .is_ok();
                    if ok {
                        req_send_ok = req_send_ok.wrapping_add(1);
                        await_steps = 0;
                        pw_log::info!("SPDM requester: sent GET_CAPABILITIES, awaiting CAPABILITIES");
                        req_state = ReqState::AwaitCapabilities;
                    } else {
                        pw_log::error!("SPDM requester: GET_CAPABILITIES send failed");
                        req_state = ReqState::Failed;
                    }
                }
                ReqState::AwaitCapabilities => {
                    await_try!(ReqState::SendAlgorithms, ReqState::AwaitCapabilities);
                    if req_state == ReqState::SendAlgorithms {
                        pw_log::info!(
                            "SPDM requester: CAPABILITIES ok, sending NEGOTIATE_ALGORITHMS"
                        );
                    }
                }
                ReqState::SendAlgorithms => {
                    msg_buf.reset();
                    let ok = generate_negotiate_algorithms_request(
                        &mut spdm_ctx,
                        &mut msg_buf,
                        None,
                        None,
                        None,
                        None,
                    )
                    .is_ok()
                        && spdm_ctx
                            .requester_send_request(&mut msg_buf, REMOTE_EID)
                            .is_ok();
                    if ok {
                        req_send_ok = req_send_ok.wrapping_add(1);
                        await_steps = 0;
                        pw_log::info!(
                            "SPDM requester: sent NEGOTIATE_ALGORITHMS, awaiting ALGORITHMS"
                        );
                        req_state = ReqState::AwaitAlgorithms;
                    } else {
                        pw_log::error!("SPDM requester: NEGOTIATE_ALGORITHMS send failed");
                        req_state = ReqState::Failed;
                    }
                }
                ReqState::AwaitAlgorithms => {
                    await_try!(ReqState::Done, ReqState::AwaitAlgorithms);
                    if req_state == ReqState::Done {
                        pw_log::info!(
                            "SPDM VCA completed: version/caps/algs OK \
                            (send_ok={} recv_ok={} recv_pending={} idle_polls={})",
                            req_send_ok as u32,
                            req_recv_ok as u32,
                            req_recv_pending as u32,
                            idle_polls as u32,
                        );
                    }
                }
                ReqState::Done | ReqState::Failed => {
                    // Terminal states — Phase 1 continues draining I2C,
                    // Phase 2 is a no-op.
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
    let sender = I2cSender::new(
        IpcI2cClient::new(handle::I2C),
        BusIndex::BUS_2,
        OWN_I2C_ADDR,
        REMOTE_I2C_ADDR,
    );
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

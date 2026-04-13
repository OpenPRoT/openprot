// Licensed under the Apache-2.0 license

//! MCTP Server — IPC Dispatch Loop
//!
//! Userspace service that receives MCTP requests over a Pigweed IPC channel,
//! dispatches them to the MCTP server core, and responds with results.
//!
//! # Architecture
//!
//! ```text
//! ┌─ Client ──────────────────────────┐
//! │ channel_transact(request)         │
//! └──────────────┬────────────────────┘
//!                │ IPC channel
//!                ▼
//! ┌─ This Server ─────────────────────┐
//! │ object_wait(READABLE)             │
//! │ channel_read → MctpRequestHeader  │
//! │ dispatch_mctp_op(op, server)      │
//! │ channel_respond ← MctpRespHeader  │
//! └──────────────┬────────────────────┘
//!                │ mctp-stack Router
//!                ▼
//! ┌─ I2C Transport ──────────────────┐
//! │ I2cSender → I2C Server IPC      │
//! └──────────────────────────────────┘
//! ```
//!
//! # IPC Pattern
//!
//! Follows the same loop as `services/i2c/server/src/main.rs`:
//!
//! 1. `object_wait(handle, READABLE)` — block until a client sends a request
//! 2. `channel_read(handle)` — read the raw request bytes
//! 3. Parse `MctpRequestHeader`, dispatch via `dispatch_mctp_op`
//! 4. `channel_respond(handle)` — send response header + data
//!
//! # Handle Binding
//!
//! The IPC handle is provided by the `app_package` Bazel rule, which generates
//! `app_mctp_server::handle::MCTP` from the system configuration.

#![no_main]
#![no_std]

use openprot_mctp_api::wire::{self, MctpRequestHeader, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE, MAX_PAYLOAD_SIZE};
use openprot_mctp_api::ResponseCode;
use openprot_mctp_server::{dispatch, RecvResult};

use i2c_api::{BusIndex, I2cTargetClient, TargetMessage};
use i2c_client::IpcI2cClient;
use openprot_mctp_transport_i2c::{I2cSender, MctpI2cReceiver};

use pw_status::Result;
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::Instant;

use app_mctp_server::handle;

const OWN_EID: u8 = 8;
const OWN_I2C_ADDR: u8 = 0x10;

// ---------------------------------------------------------------------------
// Server loop
// ---------------------------------------------------------------------------

fn mctp_server_loop() -> Result<()> {
    pw_log::info!("MCTP server starting");

    // I2C notification client: receives slave-mode interrupts via Signals::USER.
    let mut i2c_notify = IpcI2cClient::new(handle::I2C);
    i2c_notify
        .register_notification(BusIndex::BUS_0, 0)
        .map_err(|_| pw_status::Error::Internal)?;

    // Separate handle for the sender — I2cSender takes ownership.
    let sender = I2cSender::new(IpcI2cClient::new(handle::I2C), BusIndex::BUS_0, OWN_I2C_ADDR);
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

    loop {
        let ev = syscall::object_wait(handle::WG, Signals::READABLE, Instant::MAX)?;

        if ev.user_data == 1 {
            // Inbound I2C data: drain pending messages, decode I2C framing,
            // feed raw MCTP packets into the router.
            let mut msgs = [TargetMessage::default(); 1];
            if let Ok(n) = i2c_notify.get_pending_messages(BusIndex::BUS_0, &mut msgs) {
                for msg in &msgs[..n] {
                    if let Ok((pkt, _src_addr)) = receiver.decode(msg) {
                        let _ = server.inbound(pkt);
                    }
                }
            }
            // After routing inbound packets, fulfil any clients parked on recv.
            let (_, ready) = server.update(0, &mut recv_buf);
            for (_, result) in ready {
                match result {
                    RecvResult::Message(meta) => {
                        let payload = &recv_buf[..meta.payload_size];
                        if let Ok(len) = wire::encode_recv_response(
                            &mut response_buf,
                            meta.msg_type,
                            meta.msg_ic,
                            meta.remote_eid,
                            meta.msg_tag,
                            payload,
                        ) {
                            let _ = syscall::channel_respond(
                                handle::MCTP,
                                &response_buf[..len],
                            );
                        }
                    }
                    RecvResult::TimedOut => {
                        let resp = openprot_mctp_api::wire::MctpResponseHeader::error(
                            ResponseCode::TimedOut,
                        );
                        response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE]
                            .copy_from_slice(&resp.to_bytes());
                        let _ = syscall::channel_respond(
                            handle::MCTP,
                            &response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE],
                        );
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

            // Dispatch; for deferred Recv the response is sent later from
            // the I2C inbound path via server.update().
            if let Some(response_len) = dispatch::dispatch_mctp_op(
                &request_buf[..len],
                &mut response_buf,
                &mut server,
                &mut recv_buf,
            ) {
                syscall::channel_respond(handle::MCTP, &response_buf[..response_len])?;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[entry]
fn entry() -> ! {
    if let Err(e) = mctp_server_loop() {
        pw_log::error!("MCTP server error: {}", e as u32);
        let _ = syscall::debug_shutdown(Err(e));
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

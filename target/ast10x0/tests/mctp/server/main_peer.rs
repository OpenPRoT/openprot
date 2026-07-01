// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! MCTP Server — IPC Dispatch Loop (peer role)
//!
//! Same service behavior as `main.rs`, but with complementary I2C addressing
//! for the second board in dual-EVB tests.

#![no_main]
#![no_std]

use i2c_api::SlaveEvent;
use i2c_client::I2cClient;
use i2c_client_ipc::IpcTransport;
use openprot_mctp_api::wire::{
    self, MctpOp, MctpRequestHeader, MAX_PAYLOAD_SIZE, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE,
};
use openprot_mctp_api::{Handle, ResponseCode};
use openprot_mctp_server::dispatch::{self, DispatchOutcome};
use openprot_mctp_transport_i2c::{I2cSender, MctpI2cReceiver};

use pw_status::Error;
use pw_status::Result;
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::{Clock, Duration, Instant, SystemClock};

use app_mctp_server_peer::handle;

const OWN_EID: u8 = 9;
const OWN_I2C_ADDR: u8 = 0x42;
const REMOTE_I2C_ADDR: u8 = 0x10;
const I2C_RX_MAX: usize = MAX_PAYLOAD_SIZE;

fn mctp_server_loop() -> Result<()> {
    pw_log::info!("MCTP server peer starting");
    let sender = I2cSender::new(
        I2cClient::new(IpcTransport::new(handle::I2C)),
        OWN_I2C_ADDR,
        REMOTE_I2C_ADDR,
    );
    let mut i2c_rx_client = I2cClient::new(IpcTransport::new(handle::I2C));
    let i2c_receiver = MctpI2cReceiver::new(OWN_I2C_ADDR);

    if i2c_rx_client.configure_slave(OWN_I2C_ADDR).is_err() {
        pw_log::error!("configure_slave failed");
        return Err(Error::Internal);
    }
    if i2c_rx_client.enable_slave().is_err() {
        pw_log::error!("enable_slave failed");
        return Err(Error::Internal);
    }
    if i2c_rx_client.enable_notification().is_err() {
        pw_log::error!("enable_notification failed");
        return Err(Error::Internal);
    }

    let mut server = openprot_mctp_server::Server::<_, 16>::new(mctp::Eid(OWN_EID), 0, sender);

    let mut request_buf = [0u8; MAX_REQUEST_SIZE];
    let mut response_buf = [0u8; MAX_RESPONSE_SIZE];
    let mut recv_buf = [0u8; MAX_PAYLOAD_SIZE];
    let mut i2c_rx_buf = [0u8; I2C_RX_MAX];

    syscall::wait_group_add(handle::WG, handle::MCTP, Signals::READABLE, 0usize)?;
    syscall::wait_group_add(handle::WG, handle::I2C, Signals::USER, 1usize)?;

    struct PendingRecv {
        handle: Handle,
        deadline: Instant,
    }

    let mut pending_recv: Option<PendingRecv> = None;

    loop {
        let wait_deadline = pending_recv
            .as_ref()
            .map(|pending| pending.deadline)
            .unwrap_or(Instant::MAX);
        let ev = match syscall::object_wait(
            handle::WG,
            Signals::READABLE | Signals::USER,
            wait_deadline,
        ) {
            Ok(ev) => ev,
            Err(pw_status::Error::DeadlineExceeded) => {
                if pending_recv.take().is_some() {
                    let resp =
                        openprot_mctp_api::wire::MctpResponseHeader::error(ResponseCode::TimedOut);
                    response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE]
                        .copy_from_slice(&resp.to_bytes());
                    let _ = syscall::channel_respond(
                        handle::MCTP,
                        &response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE],
                    );
                    let _ = syscall::wait_group_add(
                        handle::WG,
                        handle::MCTP,
                        Signals::READABLE,
                        0usize,
                    );
                }
                continue;
            }
            Err(err) => return Err(err),
        };

        if ev.user_data == 1 {
            match i2c_rx_client.slave_receive(&mut i2c_rx_buf) {
                Ok(event) => {
                    if event.kind == SlaveEvent::DataReceived && event.data_len > 0 {
                        if let Ok((pkt, _)) = i2c_receiver.decode(&i2c_rx_buf[..event.data_len]) {
                            let _ = server.inbound(pkt);
                        } else {
                            pw_log::error!("i2c frame decode failed");
                        }
                    }
                }
                Err(_) => {
                    pw_log::error!("slave_receive failed");
                }
            }
            if let Some(pending) = pending_recv.as_ref() {
                if let Some(meta) = server.try_recv(pending.handle, &mut recv_buf) {
                    let payload = &recv_buf[..meta.payload_size];
                    let response_len = openprot_mctp_api::wire::encode_recv_response(
                        &mut response_buf,
                        meta.msg_type,
                        meta.msg_ic,
                        meta.remote_eid,
                        meta.msg_tag,
                        payload,
                    )
                    .unwrap_or_else(|_| {
                        openprot_mctp_api::wire::encode_error_response(
                            &mut response_buf,
                            ResponseCode::InternalError,
                        )
                        .unwrap_or(0)
                    });
                    syscall::channel_respond(handle::MCTP, &response_buf[..response_len])?;
                    pending_recv = None;
                    syscall::wait_group_add(handle::WG, handle::MCTP, Signals::READABLE, 0usize)?;
                }
            }
        } else {
            let len = syscall::channel_read(handle::MCTP, 0, &mut request_buf)?;
            if pending_recv.is_some() {
                let resp =
                    openprot_mctp_api::wire::MctpResponseHeader::error(ResponseCode::InternalError);
                response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE]
                    .copy_from_slice(&resp.to_bytes());
                syscall::channel_respond(
                    handle::MCTP,
                    &response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE],
                )?;
                continue;
            }

            if len < MctpRequestHeader::SIZE {
                let resp =
                    openprot_mctp_api::wire::MctpResponseHeader::error(ResponseCode::BadArgument);
                response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE]
                    .copy_from_slice(&resp.to_bytes());
                syscall::channel_respond(
                    handle::MCTP,
                    &response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE],
                )?;
                continue;
            }

            if MctpRequestHeader::from_bytes(&request_buf[..len])
                .and_then(|h| h.operation())
                .map_or(false, |op| matches!(op, MctpOp::Recv))
            {
                let header = MctpRequestHeader::from_bytes(&request_buf[..len]).unwrap();
                let recv_handle = Handle(header.handle);
                let payload = wire::get_request_payload(&request_buf[..len]);
                if payload.len() < 4 {
                    let resp = openprot_mctp_api::wire::MctpResponseHeader::error(
                        ResponseCode::BadArgument,
                    );
                    response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE]
                        .copy_from_slice(&resp.to_bytes());
                    syscall::channel_respond(
                        handle::MCTP,
                        &response_buf[..openprot_mctp_api::wire::MctpResponseHeader::SIZE],
                    )?;
                    continue;
                }

                let timeout_millis = u32::from_le_bytes(payload[..4].try_into().unwrap());
                match server.try_recv(recv_handle, &mut recv_buf) {
                    Some(meta) => {
                        let payload = &recv_buf[..meta.payload_size];
                        let response_len = openprot_mctp_api::wire::encode_recv_response(
                            &mut response_buf,
                            meta.msg_type,
                            meta.msg_ic,
                            meta.remote_eid,
                            meta.msg_tag,
                            payload,
                        )
                        .unwrap_or_else(|_| {
                            openprot_mctp_api::wire::encode_error_response(
                                &mut response_buf,
                                ResponseCode::InternalError,
                            )
                            .unwrap_or(0)
                        });
                        syscall::channel_respond(handle::MCTP, &response_buf[..response_len])?;
                    }
                    None => {
                        let deadline = if timeout_millis == 0 {
                            Instant::MAX
                        } else {
                            SystemClock::now()
                                .checked_add_duration(Duration::from_millis(timeout_millis as i64))
                                .unwrap_or(Instant::MAX)
                        };
                        pending_recv = Some(PendingRecv {
                            handle: recv_handle,
                            deadline,
                        });
                        let _ = syscall::wait_group_remove(handle::WG, handle::MCTP);
                    }
                }
            } else {
                let response_len = match dispatch::dispatch_mctp_op(
                    &request_buf[..len],
                    &mut response_buf,
                    &mut server,
                    &mut recv_buf,
                    0,
                ) {
                    DispatchOutcome::Reply(n) => n,
                    DispatchOutcome::Pending { .. } => unreachable!("Recv handled above"),
                };
                syscall::channel_respond(handle::MCTP, &response_buf[..response_len])?;
            }
        }
    }
}

#[entry]
fn entry() {
    if let Err(e) = mctp_server_loop() {
        pw_log::error!("mctp_server peer exiting with error");
        let _ = syscall::process_exit(e as u32);
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

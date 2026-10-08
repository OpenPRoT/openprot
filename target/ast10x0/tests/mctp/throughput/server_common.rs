// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

// Shared body of the combined I2C + MCTP server, `include!`d by `server_main.rs`
// and `server_main_peer.rs`; each defines `handle`, `signals`, the EIDs and the
// I2C addresses before including it.
//
// The I2C driver for Bus 2 lives in this process, so the MCTP stack drives it
// directly (no I2C IPC hop): outbound packets go through `SharedBus`, inbound
// packets are drained from the slave RX buffer on the I2C IRQ.

use core::cell::RefCell;

use ast10x0_peripherals::create_pins;
use ast10x0_peripherals::i2c::{ClockConfig, I2cConfig, I2cSpeed, I2cXferMode};
use embedded_hal::i2c::{ErrorType, I2c, Operation, SevenBitAddress};
use i2c_api::seam::{I2cIsrEvent, I2cSlaveBuffer, I2cSlaveCore, I2cSlaveEvent};
use openprot_mctp_api::wire::{
    self, MctpOp, MctpRequestHeader, MctpResponseHeader, MAX_PAYLOAD_SIZE, MAX_REQUEST_SIZE,
    MAX_RESPONSE_SIZE,
};
use openprot_mctp_api::{Handle, ResponseCode};
use openprot_mctp_server::dispatch::{self, DispatchOutcome};
use openprot_mctp_transport_i2c::{I2cSender, MctpI2cReceiver};
use pw_status::{Error, Result};
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::{Clock, Duration, Instant, SystemClock};

const SLAVE_CFG: I2cConfig = I2cConfig {
    speed: I2cSpeed::Standard,
    xfer_mode: I2cXferMode::DmaMode,
    multi_master: false,
    smbus_timeout: false,
    smbus_alert: false,
    clock_config: ClockConfig::ast1060_default(),
};

/// Lets the MCTP sender (master writes) and the IRQ handler (slave reads)
/// share the one bus driver. Single-threaded, so `RefCell` never contends.
struct SharedBus<'a, B>(&'a RefCell<B>);

impl<B: ErrorType> ErrorType for SharedBus<'_, B> {
    type Error = B::Error;
}

impl<B: I2c<SevenBitAddress>> I2c<SevenBitAddress> for SharedBus<'_, B> {
    fn transaction(
        &mut self,
        address: SevenBitAddress,
        operations: &mut [Operation<'_>],
    ) -> core::result::Result<(), Self::Error> {
        self.0.borrow_mut().transaction(address, operations)
    }
}

fn respond_error(code: ResponseCode, response_buf: &mut [u8]) -> Result<()> {
    response_buf[..MctpResponseHeader::SIZE]
        .copy_from_slice(&MctpResponseHeader::error(code).to_bytes());
    syscall::channel_respond(handle::MCTP, &response_buf[..MctpResponseHeader::SIZE])
}

fn respond_recv(
    meta: &openprot_mctp_api::RecvMetadata,
    recv_buf: &[u8],
    response_buf: &mut [u8],
) -> Result<()> {
    let len = wire::encode_recv_response(
        response_buf,
        meta.msg_type,
        meta.msg_ic,
        meta.remote_eid,
        meta.msg_tag,
        &recv_buf[..meta.payload_size],
    )
    .unwrap_or_else(|_| {
        wire::encode_error_response(response_buf, ResponseCode::InternalError).unwrap_or(0)
    });
    syscall::channel_respond(handle::MCTP, &response_buf[..len])
}

fn server_loop() -> Result<()> {
    pw_log::info!("MCTP+I2C server starting");

    // The kernel routed Bus 2's pins at the SCU before starting us; userspace has no SCU grant, so
    // we bind the already-routed pins rather than re-muxing them.
    // SAFETY: sole pin creation site in this binary, at boot; the pins! table is this chip's true pin map.
    let pins = unsafe { create_pins() };
    let (scl, sda) = (pins.scu418_0, pins.scu418_1);
    let (Some(master_dma_buf), Some(slave_dma_buf)) = (
        i2c_backend::non_cached_buf!(4096),
        i2c_backend::non_cached_buf!(512),
    ) else {
        pw_log::error!("i2c DMA buffers already taken");
        return Err(Error::Internal);
    };
    let Ok(mut driver) =
        i2c_backend::open_bus_dma(scl, sda, &SLAVE_CFG, master_dma_buf, slave_dma_buf)
    else {
        pw_log::error!("i2c bus open failed");
        return Err(Error::Internal);
    };
    if driver.configure_slave_address(OWN_I2C_ADDR).is_err() || driver.enable_slave_mode().is_err()
    {
        pw_log::error!("i2c slave setup failed");
        return Err(Error::Internal);
    }
    let bus = RefCell::new(driver);

    let sender = I2cSender::new(SharedBus(&bus), OWN_I2C_ADDR, REMOTE_I2C_ADDR);
    let i2c_receiver = MctpI2cReceiver::new(OWN_I2C_ADDR);
    let mut server = openprot_mctp_server::Server::<_, 16>::new(mctp::Eid(OWN_EID), 0, sender);

    let mut request_buf = [0u8; MAX_REQUEST_SIZE];
    let mut response_buf = [0u8; MAX_RESPONSE_SIZE];
    let mut recv_buf = [0u8; MAX_PAYLOAD_SIZE];
    let mut i2c_rx_buf = [0u8; MAX_PAYLOAD_SIZE];

    // A blocking recv whose IPC reply is deferred until a packet arrives or
    // its deadline passes.
    struct PendingRecv {
        handle: Handle,
        deadline: Instant,
    }
    let mut pending_recv: Option<PendingRecv> = None;

    // user_data=0 → MCTP client channel READABLE, user_data=1 → I2C IRQ.
    syscall::wait_group_add(handle::WG, handle::MCTP, Signals::READABLE, 0usize)?;
    syscall::wait_group_add(handle::WG, handle::I2C2_IRQ, signals::I2C2, 1usize)?;

    loop {
        let wait_deadline = pending_recv
            .as_ref()
            .map_or(Instant::MAX, |pending| pending.deadline);
        let ev = match syscall::object_wait(
            handle::WG,
            Signals::READABLE | signals::I2C2,
            wait_deadline,
        ) {
            Ok(ev) => ev,
            Err(Error::DeadlineExceeded) => {
                if pending_recv.take().is_some() {
                    let _ = respond_error(ResponseCode::TimedOut, &mut response_buf);
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
            // Slave IRQ: drain a received packet, ack, feed the router.
            let event = bus.borrow_mut().try_next_slave_event();
            if let Ok(Some((I2cIsrEvent::SlaveWrRecvd, _))) = event {
                match bus.borrow_mut().read_slave_buffer(&mut i2c_rx_buf) {
                    Ok(n) if n > 0 => match i2c_receiver.decode(&i2c_rx_buf[..n]) {
                        Ok((pkt, _)) => {
                            let _ = server.inbound(pkt);
                        }
                        Err(_) => pw_log::error!("i2c frame decode failed"),
                    },
                    Ok(_) => {}
                    Err(_) => pw_log::error!("read_slave_buffer failed"),
                }
            }
            if syscall::interrupt_ack(handle::I2C2_IRQ, ev.pending_signals & signals::I2C2)
                .is_err()
            {
                pw_log::error!("interrupt_ack failed");
            }
            // Satisfy any deferred blocking recv now that a packet was processed.
            if let Some(pending) = pending_recv.as_ref() {
                if let Some(meta) = server.try_recv(pending.handle, &mut recv_buf) {
                    respond_recv(&meta, &recv_buf, &mut response_buf)?;
                    pending_recv = None;
                    syscall::wait_group_add(handle::WG, handle::MCTP, Signals::READABLE, 0usize)?;
                }
            }
            continue;
        }

        // IPC from a client — non-blocking, the WaitGroup fired on READABLE.
        let len = syscall::channel_read(handle::MCTP, 0, &mut request_buf)?;
        if pending_recv.is_some() {
            // A blocking recv is in flight; reject so READABLE clears.
            respond_error(ResponseCode::InternalError, &mut response_buf)?;
            continue;
        }
        if len < MctpRequestHeader::SIZE {
            respond_error(ResponseCode::BadArgument, &mut response_buf)?;
            continue;
        }

        let header = MctpRequestHeader::from_bytes(&request_buf[..len]);
        if header
            .as_ref()
            .and_then(|h| h.operation())
            .is_some_and(|op| matches!(op, MctpOp::Recv))
        {
            let recv_handle = Handle(header.map_or(0, |h| h.handle));
            let payload = wire::get_request_payload(&request_buf[..len]);
            if payload.len() < 4 {
                respond_error(ResponseCode::BadArgument, &mut response_buf)?;
                continue;
            }
            let timeout_millis = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
            match server.try_recv(recv_handle, &mut recv_buf) {
                Some(meta) => respond_recv(&meta, &recv_buf, &mut response_buf)?,
                None => {
                    let deadline = if timeout_millis == 0 {
                        Instant::MAX
                    } else {
                        SystemClock::now()
                            .checked_add_duration(Duration::from_millis(timeout_millis as u64))
                            .unwrap_or(Instant::MAX)
                    };
                    pending_recv = Some(PendingRecv {
                        handle: recv_handle,
                        deadline,
                    });
                    // Remove MCTP from the WaitGroup so READABLE on the open
                    // transaction can't re-fire it.
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

#[entry]
fn entry() {
    if let Err(e) = server_loop() {
        pw_log::error!("mctp_i2c_server exiting with error");
        let _ = syscall::process_exit(e as u32);
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

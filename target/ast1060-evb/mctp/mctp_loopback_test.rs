// Licensed under the Apache-2.0 license

//! MCTP Loopback Test Application
//!
//! This application demonstrates MCTP communication using loopback transport.
//! It creates two MCTP servers (A and B) within the same process, connected
//! via loopback transport, and verifies bidirectional message exchange.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │ mctp_loopback_test (single process)         │
//! │                                              │
//! │  ┌─ Server A (EID 8) ──┐                    │
//! │  │  Router              │                    │
//! │  │  Listener(type=1)    │                    │
//! │  └──────┬───────────────┘                    │
//! │         │                                     │
//! │         │ LoopbackSender(A→B)                │
//! │         ▼                                     │
//! │  [ LoopbackPair ]                            │
//! │         │                                     │
//! │         │ LoopbackSender(B→A)                │
//! │         ▼                                     │
//! │  ┌─ Server B (EID 42) ─┐                    │
//! │  │  Router              │                    │
//! │  │  Request handle      │                    │
//! │  └──────────────────────┘                    │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! # Test Sequence
//!
//! 1. Server A registers a listener for message type 1
//! 2. Server B sends a request to EID 8 with type 1
//! 3. Transfer packets B→A through loopback pair
//! 4. Server A receives the message on the listener
//! 5. Server A echoes the message back
//! 6. Transfer packets A→B through loopback pair
//! 7. Server B receives the echo response
//! 8. Verify payload matches

#![no_main]
#![no_std]

use core::cell::RefCell;

use mctp::Eid;
use openprot_mctp_api::{Handle, MctpClient, MctpError, RecvMetadata, ResponseCode};
use openprot_mctp_server::Server;
use openprot_mctp_transport_loopback::{LoopbackPair, LoopbackSender};

use pw_status::Result;
use userspace::entry;
use userspace::syscall;

// ---------------------------------------------------------------------------
// DirectClient wrapper - provides MctpClient trait for in-process server
// ---------------------------------------------------------------------------

struct DirectClient<'a, S: mctp_lib::Sender, const N: usize> {
    server: &'a RefCell<Server<S, N>>,
}

impl<'a, S: mctp_lib::Sender, const N: usize> DirectClient<'a, S, N> {
    fn new(server: &'a RefCell<Server<S, N>>) -> Self {
        Self { server }
    }
}

impl<S: mctp_lib::Sender, const N: usize> MctpClient for DirectClient<'_, S, N> {
    fn req(&self, eid: u8) -> core::result::Result<Handle, MctpError> {
        self.server.borrow_mut().req(eid)
    }

    fn listener(&self, msg_type: u8) -> core::result::Result<Handle, MctpError> {
        self.server.borrow_mut().listener(msg_type)
    }

    fn get_eid(&self) -> u8 {
        self.server.borrow().get_eid()
    }

    fn set_eid(&self, eid: u8) -> core::result::Result<(), MctpError> {
        self.server.borrow_mut().set_eid(eid)
    }

    fn recv(
        &self,
        handle: Handle,
        _timeout_millis: u32,
        buf: &mut [u8],
    ) -> core::result::Result<RecvMetadata, MctpError> {
        self.server
            .borrow_mut()
            .try_recv(handle, buf)
            .ok_or(MctpError::from_code(ResponseCode::TimedOut))
    }

    fn send(
        &self,
        handle: Option<Handle>,
        msg_type: u8,
        eid: Option<u8>,
        tag: Option<u8>,
        integrity_check: bool,
        buf: &[u8],
    ) -> core::result::Result<u8, MctpError> {
        self.server
            .borrow_mut()
            .send(handle, msg_type, eid, tag, integrity_check, buf)
    }

    fn drop_handle(&self, handle: Handle) {
        let _ = self.server.borrow_mut().unbind(handle);
    }
}

// ---------------------------------------------------------------------------
// Test logic
// ---------------------------------------------------------------------------

fn echo_once(client: &impl MctpClient, listener_handle: Handle) -> core::result::Result<(), MctpError> {
    let mut recv_buf = [0u8; 255];
    let meta = client.recv(listener_handle, 0, &mut recv_buf)?;

    pw_log::info!(
        "Echo: received {} bytes from EID {}",
        meta.payload_size as u32,
        meta.remote_eid as u32
    );

    let payload = &recv_buf[..meta.payload_size];
    client.send(
        None,
        meta.msg_type,
        Some(meta.remote_eid),
        Some(meta.msg_tag),
        false,
        payload,
    )?;

    Ok(())
}

fn run_loopback_test() -> Result<()> {
    pw_log::info!("MCTP Loopback Test starting");

    // Create loopback pair
    let pair = LoopbackPair::new();
    let sender_a = LoopbackSender::new(&pair.a_to_b);
    let sender_b = LoopbackSender::new(&pair.b_to_a);

    // Create two MCTP servers
    let server_a: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(8), 0, sender_a));
    let server_b: RefCell<Server<_, 16>> = RefCell::new(Server::new(Eid(42), 0, sender_b));

    let client_a = DirectClient::new(&server_a);
    let client_b = DirectClient::new(&server_b);

    pw_log::info!("Created Server A (EID 8) and Server B (EID 42)");

    // A registers listener for type 1 (echo responder)
    let listener_a = client_a
        .listener(1)
        .map_err(|_| pw_status::Error::Internal)?;
    pw_log::info!("Server A: registered listener for type 1");

    // B gets request handle to send to EID 8
    let req_b = client_b
        .req(8)
        .map_err(|_| pw_status::Error::Internal)?;
    pw_log::info!("Server B: got request handle for EID 8");

    // Run multiple echo roundtrips
    for iteration in 0..5u8 {
        let msg = [iteration; 32];

        pw_log::info!("=== Iteration {} ===", iteration as u32);

        // B sends request
        pw_log::info!("Server B: sending request");
        client_b
            .send(Some(req_b), 1, None, None, false, &msg)
            .map_err(|_| pw_status::Error::Internal)?;

        // Transfer B→A
        let mut count = 0;
        while let Some(pkt) = pair.pop_b_to_a() {
            server_a.borrow_mut().inbound(&pkt).map_err(|_| pw_status::Error::Internal)?;
            count += 1;
        }
        pw_log::info!("Transferred {} packets B→A", count as u32);

        // A echoes
        pw_log::info!("Server A: echoing message");
        echo_once(&client_a, listener_a).map_err(|_| pw_status::Error::Internal)?;

        // Transfer A→B
        count = 0;
        while let Some(pkt) = pair.pop_a_to_b() {
            server_b.borrow_mut().inbound(&pkt).map_err(|_| pw_status::Error::Internal)?;
            count += 1;
        }
        pw_log::info!("Transferred {} packets A→B", count as u32);

        // B receives echo response
        let mut resp_buf = [0u8; 255];
        let resp = client_b
            .recv(req_b, 0, &mut resp_buf)
            .map_err(|_| pw_status::Error::Internal)?;

        pw_log::info!(
            "Server B: received response ({} bytes)",
            resp.payload_size as u32
        );

        // Verify payload
        let response = &resp_buf[..resp.payload_size];
        if response != &msg[..] {
            pw_log::error!("Payload mismatch!");
            return Err(pw_status::Error::DataLoss);
        }

        pw_log::info!("✓ Iteration {} passed", iteration as u32);
    }

    // Cleanup
    client_a.drop_handle(listener_a);
    client_b.drop_handle(req_b);

    pw_log::info!("=== All tests PASSED ===");
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[entry]
fn entry() -> ! {
    match run_loopback_test() {
        Ok(()) => {
            pw_log::info!("MCTP loopback test completed successfully");
            let _ = syscall::debug_shutdown(Ok(()));
        }
        Err(e) => {
            pw_log::error!("MCTP loopback test failed: {}", e as u32);
            let _ = syscall::debug_shutdown(Err(e));
        }
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

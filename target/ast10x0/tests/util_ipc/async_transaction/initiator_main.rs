// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Initiator side of the util/ipc AsyncTransaction QEMU test.
//!
//! Runs the cases below against `handler`, `ROUNDS` times over, and calls
//! `debug_shutdown(Ok(()))` on full pass or `debug_shutdown(Err(_))` on the
//! first failure. The kernel target writes `TEST_RESULT:PASS/FAIL` to UART.
//! Repeating catches state left behind by an earlier pass, a signal never
//! lowered above all.
//!
//! | Case               | Exercises                          | Expect            |
//! |--------------------|-------------------------------------|-------------------|
//! | blocking transact  | `IpcInitiator::transact`            | byte incremented  |
//! | async cancel       | `AsyncTransaction::start`/`cancel`  | channel freed     |
//! | async roundtrip    | `AsyncTransaction::start`/`try_recv`| byte incremented  |
//! | drop cancels       | `AsyncTransaction::drop`            | channel freed     |
//! | double start       | `start` while pending               | buffers returned  |
//! | try_recv early     | `try_recv` before the response      | `Ok(None)`        |
//! | send_len too big   | `start` past the end of `send`      | `OutOfRange`      |

#![no_main]
#![no_std]

use app_initiator::handle;
use pw_status::{Error, Result};
use userspace::entry;
use userspace::syscall::{self, Signals};
use userspace::time::{Clock, Instant, SystemClock};
use util_ipc::{AsyncTransaction, IpcHandle, IpcInitiator};

static mut SEND_BUF: [u8; 1] = [0x10];
static mut RECV_BUF: [u8; 1] = [0u8; 1];

/// Second pair of buffers, for the case that starts a transaction while
/// another one still borrows the first pair.
static mut RECV_BUF2: [u8; 1] = [0u8; 1];
static mut SEND_BUF2: [u8; 1] = [0x10];

/// Request byte the handler holds until Signals::USER is raised, matching
/// `GATED_REQUEST` in handler_main.rs.
static mut SEND_GATED: [u8; 1] = [0x40];

/// # Safety
/// Only called from this single-threaded app, and only while no
/// `AsyncTransaction` still holds a prior borrow of `RECV_BUF`.
unsafe fn recv_buf() -> &'static mut [u8] {
    // Safety: see function doc.
    unsafe { &mut *core::ptr::addr_of_mut!(RECV_BUF) }
}

/// # Safety
/// Same contract as `recv_buf`, for the request buffer. `start` takes
/// `send` mutably, so this hands out the same exclusive borrow.
unsafe fn send_buf() -> &'static mut [u8] {
    // Safety: see function doc.
    unsafe { &mut *core::ptr::addr_of_mut!(SEND_BUF) }
}

/// # Safety
/// Same contract as `send_buf`, for the second buffer.
unsafe fn send_buf2() -> &'static mut [u8] {
    // Safety: see function doc.
    unsafe { &mut *core::ptr::addr_of_mut!(SEND_BUF2) }
}

/// # Safety
/// Same contract as `send_buf`, for the gated request.
unsafe fn send_gated() -> &'static mut [u8] {
    // Safety: see function doc.
    unsafe { &mut *core::ptr::addr_of_mut!(SEND_GATED) }
}

/// # Safety
/// Same contract as `recv_buf`, for the second buffer.
unsafe fn recv_buf2() -> &'static mut [u8] {
    // Safety: see function doc.
    unsafe { &mut *core::ptr::addr_of_mut!(RECV_BUF2) }
}

fn test_blocking_transact() -> Result<()> {
    let ipc = IpcHandle::new(handle::IPC);
    let send = [0x20u8];
    let mut recv = [0u8; 1];

    let len = ipc.transact(&send, &mut recv, Instant::MAX)?;
    if len != 1 || recv[0] != 0x21 {
        pw_log::error!("blocking transact: unexpected response");
        return Err(Error::Internal);
    }
    Ok(())
}

fn test_async_cancel() -> Result<()> {
    let mut txn = AsyncTransaction::new(IpcHandle::new(handle::IPC));
    // Safety: no other AsyncTransaction is live right now.
    txn.start(unsafe { send_buf() }, 1, unsafe { recv_buf() })
        .map_err(|e| e.error)?;

    txn.cancel()?;
    if txn.is_pending() {
        pw_log::error!("async cancel: still pending after cancel()");
        return Err(Error::Internal);
    }

    // Verify the channel is free again.
    // Safety: the previous transaction was cancelled above.
    txn.start(unsafe { send_buf() }, 1, unsafe { recv_buf() })
        .map_err(|e| e.error)?;
    txn.cancel()?;
    Ok(())
}

fn test_async_roundtrip() -> Result<()> {
    let mut txn = AsyncTransaction::new(IpcHandle::new(handle::IPC));
    // Safety: no other AsyncTransaction is live right now.
    txn.start(unsafe { send_buf() }, 1, unsafe { recv_buf() })
        .map_err(|e| e.error)?;

    syscall::object_wait(txn.as_raw(), Signals::READABLE, Instant::MAX)?;

    let Some(completion) = txn.try_recv().map_err(|e| e.error)? else {
        pw_log::error!("async roundtrip: not ready after READABLE");
        return Err(Error::Internal);
    };
    if completion.len != 1 || completion.recv[0] != 0x11 {
        pw_log::error!("async roundtrip: unexpected response");
        return Err(Error::Internal);
    }
    Ok(())
}

/// Dropping a pending transaction has to cancel it, otherwise the channel
/// stays busy and the next start fails with `Unavailable`.
fn test_drop_cancels() -> Result<()> {
    {
        let mut txn = AsyncTransaction::new(IpcHandle::new(handle::IPC));
        // Safety: no other AsyncTransaction is live right now.
        txn.start(unsafe { send_buf() }, 1, unsafe { recv_buf() })
            .map_err(|e| e.error)?;
        // Dropped here with the transaction still pending.
    }

    let mut txn = AsyncTransaction::new(IpcHandle::new(handle::IPC));
    // Safety: the drop above cancelled the transaction. The kernel clears
    // its transaction slot on every path out of the cancel syscall, so it
    // no longer points into RECV_BUF.
    txn.start(unsafe { send_buf() }, 1, unsafe { recv_buf() })
        .map_err(|e| e.error)?;
    txn.cancel()?;
    Ok(())
}

/// A second start while one is pending fails locally and hands both
/// buffers back in the error.
fn test_double_start() -> Result<()> {
    let mut txn = AsyncTransaction::new(IpcHandle::new(handle::IPC));
    // Safety: no other AsyncTransaction is live right now.
    txn.start(unsafe { send_buf() }, 1, unsafe { recv_buf() })
        .map_err(|e| e.error)?;

    // The pending transaction still borrows SEND_BUF and RECV_BUF, so the
    // second start gets its own pair.
    // Safety: nothing else borrows SEND_BUF2 or RECV_BUF2.
    let result = txn.start(unsafe { send_buf2() }, 1, unsafe { recv_buf2() });

    let outcome = match result {
        Ok(()) => {
            pw_log::error!("double start: second start() succeeded");
            Err(Error::Internal)
        }
        Err(e) if e.error != Error::FailedPrecondition => {
            pw_log::error!("double start: status code {}", e.error as u32);
            Err(Error::Internal)
        }
        Err(e) => {
            let send_returned =
                core::ptr::eq(e.send.as_ptr(), core::ptr::addr_of!(SEND_BUF2).cast());
            let recv_returned =
                core::ptr::eq(e.recv.as_ptr(), core::ptr::addr_of!(RECV_BUF2).cast());
            if send_returned && recv_returned {
                Ok(())
            } else {
                pw_log::error!("double start: buffers not returned");
                Err(Error::Internal)
            }
        }
    };

    txn.cancel()?;
    outcome
}

/// `try_recv` before the handler responds reports `Ok(None)` and keeps the
/// transaction pending. The handler parks on this request and raises USER
/// to say so, so the observation is not a race: by the time USER arrives
/// the handler has read the request and is not going to respond until
/// released.
fn test_try_recv_before_response() -> Result<()> {
    let ipc = IpcHandle::new(handle::IPC);

    // The handler lowers the parked signal before it responds, so USER is
    // clear on entry. If it is still set, it leaked from an earlier round
    // and the wait below would return without the handler having parked.
    // A deadline of now makes this a poll rather than a wait.
    if syscall::object_wait(ipc.as_raw(), Signals::USER, SystemClock::now()).is_ok() {
        pw_log::error!("try_recv early: stale parked signal");
        return Err(Error::Internal);
    }

    let mut txn = AsyncTransaction::new(ipc);
    // Safety: no other AsyncTransaction is live right now.
    txn.start(unsafe { send_gated() }, 1, unsafe { recv_buf() })
        .map_err(|e| e.error)?;

    // Wait for the handler to say it is parked.
    syscall::object_wait(txn.as_raw(), Signals::USER, Instant::MAX)?;

    match txn.try_recv() {
        Ok(None) => {}
        Ok(Some(_)) => {
            pw_log::error!("try_recv early: completed before the handler responded");
            return Err(Error::Internal);
        }
        Err(e) => {
            pw_log::error!("try_recv early: status code {}", e.error as u32);
            return Err(Error::Internal);
        }
    }
    if !txn.is_pending() {
        pw_log::error!("try_recv early: transaction no longer pending");
        return Err(Error::Internal);
    }

    // Release the handler, then complete as usual.
    ipc.set_peer_user_signal(true)?;
    syscall::object_wait(txn.as_raw(), Signals::READABLE, Instant::MAX)?;
    let Some(completion) = txn.try_recv().map_err(|e| e.error)? else {
        pw_log::error!("try_recv early: not ready after READABLE");
        return Err(Error::Internal);
    };
    ipc.set_peer_user_signal(false)?;

    if completion.len != 1 || completion.recv[0] != 0x41 {
        pw_log::error!("try_recv early: unexpected response");
        return Err(Error::Internal);
    }
    Ok(())
}

/// A `send_len` longer than the buffer is refused before the syscall, and
/// the buffers come back so the caller can retry with a correct length.
fn test_send_len_out_of_range() -> Result<()> {
    let mut txn = AsyncTransaction::new(IpcHandle::new(handle::IPC));

    // Safety: no other AsyncTransaction is live right now.
    let result = txn.start(unsafe { send_buf() }, 2, unsafe { recv_buf() });

    match result {
        Ok(()) => {
            pw_log::error!("send_len out of range: start() succeeded");
            Err(Error::Internal)
        }
        Err(e) if e.error != Error::OutOfRange => {
            pw_log::error!("send_len out of range: status code {}", e.error as u32);
            Err(Error::Internal)
        }
        Err(_) if txn.is_pending() => {
            pw_log::error!("send_len out of range: left a transaction pending");
            Err(Error::Internal)
        }
        Err(_) => Ok(()),
    }
}

/// How often the whole sequence runs. Every case leaves the channel idle
/// and both USER signals lowered, so a repeat that fails means state leaked
/// from the pass before it.
const ROUNDS: u32 = 50;

fn run_all() -> Result<()> {
    for round in 0..ROUNDS {
        let ret = test_blocking_transact()
            .and_then(|_| test_async_cancel())
            .and_then(|_| test_async_roundtrip())
            .and_then(|_| test_drop_cancels())
            .and_then(|_| test_double_start())
            .and_then(|_| test_try_recv_before_response())
            .and_then(|_| test_send_len_out_of_range());

        if ret.is_err() {
            pw_log::error!("failed in round {}", round as u32);
            return ret;
        }
    }
    Ok(())
}

#[entry]
fn entry() {
    let ret = run_all();

    match &ret {
        Ok(()) => pw_log::info!("All test cases PASSED"),
        Err(e) => pw_log::error!("FAILED: status code {}", *e as u32),
    }

    let _ = syscall::debug_shutdown(ret);
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    let _ = userspace::syscall::debug_shutdown(Err(pw_status::Error::Internal));
    loop {}
}

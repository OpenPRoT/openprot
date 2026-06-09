// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use core::ptr::NonNull;
use core::ptr::{read_volatile, write_volatile};
use heapless::spsc::Queue;

const SWMBX_DEV_COUNT: usize = 2;
const SWMBX_NODE_COUNT: usize = 256;
const SWMBX_FIFO_COUNT: usize = 4;
const SWMBX_FIFO_DEPTH: usize = 256;

const _: () = assert!(SWMBX_NODE_COUNT == 256);
const _: () = assert!(SWMBX_NODE_COUNT == (u8::MAX as usize) + 1);

/// Number of address bits packed into each protection bitmap word.
const PROTECT_BITS_PER_WORD: usize = 32;
/// Shift converting a protection word index into its base node address.
const PROTECT_WORD_SHIFT: u32 = PROTECT_BITS_PER_WORD.ilog2();
/// Count of 32-bit words needed to cover every node's protection bit.
const PROTECT_WORD_COUNT: usize = SWMBX_NODE_COUNT / PROTECT_BITS_PER_WORD;

const SWMBX_PROTECT: u8 = 1 << 0;
const SWMBX_NOTIFY: u8 = 1 << 1;
const SWMBX_FIFO: u8 = 1 << 2;
const FLAG_MASK: u8 = SWMBX_PROTECT | SWMBX_NOTIFY | SWMBX_FIFO;

const SWMBX_FIFO_NOTIFY_START: u8 = 1 << 0;
const SWMBX_FIFO_NOTIFY_STOP: u8 = 1 << 1;

/// Error type for SW mailbox controller operations.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum SwmbxError {
    InvalidPort = 0x1001,
    InvalidAddress = 0x1002,
    InvalidFifoIndex = 0x1003,
    InvalidFifoDepth = 0x1004,
    InvalidFlagMask = 0x1005,
    InvalidProtectRange = 0x1006,
    FifoFull = 0x1007,
    FifoEmpty = 0x1008,
    // 0x1009 retired: a null/misaligned region base is now a panic (programming
    // bug), not a recoverable error. Reserved to keep the diagnostic-code ABI stable.
    FifoNotMapped = 0x100A,
}

impl SwmbxError {
    /// Stable numeric diagnostic code for firmware telemetry/logging.
    pub const fn code(self) -> u16 {
        self as u16
    }
}

struct SharedRegion<T> {
    base: NonNull<T>,
}

impl<T> SharedRegion<T> {
    /// Wraps a caller-provided region base address.
    ///
    /// # Safety
    /// `addr` must, for the lifetime of the returned region, point to a single
    /// valid, mapped, and uniquely-owned `T` (i.e. dereferenceable with correct
    /// provenance and no conflicting aliases). Volatile reads/writes are issued
    /// against it; an invalid address is undefined behavior.
    ///
    /// # Panics
    /// Panics if `addr` is null or not aligned for `T`. When `addr` is a
    /// compile-time constant, this is reported as a compile error instead.
    #[inline]
    pub const unsafe fn from_addr(addr: usize) -> Self {
        assert!(addr != 0, "region base is null");
        assert!(
            addr % core::mem::align_of::<T>() == 0,
            "region base is misaligned for T"
        );
        // SAFETY: non-null is asserted above; full validity is guaranteed by
        // this function's own safety contract, which the caller upholds.
        Self {
            base: unsafe { NonNull::new_unchecked(addr as *mut T) },
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct SwmbxFifoEntry {
    value: u8,
}

#[derive(Copy, Clone, Debug, Default)]
struct SwmbxNode {
    notify_flag: bool,
    enabled_flags: u8,
}

struct SwmbxFifo<const N: usize> {
    queue: Queue<SwmbxFifoEntry, N>,
    notify_flag: u8,
    notify_start: bool,
    fifo_write: bool,
    fifo_offset: u8,
    enabled: bool,
    msg_index: usize,
    max_msg_count: usize,
}

impl<const N: usize> SwmbxFifo<N> {
    pub const fn new() -> Self {
        Self {
            queue: Queue::new(),
            notify_flag: 0,
            notify_start: false,
            fifo_write: false,
            fifo_offset: 0,
            enabled: false,
            msg_index: 0,
            max_msg_count: SWMBX_FIFO_DEPTH,
        }
    }

    pub fn append_write(&mut self, value: u8) -> Result<(), SwmbxError> {
        if self.max_msg_count == 0 {
            return Err(SwmbxError::InvalidFifoDepth);
        }

        if self.queue.len() < self.max_msg_count {
            self.queue
                .enqueue(SwmbxFifoEntry { value })
                .map_err(|_| SwmbxError::FifoFull)?;
            if self.msg_index == (self.max_msg_count - 1) {
                self.msg_index = 0;
            } else {
                self.msg_index = (self.msg_index + 1) % self.max_msg_count;
            }
            Ok(())
        } else {
            Err(SwmbxError::FifoFull)
        }
    }

    /// Removes and returns the oldest byte, or `FifoEmpty` if drained.
    pub fn dequeue(&mut self) -> Result<u8, SwmbxError> {
        if let Some(entry) = self.queue.dequeue() {
            Ok(entry.value)
        } else {
            Err(SwmbxError::FifoEmpty)
        }
    }

    pub fn flush(&mut self) {
        while self.queue.dequeue().is_some() {}
        self.msg_index = 0;
        self.notify_start = false;
        self.fifo_write = false;
    }
}

/// In-memory software mailbox controller.
///
/// Models a shared register file with three overlaid behaviors: flag-gated
/// per-node write **protection** and change **notification**, plus optional
/// **FIFO** remapping of selected addresses. Protection and notification are
/// pure flag-gated policy applied per access; only the FIFO path is stateful,
/// running a small per-port transaction lifecycle (`send_start` opens a FIFO
/// transaction, `send_msg`/`get_msg` append/drain it while open, `send_stop`
/// closes it).
pub struct SwmbxCtrl {
    // Global feature switches (SWMBX_PROTECT/NOTIFY/FIFO). Plain `u8`: the
    // controller is driven from a single execution context (the i2c service's
    // cooperative event loop feeds it serially over IPC), so no interior
    // mutability or atomics are needed. Keeping it non-`Cell` also leaves the
    // type `!Sync`, so accidental cross-context sharing is a compile error.
    mbx_en: u8,
    node: [[SwmbxNode; SWMBX_NODE_COUNT]; SWMBX_DEV_COUNT],
    fifo: [SwmbxFifo<SWMBX_FIFO_DEPTH>; SWMBX_FIFO_COUNT],
    mbx_fifo_execute: [bool; SWMBX_DEV_COUNT],
    mbx_fifo_addr: [u8; SWMBX_DEV_COUNT],
    mbx_fifo_idx: [u8; SWMBX_DEV_COUNT],
    buffer_base: NonNull<u8>,
    buffer_size: usize,
}

impl SwmbxCtrl {
    #[inline]
    fn read_in_region(&self, offset: u8) -> Result<u8, SwmbxError> {
        if (offset as usize) >= self.buffer_size {
            return Err(SwmbxError::InvalidAddress);
        }
        // SAFETY: `buffer_base` was validated once during construction and
        // remains valid by `new_with_regions`'s safety contract. Bounds are
        // checked above, so `add(offset)` stays within the mapped region.
        unsafe {
            Ok(read_volatile(
                self.buffer_base.as_ptr().add(offset as usize),
            ))
        }
    }

    #[inline]
    fn write_in_region(&self, offset: u8, val: u8) -> Result<(), SwmbxError> {
        if (offset as usize) >= self.buffer_size {
            return Err(SwmbxError::InvalidAddress);
        }
        // SAFETY: see `read_in_region`; the cached base pointer is valid and
        // the checked offset keeps the address within the mapped region.
        unsafe { write_volatile(self.buffer_base.as_ptr().add(offset as usize), val) };
        Ok(())
    }

    /// Creates a controller backed by a caller-provided buffer region.
    ///
    /// # Safety
    /// `buffer_base` must point to at least `buffer_size` bytes of valid, mapped,
    /// uniquely-owned memory for the controller's lifetime; subsequent reads and
    /// writes (e.g. via [`Self::get_msg`]/[`Self::send_msg`]) dereference it.
    /// Passing an invalid address is undefined behavior even though later
    /// accessor methods are safe.
    pub unsafe fn new_with_regions(buffer_size: usize, buffer_base: usize) -> Self {
        // SAFETY: `buffer_base` validity is part of this function's safety
        // contract. We validate null/alignment once and cache the pointer.
        let buffer_base = unsafe { SharedRegion::<u8>::from_addr(buffer_base).base };

        SwmbxCtrl {
            mbx_en: 0,
            node: [[SwmbxNode::default(); SWMBX_NODE_COUNT]; SWMBX_DEV_COUNT],
            fifo: [
                SwmbxFifo::new(),
                SwmbxFifo::new(),
                SwmbxFifo::new(),
                SwmbxFifo::new(),
            ],
            mbx_fifo_execute: [false; SWMBX_DEV_COUNT],
            mbx_fifo_addr: [0; SWMBX_DEV_COUNT],
            mbx_fifo_idx: [0; SWMBX_DEV_COUNT],
            buffer_base,
            buffer_size,
        }
    }

    /// Enables or disables notification for a single node on a given port.
    ///
    /// When enabled, writes targeting `addr` can latch `notify_flag` for this
    /// port, provided global notify behavior is also enabled via
    /// [`Self::enable_behavior`].
    ///
    /// # Errors
    /// Returns [`SwmbxError::InvalidPort`] if `port` is out of range.
    /// Returns [`SwmbxError::InvalidAddress`] if `addr` is out of range.
    pub fn update_notify(&mut self, port: usize, addr: u8, enable: bool) -> Result<(), SwmbxError> {
        if port >= SWMBX_DEV_COUNT {
            return Err(SwmbxError::InvalidPort);
        }
        if (addr as usize) >= SWMBX_NODE_COUNT {
            return Err(SwmbxError::InvalidAddress);
        }

        let node = &mut self.node[port][addr as usize];
        if enable {
            node.enabled_flags |= SWMBX_NOTIFY;
        } else {
            node.enabled_flags &= !SWMBX_NOTIFY;
        }
        Ok(())
    }

    /// Configures one FIFO endpoint mapping.
    ///
    /// If `enable` is `true`, this binds FIFO `index` to `addr`, applies the
    /// requested queue depth and notify mask, and resets transient transaction
    /// state for that FIFO. If `enable` is `false`, the FIFO is disabled and
    /// drained/reset.
    ///
    /// # Errors
    /// Returns [`SwmbxError::InvalidFifoIndex`] if `index` is out of range.
    /// Returns [`SwmbxError::InvalidFifoDepth`] if enabling with depth `0` or
    /// greater than [`SWMBX_FIFO_DEPTH`].
    /// Returns [`SwmbxError::InvalidAddress`] if `addr` is out of range.
    pub fn update_fifo(
        &mut self,
        index: usize,
        addr: u8,
        depth: usize,
        notify: u8,
        enable: bool,
    ) -> Result<(), SwmbxError> {
        if index >= SWMBX_FIFO_COUNT {
            return Err(SwmbxError::InvalidFifoIndex);
        }

        if enable && (depth == 0 || depth > SWMBX_FIFO_DEPTH) {
            return Err(SwmbxError::InvalidFifoDepth);
        }

        if (addr as usize) >= SWMBX_NODE_COUNT {
            return Err(SwmbxError::InvalidAddress);
        }

        let fifo = &mut self.fifo[index];
        fifo.enabled = enable;

        if enable {
            fifo.fifo_offset = addr;
            fifo.max_msg_count = depth;
            fifo.notify_flag = notify;
            fifo.msg_index = 0;
            fifo.notify_start = false;
            fifo.fifo_write = false;
            fifo.queue = Queue::new();
        } else {
            fifo.notify_start = false;
            fifo.fifo_write = false;
            fifo.queue = Queue::new();
        }

        Ok(())
    }

    /// Flushes all pending messages from FIFO `index`.
    ///
    /// This also resets FIFO-local transient state such as START latch and
    /// transaction write tracking flags.
    ///
    /// # Errors
    /// Returns [`SwmbxError::InvalidFifoIndex`] if `index` is out of range.
    pub fn flush_fifo(&mut self, index: usize) -> Result<(), SwmbxError> {
        if index >= SWMBX_FIFO_COUNT {
            return Err(SwmbxError::InvalidFifoIndex);
        }

        self.fifo[index].flush();
        Ok(())
    }

    /// Enables or disables global SWMBX behavior flags.
    ///
    /// # Errors
    /// Returns [`SwmbxError::InvalidFlagMask`] if `flag` contains no known
    /// behavior bits ([`FLAG_MASK`]).
    pub fn enable_behavior(&mut self, flag: u8, enable: bool) -> Result<(), SwmbxError> {
        if (flag & FLAG_MASK) == 0 {
            return Err(SwmbxError::InvalidFlagMask);
        }

        if enable {
            self.mbx_en |= flag;
        } else {
            self.mbx_en &= !flag;
        }

        Ok(())
    }

    /// Enables or disables write protection for one node on a given port.
    ///
    /// A protected node suppresses writes only when global protect behavior is
    /// also enabled via [`Self::enable_behavior`].
    ///
    /// # Errors
    /// Returns [`SwmbxError::InvalidPort`] if `port` is out of range.
    /// Returns [`SwmbxError::InvalidAddress`] if `addr` is out of range.
    pub fn update_protect(
        &mut self,
        port: usize,
        addr: u8,
        enable: bool,
    ) -> Result<(), SwmbxError> {
        if port >= SWMBX_DEV_COUNT {
            return Err(SwmbxError::InvalidPort);
        }
        if (addr as usize) >= SWMBX_NODE_COUNT {
            return Err(SwmbxError::InvalidAddress);
        }

        let node = &mut self.node[port][addr as usize];
        if enable {
            node.enabled_flags |= SWMBX_PROTECT;
        } else {
            node.enabled_flags &= !SWMBX_PROTECT;
        }

        Ok(())
    }

    /// Applies protection bits from packed 32-bit words.
    ///
    /// # Errors
    /// Returns [`SwmbxError::InvalidPort`] if `port` is out of range, or
    /// [`SwmbxError::InvalidProtectRange`] if `start_idx` plus `bitmap.len()`
    /// would exceed [`PROTECT_WORD_COUNT`] (including on arithmetic overflow).
    pub fn apply_protect(
        &mut self,
        port: usize,
        bitmap: &[u32],
        start_idx: usize,
    ) -> Result<(), SwmbxError> {
        if port >= SWMBX_DEV_COUNT {
            return Err(SwmbxError::InvalidPort);
        }
        // Checked add so a large `start_idx` cannot wrap past this guard and
        // later overflow the `(start_idx + i) << PROTECT_WORD_SHIFT` shift.
        match start_idx.checked_add(bitmap.len()) {
            Some(end) if end <= PROTECT_WORD_COUNT => {}
            _ => return Err(SwmbxError::InvalidProtectRange),
        }

        for (i, &val) in bitmap.iter().enumerate() {
            let base = (start_idx + i) << PROTECT_WORD_SHIFT;
            for bit in 0..PROTECT_BITS_PER_WORD {
                let addr = base + bit;
                if addr >= SWMBX_NODE_COUNT {
                    break;
                }
                let node = &mut self.node[port][addr];
                if (val >> bit) & 1 != 0 {
                    node.enabled_flags |= SWMBX_PROTECT;
                } else {
                    node.enabled_flags &= !SWMBX_PROTECT;
                }
            }
        }
        Ok(())
    }

    /// Reads one byte for `port` and `addr`.
    ///
    /// If a FIFO transaction is active for `port` and FIFO behavior is enabled,
    /// this dequeues from the mapped FIFO. Otherwise it reads directly from the
    /// backing mailbox region.
    ///
    /// # Errors
    /// Returns [`SwmbxError::InvalidPort`] if `port` is out of range.
    /// Returns [`SwmbxError::FifoEmpty`] when reading from an empty active FIFO.
    /// Returns [`SwmbxError::InvalidAddress`] for out-of-bounds flat-buffer
    /// reads relative to `buffer_size`.
    pub fn get_msg(&mut self, port: usize, addr: u8) -> Result<u8, SwmbxError> {
        if port >= SWMBX_DEV_COUNT {
            return Err(SwmbxError::InvalidPort);
        }

        if self.mbx_fifo_execute[port] && (self.mbx_en & SWMBX_FIFO) != 0 {
            let fifo_index = self.mbx_fifo_idx[port] as usize;
            return self.fifo[fifo_index].dequeue();
        }

        self.read_in_region(addr)
    }

    /// Writes one byte for `port` and `addr`.
    ///
    /// If a FIFO transaction is active for `port` and FIFO behavior is enabled,
    /// the value is appended to the active FIFO. Otherwise this applies protect
    /// and notify policy to a flat-buffer write path.
    ///
    /// # Errors
    /// Returns [`SwmbxError::InvalidPort`] if `port` is out of range.
    /// Returns [`SwmbxError::FifoFull`] when appending to a full active FIFO.
    /// Returns [`SwmbxError::InvalidAddress`] for out-of-bounds flat-buffer
    /// writes relative to `buffer_size`.
    pub fn send_msg(&mut self, port: usize, addr: u8, val: u8) -> Result<(), SwmbxError> {
        if port >= SWMBX_DEV_COUNT {
            return Err(SwmbxError::InvalidPort);
        }

        let mut write_to_buffer = false;

        if self.mbx_fifo_execute[port] && (self.mbx_en & SWMBX_FIFO) != 0 {
            let fifo_addr = self.mbx_fifo_addr[port];
            let fifo_index = self.mbx_fifo_idx[port] as usize;

            if let Err(err) = self.fifo[fifo_index].append_write(val) {
                self.node[port][addr as usize].notify_flag = true;
                return Err(err);
            }

            if (self.mbx_en & SWMBX_NOTIFY) != 0
                && (self.fifo[fifo_index].notify_flag & SWMBX_FIFO_NOTIFY_START) != 0
                && !self.fifo[fifo_index].notify_start
                && (self.node[port][fifo_addr as usize].enabled_flags & SWMBX_NOTIFY) != 0
            {
                self.node[port][fifo_addr as usize].notify_flag = true;
                self.fifo[fifo_index].notify_start = true;
            }

            if !self.fifo[fifo_index].fifo_write {
                self.fifo[fifo_index].fifo_write = true;
            }
        } else {
            let node = &mut self.node[port][addr as usize];

            if (node.enabled_flags & SWMBX_PROTECT) == 0 || (self.mbx_en & SWMBX_PROTECT) == 0 {
                write_to_buffer = true;
            }

            if (self.mbx_en & SWMBX_NOTIFY) != 0 && (node.enabled_flags & SWMBX_NOTIFY) != 0 {
                node.notify_flag = true;
            }
        }

        if write_to_buffer {
            self.write_in_region(addr, val)?;
        }

        Ok(())
    }

    /// Starts a transaction on `port` for `addr`.
    ///
    /// If `addr` resolves to an enabled FIFO mapping, the port switches into
    /// FIFO transaction mode for subsequent [`Self::send_msg`]/[`Self::get_msg`]
    /// calls until [`Self::send_stop`]. Otherwise the port remains in flat
    /// buffer mode.
    ///
    /// # Errors
    /// Returns [`SwmbxError::InvalidPort`] if `port` is out of range.
    pub fn send_start(&mut self, port: usize, addr: u8) -> Result<(), SwmbxError> {
        if port >= SWMBX_DEV_COUNT {
            return Err(SwmbxError::InvalidPort);
        }

        if let Some(fifo_index) = self.check_fifo(addr) {
            self.mbx_fifo_execute[port] = true;
            self.mbx_fifo_addr[port] = addr;
            self.mbx_fifo_idx[port] = fifo_index as u8;
        } else {
            self.mbx_fifo_execute[port] = false;
        }

        Ok(())
    }

    /// Returns the enabled FIFO index mapped to `addr`, if any.
    pub fn check_fifo(&self, addr: u8) -> Option<usize> {
        self.fifo
            .iter()
            .position(|f| f.fifo_offset == addr && f.enabled)
    }

    /// Stops the current transaction for `port`.
    ///
    /// For FIFO transactions, this finalizes STOP-notify behavior and clears
    /// per-port FIFO execution state.
    ///
    /// # Errors
    /// Returns [`SwmbxError::InvalidPort`] if `port` is out of range.
    pub fn send_stop(&mut self, port: usize) -> Result<(), SwmbxError> {
        if port >= SWMBX_DEV_COUNT {
            return Err(SwmbxError::InvalidPort);
        }

        if self.mbx_fifo_execute[port] {
            let fifo_addr = self.mbx_fifo_addr[port];
            let fifo_index = self.mbx_fifo_idx[port] as usize;

            if (self.mbx_en & SWMBX_NOTIFY) != 0
                && (self.fifo[fifo_index].notify_flag & SWMBX_FIFO_NOTIFY_STOP) != 0
                && self.fifo[fifo_index].fifo_write
                && (self.node[port][fifo_addr as usize].enabled_flags & SWMBX_NOTIFY) != 0
            {
                self.node[port][fifo_addr as usize].notify_flag = true;
            }

            self.fifo[fifo_index].notify_start = false;
            self.fifo[fifo_index].fifo_write = false;
            self.mbx_fifo_execute[port] = false;
            self.mbx_fifo_addr[port] = 0;
            self.mbx_fifo_idx[port] = 0;
        }

        Ok(())
    }

    /// Direct software write helper.
    ///
    /// When `fifo` is `true`, writes to the FIFO mapped at `addr` regardless of
    /// transaction state. When `fifo` is `false`, writes directly to the flat
    /// backing mailbox region.
    ///
    /// # Errors
    /// Returns [`SwmbxError::FifoNotMapped`] when `fifo` is `true` and no FIFO
    /// is mapped to `addr`.
    /// Returns [`SwmbxError::FifoFull`] when writing to a full mapped FIFO.
    /// Returns [`SwmbxError::InvalidAddress`] for out-of-bounds flat-buffer
    /// writes relative to `buffer_size`.
    pub fn swmbx_write(&mut self, fifo: bool, addr: u8, val: u8) -> Result<(), SwmbxError> {
        if fifo {
            if let Some(index) = self.check_fifo(addr) {
                self.fifo[index].append_write(val)?;
                Ok(())
            } else {
                Err(SwmbxError::FifoNotMapped)
            }
        } else {
            self.write_in_region(addr, val)
        }
    }

    /// Direct software read helper.
    ///
    /// When `fifo` is `true`, reads from the FIFO mapped at `addr` regardless
    /// of transaction state. When `fifo` is `false`, reads directly from the
    /// flat backing mailbox region.
    ///
    /// # Errors
    /// Returns [`SwmbxError::FifoNotMapped`] when `fifo` is `true` and no FIFO
    /// is mapped to `addr`.
    /// Returns [`SwmbxError::FifoEmpty`] when reading an empty mapped FIFO.
    /// Returns [`SwmbxError::InvalidAddress`] for out-of-bounds flat-buffer
    /// reads relative to `buffer_size`.
    pub fn swmbx_read(&mut self, fifo: bool, addr: u8) -> Result<u8, SwmbxError> {
        if fifo {
            if let Some(index) = self.check_fifo(addr) {
                self.fifo[index].dequeue()
            } else {
                Err(SwmbxError::FifoNotMapped)
            }
        } else {
            self.read_in_region(addr)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_backed_buffer_read_write() {
        let mut host_buffer = [0u8; SWMBX_NODE_COUNT];

        // SAFETY: the buffer region is backed by live, uniquely-owned host stack
        // storage (`host_buffer`) for the duration of the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        ctrl.swmbx_write(false, 0x13, 0x7a)
            .expect("buffer write failed");
        let val = ctrl.swmbx_read(false, 0x13).expect("buffer read failed");
        assert_eq!(val, 0x7a);
    }

    #[test]
    fn fifo_flow_works_with_host_backing_memory() {
        let mut host_buffer = [0u8; SWMBX_NODE_COUNT];
        // SAFETY: `buffer_base` is live, uniquely-owned host storage for the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        ctrl.enable_behavior(SWMBX_FIFO | SWMBX_NOTIFY, true)
            .expect("enable behavior failed");
        ctrl.update_fifo(
            0,
            0x0d,
            4,
            SWMBX_FIFO_NOTIFY_START | SWMBX_FIFO_NOTIFY_STOP,
            true,
        )
        .expect("fifo setup failed");
        ctrl.update_notify(0, 0x0d, true)
            .expect("notify setup failed");

        ctrl.send_start(0, 0x0d).expect("send_start failed");
        ctrl.send_msg(0, 0x0d, 0x11).expect("send_msg #1 failed");
        ctrl.send_msg(0, 0x0d, 0x22).expect("send_msg #2 failed");

        assert_eq!(ctrl.get_msg(0, 0x0d).expect("get_msg #1 failed"), 0x11);
        assert_eq!(ctrl.get_msg(0, 0x0d).expect("get_msg #2 failed"), 0x22);
        assert_eq!(ctrl.get_msg(0, 0x0d), Err(SwmbxError::FifoEmpty));

        ctrl.send_stop(0).expect("send_stop failed");
        assert!(!ctrl.mbx_fifo_execute[0]);
        assert!(!ctrl.fifo[0].notify_start);
        assert!(!ctrl.fifo[0].fifo_write);
    }

    #[test]
    fn apply_protect_rejects_out_of_range_without_overflow() {
        let mut dummy = 0u8;
        // SAFETY: this test only exercises `apply_protect`, which never touches
        // the buffer region; we still pass a non-null base because
        // `new_with_regions` validates it at construction time.
        let mut ctrl = unsafe { SwmbxCtrl::new_with_regions(0, &mut dummy as *mut u8 as usize) };

        // A `start_idx` near `usize::MAX` must be rejected, not wrap past the
        // range guard and overflow the internal shift.
        assert_eq!(
            ctrl.apply_protect(0, &[0xffff_ffff], usize::MAX),
            Err(SwmbxError::InvalidProtectRange)
        );

        // Just past the last valid word is also rejected.
        assert_eq!(
            ctrl.apply_protect(0, &[0], PROTECT_WORD_COUNT),
            Err(SwmbxError::InvalidProtectRange)
        );

        // A full, in-range bitmap is accepted.
        ctrl.apply_protect(0, &[0u32; PROTECT_WORD_COUNT], 0)
            .expect("full in-range bitmap should apply");
    }

    #[test]
    fn flush_fifo_resets_start_latch_and_rearms_start_notify() {
        let mut host_buffer = [0u8; SWMBX_NODE_COUNT];
        // SAFETY: `buffer_base` is live, uniquely-owned host storage for the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        ctrl.enable_behavior(SWMBX_FIFO | SWMBX_NOTIFY, true)
            .expect("enable behavior failed");
        ctrl.update_fifo(0, 0x0d, 4, SWMBX_FIFO_NOTIFY_START, true)
            .expect("fifo setup failed");
        ctrl.update_notify(0, 0x0d, true)
            .expect("notify setup failed");

        ctrl.send_start(0, 0x0d).expect("send_start #1 failed");
        ctrl.send_msg(0, 0x0d, 0x11).expect("send_msg #1 failed");
        assert!(ctrl.fifo[0].notify_start);
        assert!(ctrl.fifo[0].fifo_write);

        // Simulate consumption of the first notification before the next transaction.
        ctrl.node[0][0x0d].notify_flag = false;

        ctrl.flush_fifo(0).expect("flush_fifo failed");
        assert!(!ctrl.fifo[0].notify_start);
        assert!(!ctrl.fifo[0].fifo_write);

        ctrl.send_stop(0).expect("send_stop failed");
        ctrl.send_start(0, 0x0d).expect("send_start #2 failed");
        ctrl.send_msg(0, 0x0d, 0x22).expect("send_msg #2 failed");

        assert!(ctrl.node[0][0x0d].notify_flag);
        assert!(ctrl.fifo[0].notify_start);
    }

    #[test]
    fn update_fifo_rejects_invalid_configuration() {
        let mut host_buffer = [0u8; SWMBX_NODE_COUNT];
        // SAFETY: `buffer_base` is live, uniquely-owned host storage for the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        assert_eq!(
            ctrl.update_fifo(SWMBX_FIFO_COUNT, 0x0d, 4, 0, true),
            Err(SwmbxError::InvalidFifoIndex)
        );
        assert_eq!(
            ctrl.update_fifo(0, 0x0d, 0, 0, true),
            Err(SwmbxError::InvalidFifoDepth)
        );
        assert_eq!(
            ctrl.update_fifo(0, 0x0d, SWMBX_FIFO_DEPTH + 1, 0, true),
            Err(SwmbxError::InvalidFifoDepth)
        );
        // `addr` is `u8`, so all values up to 0xff are representable and valid
        // for a 256-node mailbox.
        assert_eq!(ctrl.update_fifo(0, 0xff, 4, 0, true), Ok(()));
    }

    #[test]
    fn flat_buffer_path_honors_protect_and_notify_gating() {
        let mut host_buffer = [0u8; SWMBX_NODE_COUNT];
        // SAFETY: `buffer_base` is live, uniquely-owned host storage for the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        let addr = 0x21;

        // Baseline write works.
        ctrl.send_msg(0, addr, 0x12)
            .expect("baseline send_msg failed");
        assert_eq!(
            ctrl.get_msg(0, addr).expect("baseline get_msg failed"),
            0x12
        );

        // With node PROTECT enabled but global PROTECT disabled, writes still go through.
        ctrl.update_protect(0, addr, true)
            .expect("update_protect failed");
        ctrl.send_msg(0, addr, 0x34)
            .expect("send_msg with global protect disabled failed");
        assert_eq!(
            ctrl.get_msg(0, addr)
                .expect("get_msg with global protect disabled failed"),
            0x34
        );

        // Enabling global PROTECT blocks writes to protected nodes.
        ctrl.enable_behavior(SWMBX_PROTECT, true)
            .expect("enable protect failed");
        ctrl.send_msg(0, addr, 0x56)
            .expect("send_msg with protect enabled failed");
        assert_eq!(
            ctrl.get_msg(0, addr)
                .expect("get_msg after blocked protected write failed"),
            0x34
        );

        // Notify does not latch until both node-notify and global notify are enabled.
        assert!(!ctrl.node[0][addr as usize].notify_flag);
        ctrl.send_msg(0, addr, 0x78)
            .expect("send_msg before notify enable failed");
        assert!(!ctrl.node[0][addr as usize].notify_flag);

        ctrl.update_notify(0, addr, true)
            .expect("update_notify failed");
        ctrl.send_msg(0, addr, 0x9a)
            .expect("send_msg with only node notify failed");
        assert!(!ctrl.node[0][addr as usize].notify_flag);

        ctrl.enable_behavior(SWMBX_NOTIFY, true)
            .expect("enable notify failed");
        ctrl.send_msg(0, addr, 0xbc)
            .expect("send_msg with notify enabled failed");
        assert!(ctrl.node[0][addr as usize].notify_flag);
    }

    #[test]
    fn fifo_full_returns_error_and_sets_node_notify() {
        let mut host_buffer = [0u8; SWMBX_NODE_COUNT];
        // SAFETY: `buffer_base` is live, uniquely-owned host storage for the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        let addr = 0x0d;
        ctrl.enable_behavior(SWMBX_FIFO, true)
            .expect("enable fifo failed");
        ctrl.update_fifo(0, addr, 1, 0, true)
            .expect("fifo setup failed");

        ctrl.send_start(0, addr).expect("send_start failed");
        ctrl.send_msg(0, addr, 0x11)
            .expect("first fifo write failed");

        assert_eq!(ctrl.send_msg(0, addr, 0x22), Err(SwmbxError::FifoFull));
        assert!(ctrl.node[0][addr as usize].notify_flag);
    }

    #[test]
    fn send_stop_notify_requires_node_notify_and_write() {
        let mut host_buffer = [0u8; SWMBX_NODE_COUNT];
        // SAFETY: `buffer_base` is live, uniquely-owned host storage for the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        let addr = 0x0d;
        ctrl.enable_behavior(SWMBX_FIFO | SWMBX_NOTIFY, true)
            .expect("enable behavior failed");
        ctrl.update_fifo(0, addr, 4, SWMBX_FIFO_NOTIFY_STOP, true)
            .expect("fifo setup failed");

        // STOP notify configured, but node notify disabled: no latch.
        ctrl.send_start(0, addr).expect("send_start #1 failed");
        ctrl.send_msg(0, addr, 0x11).expect("send_msg #1 failed");
        ctrl.send_stop(0).expect("send_stop #1 failed");
        assert!(!ctrl.node[0][addr as usize].notify_flag);

        // Enable node notify but do not write during transaction: no STOP notify latch.
        ctrl.update_notify(0, addr, true)
            .expect("update_notify failed");
        ctrl.node[0][addr as usize].notify_flag = false;
        ctrl.send_start(0, addr).expect("send_start #2 failed");
        ctrl.send_stop(0).expect("send_stop #2 failed");
        assert!(!ctrl.node[0][addr as usize].notify_flag);

        // With node notify enabled and a write in transaction: STOP notify latches.
        ctrl.send_start(0, addr).expect("send_start #3 failed");
        ctrl.send_msg(0, addr, 0x22).expect("send_msg #2 failed");
        ctrl.send_stop(0).expect("send_stop #3 failed");
        assert!(ctrl.node[0][addr as usize].notify_flag);
    }

    #[test]
    fn fifo_start_notify_latches_once_and_rearms_after_stop() {
        let mut host_buffer = [0u8; SWMBX_NODE_COUNT];
        // SAFETY: `buffer_base` is live, uniquely-owned host storage for the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        let addr = 0x0d;
        ctrl.enable_behavior(SWMBX_FIFO | SWMBX_NOTIFY, true)
            .expect("enable behavior failed");
        ctrl.update_fifo(0, addr, 4, SWMBX_FIFO_NOTIFY_START, true)
            .expect("fifo setup failed");
        ctrl.update_notify(0, addr, true)
            .expect("update_notify failed");

        ctrl.send_start(0, addr).expect("send_start #1 failed");
        ctrl.send_msg(0, addr, 0x11).expect("send_msg #1 failed");
        assert!(ctrl.node[0][addr as usize].notify_flag);
        assert!(ctrl.fifo[0].notify_start);

        // START-notify is one-shot during a single transaction.
        ctrl.node[0][addr as usize].notify_flag = false;
        ctrl.send_msg(0, addr, 0x22).expect("send_msg #2 failed");
        assert!(!ctrl.node[0][addr as usize].notify_flag);

        // STOP clears the latch; next transaction can notify START again.
        ctrl.send_stop(0).expect("send_stop failed");
        ctrl.send_start(0, addr).expect("send_start #2 failed");
        ctrl.send_msg(0, addr, 0x33).expect("send_msg #3 failed");
        assert!(ctrl.node[0][addr as usize].notify_flag);
    }

    #[test]
    fn send_start_with_non_fifo_addr_falls_back_to_flat_buffer() {
        let mut host_buffer = [0u8; SWMBX_NODE_COUNT];
        // SAFETY: `buffer_base` is live, uniquely-owned host storage for the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        let fifo_addr = 0x0d;
        let flat_addr = 0x20;
        ctrl.enable_behavior(SWMBX_FIFO, true)
            .expect("enable fifo failed");
        ctrl.update_fifo(0, fifo_addr, 4, 0, true)
            .expect("fifo setup failed");

        ctrl.send_start(0, fifo_addr)
            .expect("send_start fifo failed");
        assert!(ctrl.mbx_fifo_execute[0]);

        // New START to a non-FIFO offset must disable FIFO execute for this port.
        ctrl.send_start(0, flat_addr)
            .expect("send_start flat-buffer failed");
        assert!(!ctrl.mbx_fifo_execute[0]);

        ctrl.send_msg(0, flat_addr, 0x5a)
            .expect("flat-buffer send_msg failed");
        assert_eq!(
            ctrl.get_msg(0, flat_addr)
                .expect("flat-buffer get_msg failed"),
            0x5a
        );
    }

    #[test]
    fn apply_protect_bitmap_maps_first_and_last_node_bits() {
        let mut host_buffer = [0u8; SWMBX_NODE_COUNT];
        // SAFETY: `buffer_base` is live, uniquely-owned host storage for the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        // Set protect bit for address 0.
        ctrl.apply_protect(0, &[1], 0)
            .expect("apply_protect low failed");
        assert_ne!(ctrl.node[0][0].enabled_flags & SWMBX_PROTECT, 0);
        assert_eq!(ctrl.node[0][1].enabled_flags & SWMBX_PROTECT, 0);

        // Set protect bit for address 255 (bit 31 of last word).
        ctrl.apply_protect(0, &[1u32 << 31], PROTECT_WORD_COUNT - 1)
            .expect("apply_protect high failed");
        assert_ne!(ctrl.node[0][255].enabled_flags & SWMBX_PROTECT, 0);
        assert_eq!(ctrl.node[0][254].enabled_flags & SWMBX_PROTECT, 0);

        ctrl.enable_behavior(SWMBX_PROTECT, true)
            .expect("enable protect failed");

        // Protected edges block writes when global protect is enabled.
        ctrl.send_msg(0, 0x00, 0xaa)
            .expect("send_msg protected low failed");
        ctrl.send_msg(0, 0xff, 0xbb)
            .expect("send_msg protected high failed");
        assert_eq!(ctrl.get_msg(0, 0x00).expect("read low failed"), 0x00);
        assert_eq!(ctrl.get_msg(0, 0xff).expect("read high failed"), 0x00);
    }

    #[test]
    fn port_fifo_transaction_state_is_isolated() {
        let mut host_buffer = [0u8; SWMBX_NODE_COUNT];
        // SAFETY: `buffer_base` is live, uniquely-owned host storage for the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        let addr = 0x0d;
        ctrl.enable_behavior(SWMBX_FIFO, true)
            .expect("enable fifo failed");
        ctrl.update_fifo(0, addr, 4, 0, true)
            .expect("fifo setup failed");

        ctrl.send_start(0, addr).expect("send_start port0 failed");
        ctrl.send_start(1, addr).expect("send_start port1 failed");

        assert!(ctrl.mbx_fifo_execute[0]);
        assert!(ctrl.mbx_fifo_execute[1]);
        assert_eq!(ctrl.mbx_fifo_addr[0], addr);
        assert_eq!(ctrl.mbx_fifo_addr[1], addr);

        ctrl.send_stop(1).expect("send_stop port1 failed");
        assert!(ctrl.mbx_fifo_execute[0]);
        assert!(!ctrl.mbx_fifo_execute[1]);

        ctrl.send_stop(0).expect("send_stop port0 failed");
        assert!(!ctrl.mbx_fifo_execute[0]);
        assert!(!ctrl.mbx_fifo_execute[1]);
    }

    #[test]
    fn direct_helpers_validate_mapping_and_buffer_bounds() {
        let mut host_buffer = [0u8; 16];
        // SAFETY: `buffer_base` is live, uniquely-owned host storage for the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        assert_eq!(ctrl.swmbx_read(true, 0x44), Err(SwmbxError::FifoNotMapped));
        assert_eq!(
            ctrl.swmbx_write(true, 0x44, 0x11),
            Err(SwmbxError::FifoNotMapped)
        );

        assert_eq!(ctrl.swmbx_read(false, 16), Err(SwmbxError::InvalidAddress));
        assert_eq!(
            ctrl.swmbx_write(false, 16, 0x55),
            Err(SwmbxError::InvalidAddress)
        );
    }

    #[test]
    fn invalid_port_and_flag_paths_return_errors() {
        let mut host_buffer = [0u8; SWMBX_NODE_COUNT];
        // SAFETY: `buffer_base` is live, uniquely-owned host storage for the test.
        let mut ctrl = unsafe {
            SwmbxCtrl::new_with_regions(host_buffer.len(), host_buffer.as_mut_ptr() as usize)
        };

        let bad_port = SWMBX_DEV_COUNT;

        assert_eq!(
            ctrl.update_notify(bad_port, 0x01, true),
            Err(SwmbxError::InvalidPort)
        );
        assert_eq!(
            ctrl.update_protect(bad_port, 0x01, true),
            Err(SwmbxError::InvalidPort)
        );
        assert_eq!(
            ctrl.apply_protect(bad_port, &[0], 0),
            Err(SwmbxError::InvalidPort)
        );
        assert_eq!(ctrl.get_msg(bad_port, 0x01), Err(SwmbxError::InvalidPort));
        assert_eq!(
            ctrl.send_msg(bad_port, 0x01, 0x11),
            Err(SwmbxError::InvalidPort)
        );
        assert_eq!(
            ctrl.send_start(bad_port, 0x01),
            Err(SwmbxError::InvalidPort)
        );
        assert_eq!(ctrl.send_stop(bad_port), Err(SwmbxError::InvalidPort));

        assert_eq!(
            ctrl.enable_behavior(0, true),
            Err(SwmbxError::InvalidFlagMask)
        );
        assert_eq!(
            ctrl.flush_fifo(SWMBX_FIFO_COUNT),
            Err(SwmbxError::InvalidFifoIndex)
        );
    }
}

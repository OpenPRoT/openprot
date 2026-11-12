// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C target-mode server facade.
//!
//! A single, narrow trait — [`I3cTarget`](crate::i3c_hardware::I3cTarget) — that each
//! controller implements **directly on top of its own register interface**
//! (ast10x0, caliptra-mcu, …). It is a *deep module*: the FIFO mechanics, ISR
//! bodies, dynamic-address assignment, and IBI sequencing all live inside each
//! controller's impl; the surface exposed here is only what the pigweed
//! server-runtime needs to hook interrupts and service IPC.
//!
//! This is deliberately **not** a portable capability HAL. There is one facade,
//! not a family of segregated traits, and it is shaped by the server-runtime's
//! needs rather than by an abstract notion of portability. Portability across
//! controllers is a consequence of having more than one implementor, not a goal
//! that widens the surface.
//!
//! ```text
//! consumer (e.g. MCTP)          out of scope here
//!        │  IPC (send / recv / dynamic_address)
//! i3c server-runtime            hooks pigweed IRQ + IPC to the facade
//!        │  I3cTarget
//! per-controller impl           deep: registers, FIFOs, ISR, DAA, IBI
//! ```
//!
//! Scope is target mode only; controller/primary-mode operations are out of
//! scope.

/// A 7-bit I3C dynamic address assigned by the active controller.
///
/// Constructed only through validation, so an out-of-range or reserved value
/// can never reach the facade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicAddress(u8);

/// Error returned when a value is not a valid 7-bit dynamic address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidAddress;

impl From<DynamicAddress> for u8 {
    /// The raw 7-bit value.
    fn from(addr: DynamicAddress) -> u8 {
        addr.0
    }
}

impl TryFrom<u8> for DynamicAddress {
    type Error = InvalidAddress;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        // 0x00 is reserved; 0x7E/0x7F are reserved; the MSB must be clear.
        match value {
            0x01..=0x7D => Ok(DynamicAddress(value)),
            _ => Err(InvalidAddress),
        }
    }
}

/// Static configuration applied before the kernel scheduler runs.
///
/// A controller consumes this at construction to bring the target up with a
/// known identity; nothing here changes on the hot path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I3cTargetConfig {
    /// Static address the device answers on before dynamic address assignment,
    /// if any.
    pub static_address: Option<DynamicAddress>,
    /// Mandatory Data Byte this target emits with its IBIs.
    pub ibi_mdb: u8,
}

/// What a target reports about itself: its addresses and its transfer limits.
///
/// Returned by [`I3cTarget::device_info`]. The limits can change while the
/// target runs (a controller may set them with the SETMRL and SETMWL CCCs), so
/// read them again whenever they matter instead of caching them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I3cDeviceInfo {
    /// Static address the device answers on before dynamic address assignment,
    /// if it has one.
    pub static_address: Option<DynamicAddress>,
    /// The dynamic address currently assigned, if any.
    pub dynamic_address: Option<DynamicAddress>,
    /// The most bytes this target can stage for the controller in one response.
    pub max_send_len: usize,
    /// The most bytes this target can receive from the controller in one write.
    pub max_recv_len: usize,
}

/// What a call to [`I3cTarget::on_interrupt`] observed on the bus.
///
/// The server-runtime maps each event to a reactor action (drain, wake a
/// waiter, record the assigned address). It is `#[non_exhaustive]` so a
/// controller can report finer events without breaking the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TargetEvent {
    /// Nothing actionable; the interrupt was handled internally.
    None,
    /// One or more inbound frames are ready; drain with
    /// [`I3cTarget::read_frame`].
    InboundReady,
    /// The controller assigned this target a dynamic address.
    ///
    /// Only a driver that learns of an assignment asynchronously reports this. A
    /// target whose address is assigned before the driver runs (the Caliptra ROM
    /// does it) never does; callers read [`I3cTarget::dynamic_address`] instead.
    AddressAssigned(DynamicAddress),
    /// The controller revoked this target's dynamic address (RSTDAA, or a bus
    /// reset). [`I3cTarget::dynamic_address`] returns `None` until a new one is
    /// assigned. Reported on the same terms as [`AddressAssigned`](Self::AddressAssigned).
    AddressLost,
    /// The controller completed the private read of a staged response.
    ResponseRead,
}

/// The server-side facade for an I3C device in target mode.
///
/// One trait, implemented directly on each controller's registers. The
/// interface is narrow on purpose — this is what the pigweed server-runtime
/// calls, and no more.
pub trait I3cTarget {
    /// Hardware error surfaced by target operations.
    type Error: core::fmt::Debug;

    /// Bring the target onto the bus using its pre-kernel configuration.
    fn enable(&mut self) -> Result<(), Self::Error>;

    /// Also report [`TargetEvent::ResponseRead`], when the controller reads a
    /// staged response out of the target's TX queue.
    ///
    /// Reporting that event can cost an extra interrupt per outbound message on
    /// some controllers, so [`enable`](Self::enable) does not turn it on. A
    /// runtime that allows one staged response at a time, and refuses the next
    /// send until the first has been read, needs it: `ResponseRead` is the only
    /// thing that releases the next send, so a runtime like that calls this after
    /// `enable`, and a target it runs on must report the event. The default does
    /// nothing, which is only right for a target that is never used that way.
    fn enable_response_read_events(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Take the target off the bus: stop reporting events and receiving
    /// frames. [`enable`](Self::enable) brings it back.
    fn disable(&mut self) -> Result<(), Self::Error>;

    /// Ask the controller to give this target a dynamic address by raising a
    /// Hot-Join request.
    ///
    /// Returns `Ok(true)` if a request was raised, and `Ok(false)` if this target
    /// does not support Hot-Join or already has a dynamic address. The default
    /// supports nothing.
    fn request_hot_join(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    /// Service a target interrupt.
    ///
    /// Called from the pigweed IRQ handler: the implementation drains hardware
    /// state and latches bus events, returning the event the runtime should
    /// act on.
    ///
    /// One call returns one event, but an interrupt can carry several. When it
    /// does, report first an event that cannot be reported again later, such as
    /// a clear-on-read status like a transmit completing, and leave for later one
    /// that will fire again by itself, such as a level-triggered "inbound frame
    /// pending". Otherwise the first kind is lost: the runtime acts on the event
    /// it is given and acknowledges the interrupt, and a status already cleared
    /// cannot be reported on the next call.
    fn on_interrupt(&mut self) -> Result<TargetEvent, Self::Error>;

    /// Drain the next buffered inbound frame into `buf`.
    ///
    /// Returns the number of bytes written, or `None` when nothing is pending.
    /// The runtime calls this after an [`TargetEvent::InboundReady`], draining
    /// until it returns `None`.
    fn read_frame(&mut self, buf: &mut [u8]) -> Result<Option<usize>, Self::Error>;

    /// Send a message to the controller.
    ///
    /// Stages `data` as the response to the controller's next private read and
    /// raises the IBI that prompts it — one fused outbound operation. The MDB
    /// is device configuration, computed internally, not a caller argument.
    /// Takes `&[u8]`: sending does not entitle the callee to mutate the
    /// caller's buffer.
    fn send(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    /// Discard the staged response, and any IBI still being raised for it, so
    /// the next [`send`](Self::send) starts from an empty transmit queue.
    ///
    /// The runtime calls this when a response has been staged for too long and
    /// the controller has not read it, so one controller that stops reading
    /// cannot block every later `send`. It must not drop inbound frames, and the
    /// target must not report [`TargetEvent::ResponseRead`] for the discarded
    /// response afterwards.
    ///
    /// This is best effort. A target, or a hardware model, that cannot discard a
    /// staged response may do nothing and return `Ok(())`: the controller can
    /// then still read the old response, and a later one queues behind it. The
    /// Caliptra emulator is such a model. It accepts the reset writes that the
    /// real controller acts on and ignores them.
    fn cancel_response(&mut self) -> Result<(), Self::Error>;

    /// The dynamic address currently assigned, or `None` if unassigned.
    ///
    /// A local state read: it cannot fail, so it returns `Option`, not
    /// `Result<Option<_>>`.
    fn dynamic_address(&self) -> Option<DynamicAddress>;

    /// The static address this target answers on before dynamic address
    /// assignment, or `None` if it has none.
    fn static_address(&self) -> Option<DynamicAddress>;

    /// The most bytes this target can stage for the controller in one response,
    /// that is, the size limit of a controller private read.
    ///
    /// The service refuses a `Send` larger than this with `TooLong` before it
    /// reaches [`send`](Self::send), so a driver is never asked to stage more than
    /// its hardware holds. The value may change while the target runs, so it is
    /// read for every `Send`.
    fn max_send_len(&self) -> usize;

    /// The most bytes this target can receive from the controller in one private
    /// write.
    ///
    /// It must not exceed the service's protocol ceiling (`i3c_api::MAX_PAYLOAD`,
    /// 256 bytes): the inbound ring slots are that size, so a larger frame would
    /// be truncated. The service reports the smaller of the two to its clients.
    /// A controller can have a smaller limit; the AST10x0 target path, for
    /// example, delivers at most 64 bytes per write.
    fn max_recv_len(&self) -> usize;

    /// This target's addresses and transfer limits in one value.
    fn device_info(&self) -> I3cDeviceInfo {
        I3cDeviceInfo {
            static_address: self.static_address(),
            dynamic_address: self.dynamic_address(),
            max_send_len: self.max_send_len(),
            max_recv_len: self.max_recv_len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_address_accepts_valid_range() {
        assert_eq!(DynamicAddress::try_from(0x01).map(u8::from), Ok(0x01));
        assert_eq!(DynamicAddress::try_from(0x42).map(u8::from), Ok(0x42));
        assert_eq!(DynamicAddress::try_from(0x7D).map(u8::from), Ok(0x7D));
    }

    #[test]
    fn dynamic_address_rejects_reserved() {
        assert_eq!(DynamicAddress::try_from(0x00), Err(InvalidAddress));
        assert_eq!(DynamicAddress::try_from(0x7E), Err(InvalidAddress));
        assert_eq!(DynamicAddress::try_from(0x7F), Err(InvalidAddress));
        assert_eq!(DynamicAddress::try_from(0x80), Err(InvalidAddress));
        assert_eq!(DynamicAddress::try_from(0xFF), Err(InvalidAddress));
    }

    /// A minimal in-memory target that exercises the whole facade, standing in
    /// for a real per-controller register implementation in host tests.
    #[derive(Default)]
    struct FakeTarget {
        enabled: bool,
        addr: Option<u8>,
        static_addr: Option<u8>,
        inbound: Option<usize>,
        sent: bool,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct FakeError;

    impl I3cTarget for FakeTarget {
        type Error = FakeError;

        fn enable(&mut self) -> Result<(), Self::Error> {
            self.enabled = true;
            Ok(())
        }

        fn on_interrupt(&mut self) -> Result<TargetEvent, Self::Error> {
            if self.inbound.is_some() {
                Ok(TargetEvent::InboundReady)
            } else {
                Ok(TargetEvent::None)
            }
        }

        fn read_frame(&mut self, buf: &mut [u8]) -> Result<Option<usize>, Self::Error> {
            match self.inbound.take() {
                Some(n) if n <= buf.len() => Ok(Some(n)),
                Some(_) => Err(FakeError),
                None => Ok(None),
            }
        }

        fn send(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
            self.sent = true;
            Ok(())
        }

        fn cancel_response(&mut self) -> Result<(), Self::Error> {
            self.sent = false;
            Ok(())
        }

        fn dynamic_address(&self) -> Option<DynamicAddress> {
            self.addr.and_then(|a| DynamicAddress::try_from(a).ok())
        }

        fn static_address(&self) -> Option<DynamicAddress> {
            self.static_addr
                .and_then(|a| DynamicAddress::try_from(a).ok())
        }

        fn max_send_len(&self) -> usize {
            256
        }

        fn max_recv_len(&self) -> usize {
            // Different from the send limit, so a test can tell them apart.
            64
        }

        fn disable(&mut self) -> Result<(), Self::Error> {
            self.enabled = false;
            Ok(())
        }
    }

    #[test]
    fn facade_round_trip() {
        let mut t = FakeTarget {
            addr: Some(0x42),
            inbound: Some(4),
            ..Default::default()
        };

        assert_eq!(t.enable(), Ok(()));
        assert!(t.enabled);

        // An interrupt reports the pending inbound frame, then it drains once.
        assert_eq!(t.on_interrupt(), Ok(TargetEvent::InboundReady));
        let mut buf = [0u8; 8];
        assert_eq!(t.read_frame(&mut buf), Ok(Some(4)));
        assert_eq!(t.read_frame(&mut buf), Ok(None));
        assert_eq!(t.on_interrupt(), Ok(TargetEvent::None));

        assert_eq!(t.send(b"pong"), Ok(()));
        assert!(t.sent);

        assert_eq!(t.dynamic_address(), DynamicAddress::try_from(0x42).ok());
    }

    #[test]
    fn disable_takes_the_target_off_the_bus() {
        let mut t = FakeTarget::default();
        assert_eq!(t.enable(), Ok(()));
        assert!(t.enabled);
        assert_eq!(t.disable(), Ok(()));
        assert!(!t.enabled);
    }

    #[test]
    fn hot_join_is_unsupported_unless_a_target_provides_it() {
        // The default reports "not raised" rather than an error.
        let mut t = FakeTarget::default();
        assert_eq!(t.request_hot_join(), Ok(false));
    }

    #[test]
    fn device_info_assembles_the_getters() {
        let t = FakeTarget {
            addr: Some(0x42),
            static_addr: Some(0x30),
            ..Default::default()
        };
        assert_eq!(
            t.device_info(),
            I3cDeviceInfo {
                static_address: DynamicAddress::try_from(0x30).ok(),
                dynamic_address: DynamicAddress::try_from(0x42).ok(),
                max_send_len: 256,
                max_recv_len: 64,
            }
        );

        // An unassigned, address-less target reports `None` for both.
        let t = FakeTarget::default();
        let info = t.device_info();
        assert_eq!((info.static_address, info.dynamic_address), (None, None));
    }
}

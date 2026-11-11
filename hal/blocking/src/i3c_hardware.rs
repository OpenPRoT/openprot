// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! I3C target-mode server facade.
//!
//! A single, narrow trait — [`I3cTarget`] — that each controller implements
//! **directly on top of its own register interface** (ast10x0, caliptra-mcu,
//! …). It is a *deep module*: the FIFO mechanics, ISR bodies, dynamic-address
//! assignment, and IBI sequencing all live inside each controller's impl; the
//! surface exposed here is only what the pigweed server-runtime needs to hook
//! interrupts and service IPC.
//!
//! This is deliberately **not** a portable capability HAL. There is one facade,
//! not a family of segregated traits, and it is shaped by the server-runtime's
//! needs rather than by an abstract notion of portability. Portability across
//! controllers is a consequence of having more than one implementor, not a goal
//! that widens the surface.
//!
//! ```text
//! MCTP transport-i3c            consumer (out of scope here)
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

impl DynamicAddress {
    /// The raw 7-bit value.
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

/// Error returned when a value is not a valid 7-bit dynamic address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidAddress;

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
    AddressAssigned(DynamicAddress),
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
    /// some controllers, and only a runtime that holds a second send until the
    /// first has been read needs it, so [`enable`](Self::enable) does not turn it
    /// on. Such a runtime calls this after `enable`. The default does nothing:
    /// a target that never reports `ResponseRead` leaves its runtime to release
    /// outbound back-pressure on the next inbound frame instead.
    fn enable_response_read_events(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Service a target interrupt.
    ///
    /// Called from the pigweed IRQ handler: the implementation drains hardware
    /// state and latches bus events, returning the event the runtime should
    /// act on.
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

    /// The dynamic address currently assigned, or `None` if unassigned.
    ///
    /// A local state read: it cannot fail, so it returns `Option`, not
    /// `Result<Option<_>>`.
    fn dynamic_address(&self) -> Option<DynamicAddress>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_address_accepts_valid_range() {
        assert_eq!(DynamicAddress::try_from(0x01).map(|a| a.as_u8()), Ok(0x01));
        assert_eq!(DynamicAddress::try_from(0x42).map(|a| a.as_u8()), Ok(0x42));
        assert_eq!(DynamicAddress::try_from(0x7D).map(|a| a.as_u8()), Ok(0x7D));
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

        fn dynamic_address(&self) -> Option<DynamicAddress> {
            self.addr.and_then(|a| DynamicAddress::try_from(a).ok())
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
}

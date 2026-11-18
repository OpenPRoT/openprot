// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # DICE Protection Environment HAL Traits
//!
//! This module provides blocking Hardware Abstraction Layer (HAL) traits for
//! a client of a DICE Protection Environment (DPE). The TCG specification
//! "DICE Protection Environment" defines the DPE and its commands.
//!
//! ## Key Components
//!
//! - `Profile` - The types that a DPE profile defines
//! - `P384Sha384` - The profile that uses ECDSA P-384 and SHA-384
//! - `ContextHandle` - Names a context of the DPE
//! - `Sign` - Trait for the DPE `Sign` command
//!
//! ## Scope
//!
//! The module has a trait for the `Sign` command. The other DPE commands have
//! no trait.
//!
//! A driver implements a command trait once for each profile that its DPE
//! supports. The driver decides how a command reaches the DPE. The DPE can be
//! another processor behind a mailbox, or a library in the same firmware.
//!
//! A DPE holds contexts. A context is a chain of measurements, from which the
//! DPE derives keys. A context belongs to one caller, which the DPE calls a
//! locality. The DPE learns the locality of a command from the way the command
//! arrives, so the traits have no locality parameter.
//!
//! ## Example Usage
//!
//! ```rust
//! use openprot_hal_blocking::digest::Digest;
//! use openprot_hal_blocking::dpe::{ContextHandle, Error, ErrorKind, P384Sha384, Sign};
//! use openprot_hal_blocking::ecdsa::P384Signature;
//!
//! const LABEL: [u8; 48] = [1; 48];
//!
//! fn sign_twice<D: Sign<P384Sha384>>(
//!     dpe: &mut D,
//!     digest: &Digest<12>,
//!     signatures: &mut [P384Signature; 2],
//! ) -> Result<(), ErrorKind> {
//!     let [first, second] = signatures;
//!     let handle = dpe
//!         .sign(&ContextHandle::DEFAULT, &LABEL, digest, first)
//!         .map_err(|error| error.kind())?;
//!     // The second command uses the handle that the first one returned.
//!     dpe.sign(&handle, &LABEL, digest, second)
//!         .map_err(|error| error.kind())?;
//!     Ok(())
//! }
//! # use openprot_hal_blocking::dpe::ErrorType;
//! # struct Fake;
//! # impl ErrorType for Fake { type Error = core::convert::Infallible; }
//! # impl Sign<P384Sha384> for Fake {
//! #     fn sign(
//! #         &mut self,
//! #         handle: &ContextHandle,
//! #         _label: &[u8; 48],
//! #         _digest: &Digest<12>,
//! #         signature: &mut P384Signature,
//! #     ) -> Result<ContextHandle, Self::Error> {
//! #         *signature = P384Signature::new([2; 48], [3; 48]);
//! #         Ok(*handle)
//! #     }
//! # }
//! # let empty = || P384Signature::new([0; 48], [0; 48]);
//! # let mut signatures = [empty(), empty()];
//! # assert!(sign_twice(&mut Fake, &Digest::new([0; 12]), &mut signatures).is_ok());
//! ```

use core::fmt;

use crate::digest::{DigestAlgorithm, Sha2_384};
use crate::ecdsa::P384Signature;

/// Error kind.
///
/// A common set of DPE errors. Generic code can act on these kinds without
/// knowing the error type of an implementation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The handle does not name a context of the DPE.
    InvalidHandle,

    /// The context belongs to another locality.
    InvalidLocality,

    /// The DPE rejected an argument of the command.
    InvalidArgument,

    /// The DPE does not support the command, the profile or an argument.
    NotSupported,

    /// The DPE is in use and cannot serve the command.
    Busy,

    /// The DPE did not respond in time.
    Timeout,

    /// The command did not reach the DPE, or its response could not be read.
    Transport,

    /// The response of the DPE is malformed.
    InvalidResponse,

    /// The DPE failed to execute the command.
    Internal,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHandle => write!(f, "invalid DPE context handle"),
            Self::InvalidLocality => write!(f, "DPE context belongs to another locality"),
            Self::InvalidArgument => write!(f, "invalid DPE command argument"),
            Self::NotSupported => write!(f, "DPE command not supported"),
            Self::Busy => write!(f, "DPE is busy"),
            Self::Timeout => write!(f, "DPE timed out"),
            Self::Transport => write!(f, "DPE transport failure"),
            Self::InvalidResponse => write!(f, "malformed DPE response"),
            Self::Internal => write!(f, "DPE internal error"),
        }
    }
}

/// Trait for DPE errors.
///
/// Implementations define their own error types and map them to an
/// [`ErrorKind`], on which generic code can act.
pub trait Error: fmt::Debug {
    /// Converts the error to a generic error kind.
    fn kind(&self) -> ErrorKind;
}

impl Error for core::convert::Infallible {
    fn kind(&self) -> ErrorKind {
        match *self {}
    }
}

/// Trait for types that have an associated error type.
pub trait ErrorType {
    /// Error type.
    type Error: Error;
}

impl<T: ErrorType + ?Sized> ErrorType for &mut T {
    type Error = T::Error;
}

/// The types that a DPE profile defines.
///
/// A profile selects the algorithms of a DPE, and with them the sizes of
/// labels, digests and signatures.
pub trait Profile {
    /// A label, which selects one of the keys of a context.
    type Label;

    /// The digest that [`Sign`] signs.
    type Digest;

    /// The signature that [`Sign`] returns.
    type Signature;
}

/// The DPE profile that uses ECDSA P-384 and SHA-384.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct P384Sha384;

impl Profile for P384Sha384 {
    type Label = [u8; 48];
    type Digest = <Sha2_384 as DigestAlgorithm>::Digest;
    type Signature = P384Signature;
}

/// Names a context of a DPE.
///
/// A command that takes a handle returns the handle to use for that context
/// in the next command. The DPE can return a new handle, and the old one then
/// no longer names the context.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ContextHandle(pub [u8; 16]);

impl ContextHandle {
    /// The handle of the default context of the caller's locality.
    pub const DEFAULT: Self = Self([0; 16]);
}

/// The DPE `Sign` command for profile `P`.
pub trait Sign<P: Profile>: ErrorType {
    /// Signs `digest` with a key of the context that `handle` names and
    /// stores the signature in `signature`.
    ///
    /// Returns the handle to use for the context in the next command.
    ///
    /// The DPE derives the key from the measurements of the context and from
    /// `label`, so the same context and label give the same key until a
    /// measurement changes.
    ///
    /// # Errors
    ///
    /// When the response is lost, the caller cannot tell whether the DPE
    /// replaced `handle`.
    ///
    /// After an error, the content of `signature` is unspecified.
    fn sign(
        &mut self,
        handle: &ContextHandle,
        label: &P::Label,
        digest: &P::Digest,
        signature: &mut P::Signature,
    ) -> Result<ContextHandle, Self::Error>;
}

impl<P: Profile, T: Sign<P> + ?Sized> Sign<P> for &mut T {
    fn sign(
        &mut self,
        handle: &ContextHandle,
        label: &P::Label,
        digest: &P::Digest,
        signature: &mut P::Signature,
    ) -> Result<ContextHandle, Self::Error> {
        T::sign(self, handle, label, digest, signature)
    }
}

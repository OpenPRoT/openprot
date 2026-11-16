// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # Random Number Generator HAL Traits
//!
//! This module provides blocking Hardware Abstraction Layer (HAL) traits for
//! cryptographically secure random number generators.
//!
//! ## Key Components
//!
//! - `CryptoRng` - Trait for filling a buffer with random bytes that are
//!   suitable for keys and nonces
//! - `RandCore` - Adapter that implements the `rand_core` traits for a
//!   `CryptoRng`
//!
//! ## Scope
//!
//! The HAL has no trait for generators that are not cryptographically
//! secure.
//!
//! The traits cover generating random bytes. A driver sets up its hardware
//! in its constructor: for example, it enables an entropy source or
//! instantiates a DRBG. Reseeding and mixing in additional input are not part
//! of these traits, because not every generator supports them.
//!
//! ## Example Usage
//!
//! ```rust
//! use openprot_hal_blocking::rng::{CryptoRng, Error, ErrorKind, ErrorType};
//!
//! fn nonce<R: CryptoRng>(rng: &mut R) -> Result<[u8; 32], ErrorKind> {
//!     let mut nonce = [0; 32];
//!     rng.fill(&mut nonce).map_err(|error| error.kind())?;
//!     Ok(nonce)
//! }
//! # struct Counter(u8);
//! # impl ErrorType for Counter { type Error = core::convert::Infallible; }
//! # impl CryptoRng for Counter {
//! #     fn fill(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
//! #         dest.iter_mut().for_each(|byte| { self.0 = self.0.wrapping_add(1); *byte = self.0; });
//! #         Ok(())
//! #     }
//! # }
//! # assert_eq!(nonce(&mut Counter(0)).unwrap()[..3], [1, 2, 3]);
//! ```

use core::fmt;

/// Error kind.
///
/// A common set of random number generator errors. Generic code can act on
/// these kinds without knowing the error type of an implementation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The generator has not been enabled, instantiated or seeded.
    NotInitialized,

    /// The generator is in use and cannot serve the request.
    Busy,

    /// The entropy source failed a health test, or the output does not have
    /// the required quality.
    EntropyFailure,

    /// The generator must be reseeded before it can produce more output.
    ReseedRequired,

    /// The generator did not produce output in time.
    Timeout,

    /// General hardware failure.
    HardwareFailure,

    /// Insufficient permissions to use the generator.
    PermissionDenied,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInitialized => write!(f, "random number generator not initialized"),
            Self::Busy => write!(f, "random number generator is busy"),
            Self::EntropyFailure => write!(f, "entropy source failure"),
            Self::ReseedRequired => write!(f, "random number generator must be reseeded"),
            Self::Timeout => write!(f, "random number generator timed out"),
            Self::HardwareFailure => write!(f, "random number generator hardware failure"),
            Self::PermissionDenied => write!(f, "insufficient permissions to access hardware"),
        }
    }
}

/// Trait for random number generator errors.
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

/// A random number generator whose output is suitable for keys, nonces and
/// other secrets.
///
/// The output of such a generator is unpredictable to an attacker who knows
/// the earlier output. An example is a NIST SP 800-90A DRBG that is seeded
/// from a health-tested entropy source. A driver for a raw entropy source
/// that has no conditioning or health tests must not implement this trait.
///
/// The requirement is the same as that of `rand_core::CryptoRng`.
pub trait CryptoRng: ErrorType {
    /// Fills `dest` with random bytes.
    ///
    /// `dest` can have any length, including zero. An implementation whose
    /// hardware produces a fixed or limited number of bytes at a time makes
    /// as many requests as it needs. The call blocks until the hardware has
    /// produced the bytes.
    ///
    /// # Errors
    ///
    /// After an error the contents of `dest` are unspecified and must not be
    /// used as random bytes.
    fn fill(&mut self, dest: &mut [u8]) -> Result<(), Self::Error>;
}

impl<T: CryptoRng + ?Sized> CryptoRng for &mut T {
    fn fill(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
        T::fill(self, dest)
    }
}

/// Adapter that implements `rand_core::TryRngCore` and
/// `rand_core::TryCryptoRng` for a [`CryptoRng`].
///
/// A [`CryptoRng`] can fail, so the adapter does not implement the
/// infallible `rand_core::RngCore`. Code that needs `RngCore` can call
/// `rand_core::TryRngCore::unwrap_err` on the adapter, which panics on an
/// error.
///
/// `RandCore::new(&mut rng)` wraps a borrowed generator.
///
/// Each `try_next_u32` or `try_next_u64` call makes one [`CryptoRng::fill`]
/// call, which is slow when each fill needs a hardware request.
#[derive(Debug)]
pub struct RandCore<R>(R);

impl<R: CryptoRng> RandCore<R> {
    /// Wraps `rng`.
    pub fn new(rng: R) -> Self {
        Self(rng)
    }

    /// Returns the wrapped generator.
    pub fn into_inner(self) -> R {
        self.0
    }
}

/// The error of a [`RandCore`]. It wraps the error of the generator.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct RandCoreError<E>(pub E);

impl<E: Error> fmt::Display for RandCoreError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.kind().fmt(f)
    }
}

impl<R: CryptoRng> rand_core::TryRngCore for RandCore<R> {
    type Error = RandCoreError<R::Error>;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        let mut bytes = [0; 4];
        self.try_fill_bytes(&mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        let mut bytes = [0; 8];
        self.try_fill_bytes(&mut bytes)?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        self.0.fill(dst).map_err(RandCoreError)
    }
}

impl<R: CryptoRng> rand_core::TryCryptoRng for RandCore<R> {}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::TryRngCore;

    /// Produces the bytes 1, 2, 3 and so on, and fails when `fail` is set.
    struct Counter {
        next: u8,
        fail: bool,
    }

    #[derive(Debug, PartialEq)]
    struct Failed;

    impl Error for Failed {
        fn kind(&self) -> ErrorKind {
            ErrorKind::EntropyFailure
        }
    }

    impl ErrorType for Counter {
        type Error = Failed;
    }

    impl CryptoRng for Counter {
        fn fill(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
            if self.fail {
                return Err(Failed);
            }
            for byte in dest {
                self.next = self.next.wrapping_add(1);
                *byte = self.next;
            }
            Ok(())
        }
    }

    fn counter() -> Counter {
        Counter {
            next: 0,
            fail: false,
        }
    }

    fn require_crypto<R: rand_core::TryCryptoRng>(_: &R) {}

    #[test]
    fn rand_core_adapter_reads_little_endian() {
        let mut rng = RandCore::new(counter());
        require_crypto(&rng);
        assert_eq!(rng.try_next_u32(), Ok(0x0403_0201));
        assert_eq!(rng.try_next_u64(), Ok(0x0c0b_0a09_0807_0605));
        let mut bytes = [0; 3];
        assert_eq!(rng.try_fill_bytes(&mut bytes), Ok(()));
        assert_eq!(bytes, [13, 14, 15]);
        assert_eq!(rng.into_inner().next, 15);
    }

    #[test]
    fn rand_core_adapter_borrows() {
        let mut inner = counter();
        let mut rng = RandCore::new(&mut inner);
        require_crypto(&rng);
        assert_eq!(rng.try_next_u32(), Ok(0x0403_0201));
        assert_eq!(inner.next, 4);
    }

    #[test]
    fn rand_core_adapter_returns_the_error() {
        let mut rng = RandCore::new(Counter {
            next: 0,
            fail: true,
        });
        let error = rng.try_next_u32().unwrap_err();
        assert_eq!(error, RandCoreError(Failed));
        assert_eq!(error.0.kind(), ErrorKind::EntropyFailure);
    }
}

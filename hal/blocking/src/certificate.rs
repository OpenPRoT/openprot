// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! # Certificate Chain Storage HAL Traits
//!
//! This module provides blocking Hardware Abstraction Layer (HAL) traits for
//! reading the certificate chains of a device.
//!
//! ## Key Components
//!
//! - `CertificateChain` - Trait for one certificate chain, read whole or in
//!   parts
//! - `CertificateStore` - Trait for the chains of a device, selected by slot
//!   and key algorithm
//! - `KeyAlgorithm` - The algorithm of the key that a chain certifies
//! - `Concat` - Two chains that are read as one
//!
//! ## Scope
//!
//! A certificate chain is a concatenation of DER-encoded X.509 certificates.
//! The last certificate is the leaf certificate. Each certificate after the
//! first is signed by the key of the certificate before it.
//!
//! A chain can be larger than the RAM of the device. An MLDSA-87 certificate
//! holds a 2592-byte public key and a 4627-byte signature, so a chain of four
//! is about 30 KB. The traits therefore do not require a whole chain to be
//! in memory. The caller reads the chain into a buffer of its own and chooses
//! the size of that buffer. A buffer as large as the chain takes the whole
//! chain in one read, which suits a short chain such as an ECDSA chain. A
//! smaller buffer takes a part of the chain.
//!
//! An implementation decides where the bytes of a chain come from. It can
//! read the chain from flash, from RAM, from one-time-programmable memory, or
//! from another processor. It can also build a certificate from parts that
//! are kept in different places, for example a public key from a key vault
//! and a signature from one-time-programmable memory. `Concat` joins chains
//! that come from different sources.
//!
//! The traits only read chains.
//!
//! ## Example Usage
//!
//! ```rust
//! use openprot_hal_blocking::certificate::{CertificateChain, Concat};
//!
//! /// Returns the sum of the bytes of `chain`, read 16 bytes at a time.
//! fn sum<C: CertificateChain>(chain: &mut C) -> Result<u32, C::Error> {
//!     let mut buf = [0u8; 16];
//!     let mut offset = 0;
//!     let mut sum = 0u32;
//!     loop {
//!         let stored = chain.read(offset, &mut buf)?;
//!         for byte in buf.iter().take(stored) {
//!             sum = sum.wrapping_add(u32::from(*byte));
//!         }
//!         offset += stored;
//!         // A read that does not fill the buffer has reached the end.
//!         if stored < buf.len() {
//!             return Ok(sum);
//!         }
//!     }
//! }
//!
//! // Three chains that are read as one.
//! let endorsement: &[u8] = &[1; 40];
//! let device: &[u8] = &[2; 30];
//! let leaf: &[u8] = &[3; 10];
//! let mut chain = Concat::new(endorsement, Concat::new(device, leaf));
//! assert_eq!(chain.len().ok(), Some(80));
//! assert_eq!(sum(&mut chain).ok(), Some(130));
//! ```

use core::convert::Infallible;
use core::fmt;

/// Error kind.
///
/// A common set of errors of certificate storage. Generic code can act on
/// these kinds without knowing the error type of an implementation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The store has no slot with this number.
    InvalidSlot,

    /// The store does not support the key algorithm.
    NotSupported,

    /// The slot has no chain for the key algorithm.
    NotProvisioned,

    /// The storage is in use and cannot be read.
    Busy,

    /// The storage could not be read.
    ReadFailed,

    /// The stored data is not a certificate chain.
    InvalidData,

    /// An error that the other kinds do not describe.
    Other,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSlot => write!(f, "invalid certificate slot"),
            Self::NotSupported => write!(f, "key algorithm not supported"),
            Self::NotProvisioned => write!(f, "certificate slot not provisioned"),
            Self::Busy => write!(f, "certificate storage is busy"),
            Self::ReadFailed => write!(f, "certificate storage read failed"),
            Self::InvalidData => write!(f, "invalid certificate chain data"),
            Self::Other => write!(f, "certificate storage error"),
        }
    }
}

/// Trait for certificate storage errors.
///
/// Implementations define their own error types and map them to an
/// [`ErrorKind`], on which generic code can act.
pub trait Error: fmt::Debug {
    /// Converts the error to a generic error kind.
    fn kind(&self) -> ErrorKind;
}

impl Error for Infallible {
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

/// The algorithm of the key that a certificate chain certifies, which is the
/// key of its leaf certificate.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum KeyAlgorithm {
    /// ECDSA with the NIST P-384 curve.
    EcdsaP384,

    /// MLDSA-87, which FIPS 204 defines.
    Mldsa87,
}

/// One certificate chain, which the caller reads whole or in parts.
pub trait CertificateChain: ErrorType {
    /// Returns the length of the chain in bytes.
    ///
    /// A caller can call this method often: [`Concat`] calls it in every
    /// read. An implementation that has to read the whole chain to find the
    /// length can keep the length for later calls.
    fn len(&mut self) -> Result<usize, Self::Error>;

    /// Returns whether the chain has no bytes.
    fn is_empty(&mut self) -> Result<bool, Self::Error> {
        Ok(self.len()? == 0)
    }

    /// Copies the chain, from byte `offset` of the chain, into `buf`.
    ///
    /// Returns the number of bytes stored. It is less than `buf.len()` only
    /// when the chain ends before `buf` is full. It is 0 when `offset` is at
    /// or after the end of the chain. The content of `buf` after the stored
    /// bytes is unspecified, and so is all of `buf` after an error.
    fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, Self::Error>;
}

impl<T: CertificateChain + ?Sized> CertificateChain for &mut T {
    fn len(&mut self) -> Result<usize, Self::Error> {
        T::len(self)
    }

    fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, Self::Error> {
        T::read(self, offset, buf)
    }
}

impl ErrorType for &[u8] {
    type Error = Infallible;
}

/// A chain that is in memory.
impl CertificateChain for &[u8] {
    fn len(&mut self) -> Result<usize, Self::Error> {
        Ok(<[u8]>::len(self))
    }

    fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let rest = self.get(offset..).unwrap_or(&[]);
        buf.iter_mut().zip(rest).for_each(|(to, from)| *to = *from);
        Ok(rest.len().min(buf.len()))
    }
}

/// The certificate chains of a device.
///
/// A device can have several identities, for example one that its vendor
/// gave it and one that its owner gave it. A slot holds the chains of one
/// identity. It holds at most one chain for each key algorithm. SPDM calls
/// these certificate slots.
pub trait CertificateStore: ErrorType {
    /// Returns the number of slots. The slots have the numbers 0 to
    /// `slot_count() - 1`.
    fn slot_count(&self) -> u8;

    /// Returns the length in bytes of the chain that `slot` holds for
    /// `algorithm`.
    ///
    /// A caller can call this method often. An implementation that has to
    /// read the whole chain to find the length can keep the length for later
    /// calls.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::NotProvisioned`] when the slot has no chain for
    /// `algorithm`.
    fn chain_len(&mut self, slot: u8, algorithm: KeyAlgorithm) -> Result<usize, Self::Error>;

    /// Copies the chain that `slot` holds for `algorithm`, from byte `offset`
    /// of the chain, into `buf`.
    ///
    /// Returns the number of bytes stored, as [`CertificateChain::read`]
    /// does.
    ///
    /// # Errors
    ///
    /// [`ErrorKind::NotProvisioned`] when the slot has no chain for
    /// `algorithm`. After an error, the content of `buf` is unspecified.
    fn read_chain(
        &mut self,
        slot: u8,
        algorithm: KeyAlgorithm,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<usize, Self::Error>;
}

impl<T: CertificateStore + ?Sized> CertificateStore for &mut T {
    fn slot_count(&self) -> u8 {
        T::slot_count(self)
    }

    fn chain_len(&mut self, slot: u8, algorithm: KeyAlgorithm) -> Result<usize, Self::Error> {
        T::chain_len(self, slot, algorithm)
    }

    fn read_chain(
        &mut self,
        slot: u8,
        algorithm: KeyAlgorithm,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<usize, Self::Error> {
        T::read_chain(self, slot, algorithm, offset, buf)
    }
}

/// Errors from [`Concat`].
///
/// When a chain of the `Concat` is itself a `Concat`, its error is a
/// `ConcatError`, so the error of three or more joined chains is nested in
/// the same way as the chains. [`Error::kind`] returns the kind of the error
/// of the chain that failed, and [`ErrorKind::InvalidData`] for `Length`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ConcatError<A, B> {
    /// The first chain failed.
    First(A),

    /// The second chain failed.
    Second(B),

    /// The sum of the lengths of the chains does not fit in a `usize`, or
    /// a read of the first chain stored a number of bytes that does not
    /// agree with its length.
    Length,
}

impl<A: Error, B: Error> Error for ConcatError<A, B> {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::First(error) => error.kind(),
            Self::Second(error) => error.kind(),
            Self::Length => ErrorKind::InvalidData,
        }
    }
}

/// Two certificate chains that are read as one: the first chain, followed by
/// the second.
///
/// The chains can be kept in different places. For example, the first can be
/// the certificates that a vendor stored in flash, and the second the
/// certificates that the device generates for its own keys.
///
/// A `Concat` is a chain, so it can be one of the two chains of another
/// `Concat`. `Concat::new(a, Concat::new(b, c))` reads `a`, `b` and `c` as
/// one chain, and more chains are joined in the same way.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Concat<A, B> {
    first: A,
    second: B,
}

impl<A, B> Concat<A, B> {
    /// Joins `first` and `second`.
    pub fn new(first: A, second: B) -> Self {
        Self { first, second }
    }
}

impl<A: CertificateChain, B: CertificateChain> ErrorType for Concat<A, B> {
    type Error = ConcatError<A::Error, B::Error>;
}

impl<A: CertificateChain, B: CertificateChain> CertificateChain for Concat<A, B> {
    fn len(&mut self) -> Result<usize, Self::Error> {
        let first = self.first.len().map_err(ConcatError::First)?;
        let second = self.second.len().map_err(ConcatError::Second)?;
        first.checked_add(second).ok_or(ConcatError::Length)
    }

    fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let first_len = self.first.len().map_err(ConcatError::First)?;
        let stored = if offset < first_len {
            self.first.read(offset, buf).map_err(ConcatError::First)?
        } else {
            0
        };
        let rest = buf.get_mut(stored..).ok_or(ConcatError::Length)?;
        if rest.is_empty() {
            return Ok(stored);
        }
        // `buf` is not full, so the first chain has ended at `offset +
        // stored`.
        let second_offset = offset
            .checked_add(stored)
            .and_then(|end| end.checked_sub(first_len))
            .ok_or(ConcatError::Length)?;
        let second = self
            .second
            .read(second_offset, rest)
            .map_err(ConcatError::Second)?;
        stored.checked_add(second).ok_or(ConcatError::Length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads `chain` from `offset` into a buffer of `N` bytes that is filled
    /// with `0xee`, and returns the buffer and the number of bytes stored.
    fn read<C: CertificateChain, const N: usize>(chain: &mut C, offset: usize) -> ([u8; N], usize) {
        let mut buf = [0xee; N];
        let stored = chain.read(offset, &mut buf).ok();
        assert!(stored.is_some());
        (buf, stored.unwrap_or(0))
    }

    #[test]
    fn slice_reads_parts() {
        let mut chain: &[u8] = &[1, 2, 3, 4, 5];
        assert_eq!(CertificateChain::len(&mut chain), Ok(5));
        assert_eq!(CertificateChain::is_empty(&mut chain), Ok(false));
        let mut empty: &[u8] = &[];
        assert_eq!(CertificateChain::is_empty(&mut empty), Ok(true));
        assert_eq!(read::<_, 2>(&mut chain, 0), ([1, 2], 2));
        assert_eq!(read::<_, 2>(&mut chain, 3), ([4, 5], 2));
        assert_eq!(read::<_, 2>(&mut chain, 4), ([5, 0xee], 1));
        assert_eq!(read::<_, 2>(&mut chain, 5), ([0xee, 0xee], 0));
        assert_eq!(read::<_, 2>(&mut chain, usize::MAX), ([0xee, 0xee], 0));
        assert_eq!(read::<_, 0>(&mut chain, 0), ([], 0));
    }

    #[test]
    fn concat_reads_across_the_join() {
        let first: &[u8] = &[1, 2, 3];
        let second: &[u8] = &[4, 5];
        let mut chain = Concat::new(first, second);
        assert_eq!(chain.len(), Ok(5));
        assert_eq!(read::<_, 5>(&mut chain, 0), ([1, 2, 3, 4, 5], 5));
        assert_eq!(read::<_, 2>(&mut chain, 0), ([1, 2], 2));
        assert_eq!(read::<_, 2>(&mut chain, 2), ([3, 4], 2));
        assert_eq!(read::<_, 2>(&mut chain, 3), ([4, 5], 2));
        assert_eq!(read::<_, 3>(&mut chain, 4), ([5, 0xee, 0xee], 1));
        assert_eq!(read::<_, 3>(&mut chain, 5), ([0xee; 3], 0));
        assert_eq!(read::<_, 3>(&mut chain, usize::MAX), ([0xee; 3], 0));
        assert_eq!(read::<_, 8>(&mut chain, 1).1, 4);
    }

    #[test]
    fn concat_nests_and_takes_empty_chains() {
        let empty: &[u8] = &[];
        let first: &[u8] = &[1];
        let second: &[u8] = &[2, 3];
        let third: &[u8] = &[4];
        let mut chain = Concat::new(Concat::new(empty, first), Concat::new(second, third));
        assert_eq!(chain.len(), Ok(4));
        assert_eq!(read::<_, 4>(&mut chain, 0), ([1, 2, 3, 4], 4));
        assert_eq!(read::<_, 2>(&mut chain, 2), ([3, 4], 2));
        // Reads a chain through a `&mut` reference to it.
        let mut by_ref = &mut chain;
        assert_eq!(read::<_, 3>(&mut by_ref, 1), ([2, 3, 4], 3));
    }

    /// A chain of `len` bytes, each 7. Its `read` stores at most one byte,
    /// which the trait does not allow. It fails when `fail` is set.
    struct OneByte {
        len: usize,
        fail: bool,
    }

    #[derive(Debug, PartialEq)]
    struct Failed;

    impl Error for Failed {
        fn kind(&self) -> ErrorKind {
            ErrorKind::ReadFailed
        }
    }

    impl ErrorType for OneByte {
        type Error = Failed;
    }

    impl CertificateChain for OneByte {
        fn len(&mut self) -> Result<usize, Failed> {
            if self.fail {
                return Err(Failed);
            }
            Ok(self.len)
        }

        fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, Failed> {
            match buf.first_mut() {
                Some(first) if offset < self.len => {
                    *first = 7;
                    Ok(1)
                }
                _ => Ok(0),
            }
        }
    }

    #[test]
    fn concat_reports_errors() {
        let slice: &[u8] = &[1, 2];
        let mut buf = [0; 4];

        let mut failing = Concat::new(OneByte { len: 1, fail: true }, slice);
        let error = failing.read(0, &mut buf).err();
        assert_eq!(error, Some(ConcatError::First(Failed)));
        assert_eq!(error.map(|error| error.kind()), Some(ErrorKind::ReadFailed));
        assert_eq!(failing.len(), Err(ConcatError::First(Failed)));

        let mut failing = Concat::new(slice, OneByte { len: 1, fail: true });
        assert_eq!(failing.len(), Err(ConcatError::Second(Failed)));

        // The error of three joined chains is nested in the same way as the
        // chains.
        let mut nested = Concat::new(slice, Concat::new(OneByte { len: 1, fail: true }, slice));
        let error = nested.read(2, &mut buf).err();
        assert_eq!(error, Some(ConcatError::Second(ConcatError::First(Failed))));
        assert_eq!(error.map(|error| error.kind()), Some(ErrorKind::ReadFailed));

        // The first chain stores one byte of its three, so the join cannot
        // tell where the second chain starts.
        let mut short = Concat::new(
            OneByte {
                len: 3,
                fail: false,
            },
            slice,
        );
        let error = short.read(0, &mut buf).err();
        assert_eq!(error, Some(ConcatError::Length));
        assert_eq!(
            error.map(|error| error.kind()),
            Some(ErrorKind::InvalidData)
        );

        let mut long = Concat::new(
            OneByte {
                len: usize::MAX,
                fail: false,
            },
            slice,
        );
        assert_eq!(long.len(), Err(ConcatError::Length));
    }

    /// A store with one slot, which holds an ECDSA P-384 chain.
    struct OneSlot(&'static [u8]);

    #[derive(Debug, PartialEq)]
    struct StoreError(ErrorKind);

    impl Error for StoreError {
        fn kind(&self) -> ErrorKind {
            self.0
        }
    }

    impl ErrorType for OneSlot {
        type Error = StoreError;
    }

    impl OneSlot {
        fn chain(
            &mut self,
            slot: u8,
            algorithm: KeyAlgorithm,
        ) -> Result<&mut &'static [u8], StoreError> {
            match (slot, algorithm) {
                (0, KeyAlgorithm::EcdsaP384) => Ok(&mut self.0),
                (0, _) => Err(StoreError(ErrorKind::NotProvisioned)),
                _ => Err(StoreError(ErrorKind::InvalidSlot)),
            }
        }
    }

    impl CertificateStore for OneSlot {
        fn slot_count(&self) -> u8 {
            1
        }

        fn chain_len(&mut self, slot: u8, algorithm: KeyAlgorithm) -> Result<usize, StoreError> {
            let chain = self.chain(slot, algorithm)?;
            Ok(<[u8]>::len(chain))
        }

        fn read_chain(
            &mut self,
            slot: u8,
            algorithm: KeyAlgorithm,
            offset: usize,
            buf: &mut [u8],
        ) -> Result<usize, StoreError> {
            let chain = self.chain(slot, algorithm)?;
            chain
                .read(offset, buf)
                .map_err(|_| StoreError(ErrorKind::Other))
        }
    }

    #[test]
    fn store_selects_a_chain() {
        let mut store = OneSlot(&[1, 2, 3]);
        // Reads a store through a `&mut` reference to it.
        let store = &mut store;
        let mut buf = [0; 2];
        assert_eq!(store.slot_count(), 1);
        assert_eq!(store.chain_len(0, KeyAlgorithm::EcdsaP384), Ok(3));
        assert_eq!(
            store.read_chain(0, KeyAlgorithm::EcdsaP384, 1, &mut buf),
            Ok(2)
        );
        assert_eq!(buf, [2, 3]);
        assert_eq!(
            store.chain_len(0, KeyAlgorithm::Mldsa87),
            Err(StoreError(ErrorKind::NotProvisioned))
        );
        assert_eq!(
            store.read_chain(1, KeyAlgorithm::EcdsaP384, 0, &mut buf),
            Err(StoreError(ErrorKind::InvalidSlot))
        );
    }
}

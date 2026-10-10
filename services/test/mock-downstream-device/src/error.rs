// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! What a poll reports when a line fails.

/// Why a poll could not be completed. Carries the pin error so a caller
/// sees which line failed, not just that one did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<R, Y> {
    /// The reset line could not be read.
    Reset(R),
    /// The ready line could not be driven.
    Ready(Y),
}

impl<R: core::fmt::Display, Y: core::fmt::Display> core::fmt::Display for Error<R, Y> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Reset(e) => write!(f, "reading the reset line: {e}"),
            Self::Ready(e) => write!(f, "driving the ready line: {e}"),
        }
    }
}

impl<R, Y> core::error::Error for Error<R, Y>
where
    R: core::fmt::Debug + core::fmt::Display,
    Y: core::fmt::Debug + core::fmt::Display,
{
}

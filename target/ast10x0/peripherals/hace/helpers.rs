// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Shared helper routines for HACE digest operations.

use super::context::HashContext;
use super::error::HaceError;

pub(crate) fn ptr_to_u32<T>(ptr: *const T) -> Result<u32, HaceError> {
    u32::try_from(ptr as usize).map_err(|_| HaceError::InvalidInput)
}

/// Append SHA padding to `ctx.buffer` starting at `ctx.bufcnt`.
///
/// Mirrors `aspeed-rust` `fill_padding`: uses `ctx.bufcnt` as the write
/// position and `ctx.digcnt[0]` (plus carry in `digcnt[1]`) as the total
/// byte count. `remaining` is the number of bytes not yet reflected in
/// `ctx.digcnt` (pass 0 when `digcnt` is fully up to date, as in `finalize`).
pub(crate) fn fill_padding(ctx: &mut HashContext, remaining: usize) {
    let block_size = ctx.block_size as usize;
    let bufcnt = ctx.bufcnt as usize;

    let index = (bufcnt + remaining) & (block_size - 1);
    let padlen = if block_size == 64 {
        if index < 56 {
            56 - index
        } else {
            64 + 56 - index
        }
    } else if index < 112 {
        112 - index
    } else {
        128 + 112 - index
    };

    ctx.buffer[bufcnt] = 0x80;
    ctx.buffer[bufcnt + 1..bufcnt + padlen].fill(0);

    if block_size == 64 {
        let bits = (ctx.digcnt[0] << 3).to_be_bytes();
        ctx.buffer[bufcnt + padlen..bufcnt + padlen + 8].copy_from_slice(&bits);
        ctx.bufcnt += (padlen + 8) as u32;
    } else {
        let low = (ctx.digcnt[0] << 3).to_be_bytes();
        let high = ((ctx.digcnt[1] << 3) | (ctx.digcnt[0] >> 61)).to_be_bytes();
        ctx.buffer[bufcnt + padlen..bufcnt + padlen + 8].copy_from_slice(&high);
        ctx.buffer[bufcnt + padlen + 8..bufcnt + padlen + 16].copy_from_slice(&low);
        ctx.bufcnt += (padlen + 16) as u32;
    }
}

/// Load IV words into the digest buffer using native-endian byte order.
///
/// Mirrors `aspeed-rust` `copy_iv_to_digest`: reinterprets each `u32` IV word
/// as its native-endian byte sequence and writes it into `digest`.
pub(crate) fn load_iv(digest: &mut [u8], iv_words: &[u32]) -> Result<(), HaceError> {
    if digest.len() != iv_words.len() * 4 {
        return Err(HaceError::InvalidInput);
    }

    for (i, word) in iv_words.iter().enumerate() {
        let bytes = word.to_ne_bytes();
        let off = i * 4;
        digest[off..off + 4].copy_from_slice(&bytes);
    }

    Ok(())
}

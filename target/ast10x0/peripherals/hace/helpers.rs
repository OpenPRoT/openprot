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

    ctx.buffer
        .get_mut(bufcnt)
        .map(|b| *b = 0x80)
        .unwrap_or(());
    ctx.buffer
        .get_mut(bufcnt + 1..bufcnt + padlen)
        .map(|s| s.fill(0))
        .unwrap_or(());

    if block_size == 64 {
        let bits = (ctx.digcnt[0] << 3).to_be_bytes();
        if let Some(dst) = ctx.buffer.get_mut(bufcnt + padlen..bufcnt + padlen + 8) {
            dst.copy_from_slice(&bits);
        }
        ctx.bufcnt += (padlen + 8) as u32;
    } else {
        let low = (ctx.digcnt[0] << 3).to_be_bytes();
        let high = ((ctx.digcnt[1] << 3) | (ctx.digcnt[0] >> 61)).to_be_bytes();
        if let Some(dst) = ctx.buffer.get_mut(bufcnt + padlen..bufcnt + padlen + 8) {
            dst.copy_from_slice(&high);
        }
        if let Some(dst) = ctx.buffer.get_mut(bufcnt + padlen + 8..bufcnt + padlen + 16) {
            dst.copy_from_slice(&low);
        }
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

    // Index-based copy (no `chunks_exact`): the `ChunksExact` iterator stores its
    // chunk size as a runtime field, so zipping it makes the optimizer emit a
    // `len / chunk_size` division it cannot prove is non-zero (a `div_by_zero`
    // panic path). Iterating word-by-word with a const stride and a length-proven
    // `&mut [u8; 4]` keeps the copy panic-free.
    for (i, word) in iv_words.iter().enumerate() {
        let off = i * 4;
        if let Some(dst) = digest.get_mut(off..off + 4) {
            if let Ok(dst) = <&mut [u8; 4]>::try_from(dst) {
                *dst = word.to_ne_bytes();
            }
        }
    }

    Ok(())
}

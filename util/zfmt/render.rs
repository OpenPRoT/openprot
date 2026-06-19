// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use zerocopy::FromBytes;
use zfmt::events::{DebugMessage, EventHeader, StreamStart};
use zfmt::leb128;
use zfmt::FixedBuf;
use zfmt::Write;

/// Renders a serialized `zfmt` event into a string and passes it to the callback.
///
/// `N` is the size of the temporary formatting buffer.
/// Returns the number of bytes consumed from `event` on success, or `None` on failure.
pub fn render_event<const N: usize>(event: &[u8], buf: &mut FixedBuf<N>) -> Option<usize> {
    let mut i = 0usize;
    let mut rest = event;
    loop {
        let (tag, mut next) = u32::read_from_prefix(rest).ok()?;
        i += 4;
        let (len, n) = leb128::decode(next)?;
        i += n;
        next = next.get(n..)?;
        let len = usize::try_from(len).ok()?;
        i += len;
        match tag {
            StreamStart::ZFMT_TAG => {
                let _ = buf.write_str("[StreamStart Event]");
                return Some(i);
            }
            EventHeader::ZFMT_TAG => {
                let eh = EventHeader::from_bytes(next.get(..len)?)?;
                let _ = eh.format_into(buf);
                let _ = buf.write_char(' ');
            }
            DebugMessage::ZFMT_TAG => {
                let (msg_len, n) = leb128::decode(next)?;
                let msg_len = usize::try_from(msg_len).ok()?;
                let end = n.checked_add(msg_len)?;
                let msg_bytes = next.get(n..end)?;
                // nosemgrep
                let msg = unsafe {
                    // SAFETY: the DebugMessage is guaranteed to contain a string.
                    core::str::from_utf8_unchecked(msg_bytes)
                };
                let _ = buf.write_str(msg);
                return Some(i);
            }
            _ => {
                // Unknown tag, silently consume.
                //pw_log::error!("Unknown tag {:08x} with {} bytes", tag, len);
            }
        }
        rest = next.get(len..)?;
    }
}

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
    let mut has_header = false;
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
                let _ = buf.write_str("[StreamStart Event]\r\n");
                return Some(i);
            }
            EventHeader::ZFMT_TAG => {
                let eh = EventHeader::from_bytes(next.get(..len)?)?;
                let _ = eh.format_into(buf);
                let _ = buf.write_char(' ');
                has_header = true;
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
                if has_header {
                    let _ = buf.write_str("\r\n");
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_bare_debug_message() {
        let msg = DebugMessage { message: "hwe> " };
        let mut frame = [0u8; 64];
        let mut n = 0;
        frame[n..n + 4].copy_from_slice(&msg.zfmt_tag().to_le_bytes());
        n += 4;
        let payload_size = msg.payload_size();
        n += leb128::encode(payload_size as u32, &mut frame[n..]);
        msg.serialize_into(&mut frame[n..]);
        n += payload_size;

        let mut buf = FixedBuf::<64>::new();
        let consumed = render_event(&frame[..n], &mut buf);
        assert_eq!(consumed, Some(n));
        assert_eq!(buf.as_str(), "hwe> ");
    }

    #[test]
    fn test_render_header_debug_message() {
        let hdr = EventHeader::new(
            zfmt::ZfmtU64::from_u64(100),
            zfmt::events::Severity::Debug,
            1,
        );
        let msg = DebugMessage {
            message: "test log",
        };
        let mut frame = [0u8; 128];
        let mut n = 0;
        frame[n..n + 4].copy_from_slice(&hdr.zfmt_tag().to_le_bytes());
        n += 4;
        let hdr_size = hdr.payload_size();
        n += leb128::encode(hdr_size as u32, &mut frame[n..]);
        hdr.serialize_into(&mut frame[n..]);
        n += hdr_size;

        frame[n..n + 4].copy_from_slice(&msg.zfmt_tag().to_le_bytes());
        n += 4;
        let msg_size = msg.payload_size();
        n += leb128::encode(msg_size as u32, &mut frame[n..]);
        msg.serialize_into(&mut frame[n..]);
        n += msg_size;

        let mut buf = FixedBuf::<128>::new();
        let consumed = render_event(&frame[..n], &mut buf);
        assert_eq!(consumed, Some(n));
        assert_eq!(buf.as_str(), "100 DEBUG test log\r\n");
    }
}

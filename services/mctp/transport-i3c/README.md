# openprot-mctp-transport-i3c

I3C transport binding for the MCTP server.

## Overview

This crate implements MCTP-over-I3C transport. It provides the `Sender`
implementation for outbound packets and a receiver/decoder for inbound frames,
driving the `services/i3c` target service over its IPC seam.

`mctp-estack` ships `i2c`, `serial` and `usb` bindings but no I3C one, so the
DSP0233 header handling lives here in `encap`. Fragmentation and reassembly
stay in the MCTP stack, which is transport-independent.

## Key Types

- `I3cSender<T, W>` — implements `mctp_lib::Sender`; handles fragmentation,
  framing and the PEC via `MctpI3cEncap`, and retries a frame while the i3c
  service is `Busy`, calling the wait hook `W` between attempts
- `MctpI3cReceiver` — verifies and strips the PEC of inbound I3C frames and
  returns the MCTP packets; caches this target's dynamic address for the PEC
- `MctpI3cEncap` — frames an outbound MCTP packet (packet, then PEC)

## Wire format

DSP0233: one MCTP packet is the data of one I3C private transfer, followed by a
PEC byte. There is **no** command code, byte count or source-address field in
front of the packet (those belong to the SMBus/I2C binding, DSP0237).

```text
[Target Address + R/W][ACK][ MCTP packet ... ][ PEC ]
                            ^ Write/Read Data 1
```

The PEC is an SMBus CRC-8 over the address byte of the transfer (`addr << 1 |
R/W`) followed by the packet. The address byte is on the bus in the address
phase and is not part of the data, but it is covered by the PEC:

| Direction | PEC covers |
|---|---|
| controller write to this target (inbound) | `(own_addr << 1) \| 0`, then the packet |
| controller read of this target (outbound) | `(own_addr << 1) \| 1`, then the packet |

## How it differs from the I2C binding

The two are deliberately shaped alike, but three things are forced by I3C:

- **This side is a target, not a bus master.** It cannot start a transfer, so
  sending stages a TX and raises an IBI for the controller to collect by
  private read — one `I3cOp::Send` carrying the whole framed packet. The I2C
  sender trims the leading destination byte because embedded-hal prepends it;
  nothing is trimmed here.
- **The address is runtime state.** I3C dynamic addresses are assigned by
  ENTDAA. The PEC covers the address byte, so a wrong address makes every frame
  fail its PEC. `I3cSender` reads the address back per fragment rather than
  taking it at construction (sending before enumeration is `Unreachable`), and
  names it in the `Send` (`send_for`). The service stages the frame only if the
  target still has that address, so ENTDAA cannot slip in between the read and
  the staging: it answers `AddressChanged`, and the sender rebuilds the frame and
  retries (`MAX_ADDRESS_RETRIES`). A frame already staged when the address moves
  is discarded, and the next `Send` reports it (`Discarded`), which fails the
  message. `MctpI3cReceiver` caches the address and re-reads it only when a PEC
  check fails.
- **The PEC is mandatory.** DSP0233 requires it, so every frame carries one and
  every inbound frame is checked; there is no switch to turn it off.
- **One response is staged at a time.** The service answers a second `Send`
  with `Busy` until the controller has read the first, so a multi-fragment
  message waits between fragments (`MAX_BUSY_RETRIES` attempts, then
  `TxFailure`). A controller that stops reading cannot block this for good: the
  service discards a response left unread for
  `i3c_api::STAGED_RESPONSE_TIMEOUT_MS` (1 s) and accepts `Send` again, losing
  that one message. The retries have to outlast it, so `MAX_BUSY_RETRIES` times
  the wait must be more than that timeout (the image checks this at build time).
  The wait runs in the MCTP server's thread, so it holds up every MCTP client
  for as long as it lasts.

## MTU

`get_mtu()` follows what the target can send. `I3cSender::refresh_limits` reads the
service's `DeviceInfo` and sets the MTU to one byte less than its `max_send_len` (the
frame is the packet plus the PEC), within two bounds:

- **At most `MCTP_I3C_IPC_MAXMTU` (254).** The service carries one framed packet per
  `I3cOp::Send`, at most `i3c_api::MAX_SEND` (255: the 256-byte TTI FIFO less the
  address byte that leads a `Send` payload). `mctp-estack`'s `Fragmenter` treats the
  MTU as the largest whole packet, header included, and the frame is the packet plus
  the PEC, so the packet is one byte shorter than the frame. This is caliptra-mcu-sw's
  rule (`max_read_len - 1`) for our frame.
- **At least `MCTP_I3C_MINMTU` (68): the MCTP header plus the 64-byte baseline
  payload of DSP0236.** A target that cannot send that much cannot carry MCTP:
  `refresh_limits` fails, and so does every send.

The controller can lower the limit (SETMRL), so every send reads it again. The
stack chooses a message's MTU *before* calling the sender, though, so call
`refresh_limits` once at startup, or the first message is built for the ceiling. A
packet that is over the limit read at the start of a send abandons that message
(`TxFailure`, nothing sent past it), and the next one is built for the new MTU. What
the target sends ran 40 of 40 on the VCK190 at 254 bytes.

Like caliptra-mcu-sw, this does not negotiate a size larger than the baseline with the
peer, which DSP0236 says should be known to be supported first.

### What a peer sends us is a different limit

Nothing here bounds what a *peer* writes to the target, and the hardware does not
refuse a large write:

- The core's RX data queue is 64 words (256 bytes), so a 255-byte frame fills it. A
  frame needs the queue empty when it starts.
- Reading it out costs about 180 microseconds a word on the FPGA (measured in the
  firmware), so a full frame takes about 12 ms.
- Writes reach the target closer together than a host spaces them: the FPGA model
  queues them and runs them with only a 5 ms pause. A second full frame that arrives
  during the read-out is cut short, and the core flags it with error 1 in the RX
  descriptor (`rx_byte_err_i` in `descriptor_rx.sv`: the queue was full while bytes
  were still arriving). Bytes past the room left are dropped silently.
- A 68-byte packet and its PEC are 18 words, about 3 ms to read out, and the queue
  holds three.

So a peer should send the baseline, a 64-byte payload, which is what DSP0236 has every
endpoint use unless it knows more, and what caliptra-mcu-sw's own host helper does. The
host tests write 68-byte packets for this reason. A controller that sends full-size
packets back to back can still overflow the queue, until the read-out is faster or the
spacing is guaranteed. The driver reports a damaged frame (`take_rx_error`) so that it
is visible.

## Testing

```bash
bazel test //services/mctp/transport-i3c/...
```

- `mctp_transport_i3c_test` — framing unit tests (PEC direction and address
  byte, rejection of bad PECs, buffer limits, MTU arithmetic) and the receiver's
  address cache
- `mctp_transport_i3c_roundtrip_test` — `Server::send()` through `I3cSender`
  and the real `i3c_server::dispatch`, checked as a controller would read it;
  no kernel, no emulator

## Dependencies

- `i3c_api`, `i3c_client` — the i3c target service seam and client
- `mctp-lib` — `Sender` trait, fragmentation
- `mctp` — core MCTP types
- `pw_log` — logging

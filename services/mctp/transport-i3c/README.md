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
  fail its PEC. `I3cSender` reads the address back per message rather than
  taking it at construction (sending before enumeration is `Unreachable`);
  `MctpI3cReceiver` caches it and re-reads it only when a PEC check fails.
- **The PEC is mandatory.** DSP0233 requires it, so every frame carries one and
  every inbound frame is checked; there is no switch to turn it off.
- **One response is staged at a time.** The service answers a second `Send`
  with `Busy` until the controller has read the first, so a multi-fragment
  message waits between fragments (`MAX_BUSY_RETRIES` attempts, then
  `TxFailure`).

## MTU

`get_mtu()` reports `MCTP_I3C_IPC_MAXMTU` (251). The i3c service carries one
framed packet per `I3cOp::Send`, capped at `i3c_api::MAX_PAYLOAD` (256 — the
TTI FIFO size). `mctp-estack`'s `Fragmenter` treats the MTU as the largest whole
packet, header included, so a frame (packet plus PEC) is never larger than
`MTU + 1`:

```text
packet <= MTU (251)   =>   frame <= 252 <= 256
```

251 leaves 4 bytes of the frame unused; `MAX_PAYLOAD - 1` (255) would use them.
The derivation (`MAX_PAYLOAD - header - PEC`) is the bench branch's, which ran at
245 when the service limit was 250; 251 has not been run on the board yet.

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

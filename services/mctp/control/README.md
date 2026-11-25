# openprot-mctp-control

MCTP control protocol responder policy for one endpoint.

## Overview

This crate answers four of the MCTP control commands an OpenPRoT endpoint
must answer. Get Vendor Defined Message Support is not answered yet, so an
endpoint built on this crate does not meet the OpenPRoT requirement; see
"Deferred commands". MCTP control is message type `0x00` (DSP0236 1.3.1,
Table 3).
The Bazel target `//services/mctp/control:mctp_control` builds the crate
`openprot_mctp_control`.

`ControlResponder` holds no transport: the caller owns the receive call, the
send call, and the loop that drives them. `ControlResponder::handle` takes the
received payload and writes the response bytes.

## Why no receive loop

Every MCTP receive path in this tree blocks the calling thread. An IPC
client's `recv` passes `Instant::MAX` to the kernel, so a component that
owns its own receive call needs a thread of its own, or blocks every other
deadline on the thread it shares. A responder with no loop of its own
leaves that choice to the assembling `main`, which can give the responder
its own thread or host it inside a loop that already exists.

## What the caller owns

The caller registers the listener for message type `0x00` once, then runs
these steps for each control message:

1. Receive one message on that listener.
2. Call `ControlResponder::handle` with the stack, the received payload,
   and a response buffer of at least `MAX_RESPONSE_LEN` bytes.
3. Send the returned number of bytes from that buffer with the request's
   own tag.

The endpoint ID lives in the MCTP stack, and `handle` reads and writes it
there, so no second copy can drift from it. Get Endpoint ID reports
`Stack::get_eid`, and an accepted Set Endpoint ID calls `Stack::set_eid`
before `handle` returns, so the reply leaves from the address the bus
owner has just given this endpoint. Where the stack refuses the write,
`handle` answers `ERROR` rather than claiming an assignment that did not
happen.

`handle` returns `MCTP_CONTROL_MALFORMED` for a request too short to
carry a control header, and `MCTP_CONTROL_RESPONSE_TOO_SHORT` for a
response buffer too short to hold the reply. Neither returns a reply
length, so the caller sends nothing.

A response (`rq=0`) and a datagram (`rq=1`, `d=1`) both return `Ok(None)`,
which is no error. DSP0236 Table 11 states that a datagram's responses are
"neither expected nor tracked by Instance ID at the requester", and a
broadcast datagram reaches every endpoint at once, so a reply would put one
message per endpoint on the bus for a request nobody awaits.

## Key types

- `ControlResponder`: answers one control request per `handle` call.
- `ControlConfig`: what one endpoint reports about itself.
- `MCTP_CONTROL`: the `util_error` module (`0x4d43`, ASCII `MC`) for every
  error this crate returns.

## Answered commands

- Set Endpoint ID (`0x01`)
- Get Endpoint ID (`0x02`)
- Get MCTP Version Support (`0x04`)
- Get Message Type Support (`0x05`)

Every other command code answers `ERROR_UNSUPPORTED_CMD`, which DSP0236
Table 13 defines for a command an endpoint does not support.

Set Endpoint ID accepts the Set EID and Force EID operations for a legal
EID: DSP0236 Table 14 makes `0x00` and `0xFF` illegal, and clause 8.2
reserves 1 through 7. It answers `ERROR_INVALID_DATA` for an illegal EID.
Set Discovered Flag answers `ERROR_INVALID_DATA` too, since this crate does
not track the discovered flag. DSP0236 Table 14 prescribes that answer for a
transport binding without a Discovered flag, and DSP0237 1.2.0 has none: an
SMBus/I2C endpoint is discovered through Get UDID (clause 6.5).
"Static and dynamic endpoints" describes Reset
EID. A body shorter or longer than its command and operation define answers
`ERROR_INVALID_LENGTH`, as a Set EID or Force EID body without its EID byte.

## Deferred commands

- Get Endpoint UUID (`0x03`): DSP0236 Table 12 marks this command
  conditional to accept.
- Get Vendor Defined Message Support (`0x06`): Table 12 marks this
  command optional to accept, and
  `docs/src/specification/middleware/mctp.md:31` lists it among
  OpenPRoT's own required commands, which is stricter than DSP0236.
  [#697](https://github.com/OpenPRoT/openprot/issues/697) tracks it.
- Transport Specific (`0xF0`-`0xFF`): DSP0236 Table 12 leaves the range to
  each transport binding. DSP0237 1.2.0 defines no command in it, so
  `ERROR_UNSUPPORTED_CMD` is the right answer on SMBus/I2C today.

## Configuration

`ControlConfig` asks the assembling `main` to supply the message types
this endpoint answers excluding `0x00` itself, the base-specification and
control-protocol version codes at 4 bytes per entry, the endpoint type,
and the transport-specific byte Get Endpoint ID returns. A static EID is no
part of it: the constructor takes that value, as "Static and dynamic
endpoints" describes.

Each message type is a `MsgTypeVersions` entry carrying the type number and
its own version codes. Get Message Type Support lists the type numbers, and
Get MCTP Version Support answers each listed type with its codes, so the two
replies cannot disagree. Listing a type commits the image to both answers.
DSP0236 Table 18 has a type with a binding specification report that
binding's version, as DSP0241 for PLDM or DSP0275 for SPDM. A type the list
leaves out answers `0x80`, "message type number not supported".

That `main` is the one place which knows the image's own message types and
endpoint role.

`ControlConfig::simple_endpoint` fills the rest of the configuration for
a simple endpoint, static or dynamic. Its version tables list MCTP 1.0,
1.1, 1.2, and 1.3.1. `docs/src/specification/middleware/mctp.md` names 1.3.1
as the minimum required version.

`simple_endpoint` also sets the medium-specific byte Get Endpoint ID returns
to `0`. On SMBus/I2C, DSP0237 1.2.0 Table 4 defines bit 0 of that byte as
fairness arbitration support, and clause 6.13.3 requires an endpoint to
report its support there. The bit stays clear because no I2C backend in this
tree implements fairness arbitration, so `0` reports the stack truthfully.
An image whose backend gains it sets bit 0 in its own `ControlConfig`.
`target/ast10x0/tests/mctp/control/README.md` ties this to that image's
I2C configuration.

## Version support

The tables stop at 1.3.1 because claiming DSP0236 1.4.0 takes more than a
table edit. 1.4.0 Table 12 makes Query Endpoint Advanced Capabilities
(`0x15`) mandatory to accept for a simple endpoint, and clause 13.7.3 reports
1.3 with any update (`0xF1F3FF00`) followed by 1.4.0 (`0xF1F4F000`).
`mctp-lib` has no `CommandCode` for `0x15`, so it decodes as an unknown
command and answers `ERROR_UNSUPPORTED_CMD`, which 1.3.1 permits. Moving to
1.4.0 also shifts the clause numbers this crate cites, as Set Endpoint ID
from 12.3 to 13.4.
[#698](https://github.com/OpenPRoT/openprot/issues/698) tracks the move.

## Tests

Run the host test with:

```bash
bazelisk test --test_output=errors \
  //services/mctp/control:mctp_control_responder_test
```

Run the doc test, which compiles the example in the crate doc, with:

```bash
bazelisk test --test_output=errors \
  //services/mctp/control:mctp_control_doc_test
```

## Dependencies

This crate is `no_std`. The `Stack` that `handle` takes comes from
`//services/mctp/api:mctp_api`, and the `ErrorCode` every fallible call
returns comes from `//util/error`.

## Static and dynamic endpoints

The constructor an image calls makes its endpoint static or dynamic.
`ControlResponder::try_new` builds a dynamic endpoint and writes nothing to
the stack, so the stack keeps the EID it already holds.
`ControlResponder::try_new_with_static_eid` builds one configured with a
static EID, and writes that EID to the stack as part of construction.

DSP0236 clause 12.3 makes a static endpoint's configured value its initial
default rather than something a bus owner supplies, so the stack has to
hold it before the first request arrives. Writing it in the constructor
means no static responder exists beside an unseeded stack.

`try_new_with_static_eid` returns an error for an EID no endpoint may hold,
and for a write the stack refuses. It checks before it writes, so a refused
configuration leaves the stack as it was, and the assembling `main`
decides whether to log and carry on with a dynamic responder or to stop.

Both constructors also refuse a `ControlConfig` that breaks one of these
rules:

- Every version table holds whole 4-byte codes, at most `MAX_VERSION_CODES`
  of them. A partial code would panic in `mctp-lib`'s encoders, and a longer
  table would produce a reply a `mctp-lib` requester cannot decode.
- Each listed type carries at least one code, appears once, and is a 7-bit
  number other than control's `0x00`.
- The list holds at most `MAX_MSG_TYPES`, 59 entries. DSP0236 clause 11.4
  carries a control message in one packet whose payload fits the 64-byte
  baseline transmission unit, and 59 types fill it.

A static endpoint differs in two answers:

- Get Endpoint ID reports EID type `10b` while the stack still holds the
  configured value and `11b` once a bus owner has moved it. Both values
  mean "static EID supported", since the field describes how the endpoint
  was configured rather than where its current EID came from.
- Reset EID restores the configured value. A dynamic endpoint answers
  `ERROR_INVALID_DATA`, since it has nothing to restore.

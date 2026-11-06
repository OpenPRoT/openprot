# openprot-mctp-control

MCTP control protocol responder policy for one endpoint.

## Overview

This crate answers four of the MCTP control requests an OpenPRoT endpoint
must answer. MCTP control is message type `0x00` (DSP0236 1.3.1, Table 3).
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
   and a response buffer of at least `MAX_RESPONSE_LEN` bytes, larger where
   `supported_msg_types` passes 32 entries.
3. Send the returned number of bytes from that buffer with the request's
   own tag.

The endpoint ID lives in the MCTP stack, and `handle` reads and writes it
there, so no second copy can drift from it. Get Endpoint ID reports
`Stack::get_eid`, and an accepted Set Endpoint ID calls `Stack::set_eid`
before `handle` returns, so the reply leaves from the address the bus
owner has just given this endpoint. Where the stack refuses the write,
`handle` answers `ERROR` rather than claiming an assignment that did not
happen.

`handle` returns `ControlError::Malformed` for a request too short to
carry a control header, `ControlError::NoReply` for a message that draws
no reply, and `ControlError::ResponseTooShort` for a response buffer too
short to hold the reply. No such case returns a reply length, so the
caller sends nothing.

A response (`rq=0`) and a datagram (`rq=1`, `d=1`) both answer
`NoReply`. DSP0236 Table 11 states that a datagram's responses are
"neither expected nor tracked by Instance ID at the requester", and a
broadcast datagram reaches every endpoint at once, so a reply would put
one message per endpoint on the bus for a request nobody awaits.

## Key types

- `ControlResponder`: answers one control request per `handle` call.
- `ControlConfig`: what one endpoint reports about itself.
- `ControlError`: the reason `handle` returned no reply length.

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
not track the discovered flag. "Static and dynamic endpoints" describes Reset
EID. A body shorter or longer than its command and operation define answers
`ERROR_INVALID_LENGTH`, as a Set EID or Force EID body without its EID byte.

## Deferred commands

- Get Endpoint UUID (`0x03`): DSP0236 Table 12 marks this command
  conditional to accept.
- Get Vendor Defined Message Support (`0x06`): Table 12 marks this
  command optional to accept, and
  `docs/src/specification/middleware/mctp.md:31` lists it among
  OpenPRoT's own required commands, which is stricter than DSP0236.
- Transport Specific (`0xF0`-`0xFF`): DSP0236 Table 12 leaves the range to
  each transport binding.

## Configuration

`ControlConfig` asks the assembling `main` to supply the message types
this endpoint answers excluding `0x00` itself, the base-specification and
control-protocol version codes at 4 bytes per entry, the endpoint type,
and the transport-specific byte Get Endpoint ID returns. A static EID is no
part of it: the constructor takes that value, as "Static and dynamic
endpoints" describes.

That `main` is the one place which knows the image's own message types and
endpoint role.

`ControlConfig::simple_endpoint` fills the rest of the configuration for
a simple endpoint, static or dynamic. Its version tables list MCTP 1.0,
1.1, 1.2, and 1.3.1. `docs/src/specification/middleware/mctp.md` names 1.3.1
as the minimum required version.

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
`//services/mctp/api:mctp_api`.

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

Both constructors also refuse a `ControlConfig` whose version tables are
not whole 4-byte codes or hold more than `MAX_VERSION_CODES`, or whose type
list exceeds 255 entries. A partial code or a type list past 255 entries
would panic in `mctp-lib`'s encoders. A table past `MAX_VERSION_CODES`
would produce a reply that a `mctp-lib` requester cannot decode.

A static endpoint differs in two answers:

- Get Endpoint ID reports EID type `10b` while the stack still holds the
  configured value and `11b` once a bus owner has moved it. Both values
  mean "static EID supported", since the field describes how the endpoint
  was configured rather than where its current EID came from.
- Reset EID restores the configured value. A dynamic endpoint answers
  `ERROR_INVALID_DATA`, since it has nothing to restore.

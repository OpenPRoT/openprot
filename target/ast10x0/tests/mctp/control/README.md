# MCTP control responder images

Two AST10x0 images run the `services/mctp/control` responder: one in QEMU
against a fake MCTP server, one on hardware against a BMC.

## QEMU image

`mctp_control_qemu_test` runs in CI. QEMU offers no I2C peer, so
`fake_server_main.rs` stands in for both the MCTP server and the bus owner
behind it: it answers the responder's IPC calls, hands it one control request
per receive, and checks the IPC header and every byte of each reply.

```sh
bazel test --config=virt_ast10x0 \
  //target/ast10x0/tests/mctp/control:mctp_control_qemu_test
```

The run passes on the UART sentinel `TEST_RESULT:PASS` and fails on
`TEST_RESULT:FAIL`. A failing case logs its number and what differed before the
sentinel.

### What the cases cover

Fourteen requests run in order, from the `CASES` array in `fake_server_main.rs`.

Case 2 is why this image exists. It asks Get Endpoint ID after the Set Endpoint
ID in case 1 and expects the newly assigned EID, which can only hold where the
assignment reached the MCTP stack. The crate's host tests drive a stack they
own, so none of them observes that write arriving over IPC.

## Hardware image

`mctp_control_test` exchanges real traffic with a BMC on the fixture. It
assembles the control responder beside copies of the I2C server and MCTP
server apps `tests/mctp/server` uses. The copies change only the bus number
and the I2C addresses.

```sh
bazel test --config=k_ast1060_evb \
  //target/ast10x0/tests/mctp/control:mctp_control_test
```

It is tagged `hardware` and `manual`, and it declares itself incompatible when
`qemu_enabled`, because without an I2C peer the responder receives nothing.

No app in this image calls `debug_shutdown`, because that call stops the
whole system and the endpoint would drop off the bus. The run therefore
prints no `TEST_RESULT` sentinel, and bazel reports it as timed out whatever
happened on the bus. Read the result from the UART log and from the BMC.

Besides the real I2C path, this image covers what no host test or QEMU case
reaches: a reply's source EID is filled in by the MCTP stack when it sends, not
by `handle`. Check the source EID of the Set Endpoint ID reply on the BMC side
to confirm it is the EID the BMC just assigned.

The AST2750 EVB fixture also needs the BMC released from reset before it can
send anything. This image does not do that, so a repeated run needs that
release from elsewhere, as an image that adds it beside these apps.

DSP0237 1.2.0 Table 9 requires a control response within 100 ms of its
request (MT1), measured at the responder from the end of the request to the
start of the response. No host, QEMU, or hardware step measures it. Two
methods check it:

- On the BMC, time each request from `sendto` to the matching `recvfrom`.
  That round trip holds MT1 plus both bus transfers and the BMC's own
  software delay, so a round trip under 100 ms bounds MT1, while a longer
  one leaves MT1 unknown.
- On the bus, put a logic analyzer on bus 0's SCL and SDA and time a
  request's STOP condition to the START of the reply the AST1060 writes.
  This measures MT1 itself.

## Endpoint identity

The hardware image's responder is static at EID 8. Get Endpoint ID answers EID
type `10b` while the endpoint holds 8 and `11b` once a bus owner moves it, and
Reset EID restores 8.

`control_main_qemu.rs` stays dynamic, because the fake server owns the stack's
EID state outright and starts it unassigned.

## Medium-specific byte

Get Endpoint ID reports `0x00` as its medium-specific byte, the value
`ControlConfig::simple_endpoint` sets. DSP0237 1.2.0 Table 4 defines bit 0 as
fairness arbitration support, so the clear bit says this endpoint does not
implement it.

That matches how `target.rs` configures bus 0. `I2C0_CFG` sets
`multi_master: false`, which sets the controller's multi-master disable bit,
and the I2C backend has no `FAIR_IDLE` detection or the fast START retry that
clause 6.13 requires. Enabling `multi_master` alone would not change the
answer: it turns on the controller's arbitration, which decides who wins a
collision, while fairness arbitration decides who goes next and needs backend
support this tree lacks.

## Message types

`control_main.rs` reports an empty `SUPPORTED_MSG_TYPES`, because this image
carries no upper-layer service. An image assembling PLDM or SPDM beside the
responder lists each type there with its binding versions.

`control_main_qemu.rs` lists PLDM with one version and SPDM with three anyway.
Case 5 checks that Get Message Type Support reports the configured list, cases
6 and 7 check that each listed type answers its own versions, and case 8
checks that an unlisted type answers `0x80`. An empty list would answer the
same whether the reply carried the list or dropped it.

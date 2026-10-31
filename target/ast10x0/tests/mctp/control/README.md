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

Eleven requests run in order, from the `CASES` array in `fake_server_main.rs`.

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

## Endpoint identity

The hardware image's responder is static at EID 8. Get Endpoint ID answers EID
type `10b` while the endpoint holds 8 and `11b` once a bus owner moves it, and
Reset EID restores 8.

`control_main_qemu.rs` stays dynamic, because the fake server owns the stack's
EID state outright and starts it unassigned.

## Message types

`control_main.rs` reports an empty `supported_msg_types`, because this image
carries no upper-layer service. An image assembling PLDM or SPDM beside the
responder names those types there.

`control_main_qemu.rs` names PLDM and SPDM anyway, so case 5 checks that Get
Message Type Support reports the configured list. An empty list would answer
the same whether the reply carried the list or dropped it.

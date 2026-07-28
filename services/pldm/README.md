# openprot-pldm-service

Platform-independent PLDM Firmware Device (FD) service, talking PLDM-over-MCTP
directly to a remote Update Agent (UA).

## Overview

This crate bridges [`openprot-mctp-api`](../mctp/api) and
[`pldm-interface`](https://github.com/OpenPRoT/pldm-lib/tree/main/pldm-interface)
so that firmware can run the PLDM firmware-update state machine and exchange
PLDM messages over MCTP without depending on any particular MCTP
implementation or OS.

`FirmwareDevice` owns two [`MctpPldmTransport`] instances and drives both
directions of traffic itself — there is no separate responder/requester
process or platform-specific IPC bridge:

```text
┌──────────────────────────┐
│   Application / Firmware │  constructs FirmwareDevice, calls run_terminus()
└───────────┬──────────────┘
            │
            ▼
┌──────────────────────────────────────────┐
│   openprot-pldm-service                  │◄── this crate
│   FirmwareDevice<'a, O: FdOps, Cr, Cq>    │
│     - cmd_interface: CmdInterface<'a, O>  │  PLDM FW-update state machine
│     - responder_transport: inbound UA→FD  │
│     - requester_transport: outbound FD→UA │
└───────────┬──────────────┬────────────────┘
            │              │ MctpPldmTransport<C: MctpClient>
            ▼              ▼
┌──────────────────────────────────────────┐
│   openprot-mctp-api                      │  Stack<C: MctpClient>
└───────────┬───────────────────────────────┘
            │ IPC / transport
            ▼
┌──────────────────────────┐
│   MCTP Server            │
└──────────────────────────┘
```

## Key types

| Type | Description |
|------|-------------|
| `FirmwareDevice<'a, O: FdOps, Cr: MctpClient, Cq: MctpClient>` | Owns the `CmdInterface` FW-update state machine plus the responder/requester `MctpPldmTransport`s; `run_terminus()` drives both to completion |
| `MctpPldmTransport<C: MctpClient>` | Wraps a `Stack<C>` and manages the MCTP PLDM framing byte (`0x01`) for sends/receives |
| `PldmServiceError` | Union of MCTP transport errors (`Mctp`), PLDM handler errors (`MsgHandler`), buffer/arithmetic overflow, and IPC errors |
| `PLDM_MSG_TYPE` (`pldm_common::util::mctp_transport::MCTP_PLDM_MSG_TYPE`) | MCTP message-type constant for PLDM (`0x01`) |

## Usage

```rust,ignore
use openprot_pldm_service::firmware_device::FirmwareDevice;
use openprot_pldm_service::MctpPldmTransport;
use pldm_interface::config::PLDM_PROTOCOL_CAPABILITIES;

// `fd_ops` implements `FdOps` (platform-specific flash / component logic).
// `responder_client` / `requester_client` are `MctpClient` implementations
// (they may share the same underlying MCTP endpoint).
let responder_transport = MctpPldmTransport::new(responder_client);
let requester_transport = MctpPldmTransport::new(requester_client);

let mut fd = FirmwareDevice::init(
    &fd_ops,
    &PLDM_PROTOCOL_CAPABILITIES,
    responder_transport,
    requester_transport,
);

const UA_EID: u8 = 8;
let mut buf = [0u8; 1024];

// `run_terminus` loops forever, interleaving inbound UA commands with any
// FD-initiated requests (e.g. RequestFirmwareData) once an update begins.
// It returns only on error; a `timeout_millis`/`requester_timeout_millis`
// of `0` blocks indefinitely while idle.
if let Err(e) = fd.run_terminus(UA_EID, &mut buf, 0, 0) {
    // handle or log error
}
```

## Buffer layout

All transport methods (`MctpPldmTransport::send_request`,
`recv_and_respond`, `respond_once`) and `FirmwareDevice::run_terminus` use the
same flat-buffer convention:

```text
buf[0]   : MCTP message-type byte (0x01) — managed by MctpPldmTransport
buf[1..] : PLDM request / response bytes
```

Size the buffer to accommodate the largest PLDM message your application
expects (typically ≤ 4096 bytes; smaller for embedded targets). See
`FD_IPC_MAX_MSG` in [`src/firmware_device.rs`](src/firmware_device.rs) for the
scratch-buffer size used internally for FD-initiated requests.

## Build

```
bazel build //services/pldm:pldm_service
```

Run the host-side integration tests:

```
bazel build -c dbg --strip=never //services/pldm:base_host_test
bazel build -c dbg --strip=never //services/pldm:firmware_update_host_test
```

After changing `pldm-common` / `pldm-interface` versions in
`third_party/crates_io/Cargo.toml`, re-pin the lock file. This repo is
bzlmod-only (no `WORKSPACE` file), so use `bazel build` with
`CARGO_BAZEL_REPIN=1`, not `bazel sync` (which errors with "WORKSPACE has to
be enabled"):

```
CARGO_BAZEL_REPIN=1 bazel build //services/pldm:pldm_service
```

## Dependencies

- [`openprot-mctp-api`](../mctp/api) — MCTP stack facade and traits
- [`pldm-interface`](https://github.com/OpenPRoT/pldm-lib/tree/main/pldm-interface) — PLDM command dispatcher (`CmdInterface`, `FdOps`, `FirmwareDeviceContext`)
- [`pldm-common`](https://github.com/OpenPRoT/pldm-lib/tree/main/pldm-common) — PLDM protocol types and MCTP transport helpers

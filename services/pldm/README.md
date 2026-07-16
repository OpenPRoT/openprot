# openprot-pldm-service

Platform-independent PLDM-over-MCTP responder service.

## Overview

This crate bridges [`openprot-mctp-api`](../mctp/api) and
[`pldm-interface`](https://github.com/OpenPRoT/pldm-lib/tree/main/pldm-interface)
so that firmware can receive and respond to PLDM messages transported over
MCTP without depending on any particular MCTP implementation or OS.

```text
┌──────────────────────────┐
│   Application / Firmware │  creates PldmResponder, calls run_once()
└───────────┬──────────────┘
            │
            ▼
┌──────────────────────────┐
│   openprot-pldm-service  │  dispatches to CmdInterface (this crate)
└───────────┬──────────────┘
            │ MctpListener / MctpRespChannel
            ▼
┌──────────────────────────┐
│   openprot-mctp-api      │  Stack<C: MctpClient>
└───────────┬──────────────┘
            │ IPC / transport
            ▼
┌──────────────────────────┐
│   MCTP Server            │
└──────────────────────────┘
```

## Key types

| Type | Description |
|------|-------------|
| `PldmResponder<'a>` | Holds a `CmdInterface`; call `run_once()` in a loop |
| `PldmServiceError` | Union of MCTP transport errors, PLDM handler errors, and overflow |
| `PLDM_MSG_TYPE` | MCTP message-type constant for PLDM (`0x01`) |

## Usage

```rust,ignore
use openprot_pldm_service::PldmResponder;
use pldm-interface::control_context::ProtocolCapability;
use pldm-common::protocol::base::{PldmControlCmd, PldmSupportedType};

const CTRL_CMDS: [u8; 5] = [
    PldmControlCmd::SetTid as u8,
    PldmControlCmd::GetTid as u8,
    PldmControlCmd::GetPldmCommands as u8,
    PldmControlCmd::GetPldmVersion as u8,
    PldmControlCmd::GetPldmTypes as u8,
];

let caps = [
    ProtocolCapability::new(PldmSupportedType::Base, "1.1.0", &CTRL_CMDS).unwrap(),
];

let mut responder = PldmResponder::new(&caps);
let mut buf = [0u8; 1024];

// `stack` is a `Stack<impl MctpClient>` obtained from the platform MCTP client.
loop {
    if let Err(e) = responder.run_once(&stack, &mut buf, 0) {
        // handle or log error
    }
}
```

## Buffer layout

`run_once` expects `buf` to be at least 2 bytes.  Internally byte 0 is
reserved for the MCTP message-type prefix (`0x01`); the PLDM payload is
received into `buf[1..]` and the response is written back in-place:

```text
buf[0]   : MCTP message-type byte (0x01) — managed by PldmResponder
buf[1..] : PLDM request / response bytes
```

Size the buffer to accommodate the largest PLDM message your application
expects (typically ≤ 4096 bytes; smaller for embedded targets).

## Build

```
bazel build //services/pldm:pldm_service
```

After adding `pldm-common` and `pldm-interface` to
`third_party/crates_io/Cargo.toml`, re-pin the lock file:

```
CARGO_BAZEL_REPIN=1 bazel sync
```

## Dependencies

- [`openprot-mctp-api`](../mctp/api) — MCTP stack facade and traits
- [`pldm-interface`](https://github.com/OpenPRoT/pldm-lib/tree/main/pldm-interface) — PLDM command dispatcher (`CmdInterface`)
- [`pldm-common`](https://github.com/OpenPRoT/pldm-lib/tree/main/pldm-common) — PLDM protocol types and MCTP transport helpers

# Platform Architecture

The orchestrator is the eRoT service that owns the
firmware lifecycle of the platform's downstream devices: verify before
running, supervise the boot, recover from corruption, apply updates,
report every degradation. In NIST SP 800-193 terms: *protect*, *detect*,
*recover*.

It is pure policy. It decides *when* a device leaves reset, *which* image
gets verified, *whether* an update activates — the platform's controllers
and services carry the decisions out. It never drives a wire, parses a bus
protocol, or holds a key (see [Where the responsibility
ends](#where-the-responsibility-ends)).

## Responsibilities

1. **Verified boot.** Walk the chain of trust in dependency order.
   Devices with eRoT-readable flash (BMC flash behind the SPI monitor)
   are verified (signature + SVN) while held in reset; the monitor's
   hardware write filter stays armed until first fetch, closing the
   time-of-check/time-of-use window. Devices with private flash (NIC,
   retimer) rely on their own boot ROM and join the trust chain only
   after SPDM attestation.
2. **Boot supervision.** After release, wait on the device's declared
   boot checkpoints (boot-complete GPIO, heartbeat, MCTP ready, version
   query). An expired checkpoint window means the boot failed — hung
   devices report nothing.
3. **Recovery and isolation.** Restore a corrupt device from its recovery
   image, with a bounded retry count. If its policy allows degraded
   operation, isolate it instead: hold in reset, drop from the trust
   chain, report. If a required device cannot be recovered, latch the
   platform locked.
4. **Firmware update.** Accept, authenticate, stage, activate; defer
   requests while a boot walk, update, or recovery is in flight. The
   anti-rollback (SVN) floor advances only after the new image proves it
   boots and runs.
5. **Attestation and reporting.** Answer attestation challenges as the
   SPDM responder's policy half: the orchestrator decides which
   measurements answer a challenge, the crypto engine signs them on its
   behalf — the key never leaves the vault, the transport only carries
   the session. Report isolation, recovery failure, and deferred or
   aborted updates to platform management.

## Structure

Three board-agnostic layers:

- **Policy state machine** (`services/orchestrator/sm`) — a pure reducer
  that consumes events (verification results, boot signals, update requests,
  timeouts) and emits effects (verify this image, release that reset,
  restore golden image). It performs no I/O itself.
- **Device capabilities** (`services/orchestrator/capabilities`) — the narrow contracts
  the state machine's effects are executed against, e.g. `BootControl`
  (hold a device in reset / release it). HAL bindings live in
  `services/orchestrator/hal-adapters`.
- **Board device table** (`services/orchestrator/config`, schema; values in
  `target/<board>/devices.rs`) — declares the managed devices: reset signal,
  boot checkpoints and windows, commit policy. Validated at compile time, so
  a malformed table is a build error.

## Where the responsibility ends

The orchestrator stops at the capability contracts.
Controllers that touch signals and buses belong to the platform HAL and
drivers; crypto and transport are services it *uses* but does not own.
Two rules follow:

- **Decide *what*, never *how*.** The orchestrator says "hold device 3
  in reset" — not "drive GPIO pin 14 low." The pin-to-device mapping lives
  in the board device table; the register access lives in the HAL behind
  the capability traits. Porting to a new board means a new table and HAL,
  zero policy changes.
- **Protection survives its crash.** The SPI monitor filters flash traffic
  in hardware, on its own; the orchestrator only loads its rules at
  boot and is not in the data path. Busy or crashed, it cannot be bypassed
  — there is nothing to bypass.

```mermaid
flowchart TB
    TABLE["Board device table<br/>target/&lt;board&gt;/devices.rs<br/>(pure data, board-owned)"]
    BMC["Platform management<br/>(BMC / host)"]

    subgraph ORCH["Orchestrator&nbsp;(one&nbsp;process)"]
        SM["Boot state machine<br/>verify / release / supervise /<br/>recover / update / lock"]
        MOD["Update, recovery,<br/>anti-rollback modules<br/>(libraries, not services)"]
        CAP["Device capabilities<br/>BootControl and peers<br/>(+ HAL adapters)"]
    end

    NET["MCTP / PLDM / SPDM<br/>(transport task, protocols as libs;<br/>carries data, holds no authority —<br/>SPDM responder decisions: orchestrator)"]
    CRYPTO["Crypto engine<br/>+ key vault"]
    STORE["Storage<br/>pending-update record,<br/>retry counts, lockdown latch<br/>(write: orchestrator only)"]
    SPI["SPI monitor<br/>flash access, bus filtering"]
    RST["Reset controller<br/>(fail-safe: lines come up<br/>asserted after any restart)"]
    GPIO["GPIO controller"]

    DEV["Downstream devices<br/>(BMC flash, NIC, retimer, ...)"]

    %% invisible links first: they win dagre's cycle-breaking, pinning the layers
    BMC ~~~ MOD & CAP
    ORCH ~~~ GPIO
    SM ~~~ STORE
    MOD ~~~ GPIO

    %% config is compiled in, not owned
    TABLE -.->|"compiled in"| ORCH

    %% intake path — BMC never talks to the orchestrator directly
    BMC <-->|"PLDM update / status"| NET
    NET -->|"candidate<br/>staged"| ORCH

    %% commands (down, fire-and-forget) and events (up) — replies are queued events, the orchestrator never blocks
    ORCH -->|"read /<br/>restore image"| SPI
    ORCH -->|"hash, verify sig,<br/>sign attestation"| CRYPTO
    ORCH -->|"persist / resume scan<br/>(sole write capability)"| STORE
    ORCH -->|"assert / release"| RST
    CRYPTO -->|"pass / fail<br/>verdict"| ORCH
    STORE -->|"ack /<br/>resume state"| ORCH
    RST -->|"restart<br/>notice"| ORCH
    SPI -->|"corruption<br/>detected"| ORCH
    GPIO -->|"boot-complete<br/>line"| ORCH
    NET -->|"boot<br/>signals"| ORCH

    %% hardware
    RST --> DEV
    GPIO <--> DEV
    SPI <--> DEV
    NET <-->|"MCTP to active devices:<br/>heartbeat, MCTP ready,<br/>version query, SPDM, PLDM"| DEV


    %% all plain nodes outside the box are platform services: own tasks, shared, board-wired
    classDef svc fill:#eef6ee,stroke:#7a9a7a
    class NET,CRYPTO,STORE,SPI,RST,GPIO svc
```

The six green boxes outside the orchestrator box are platform services —
each one its own task. Four rules govern how the orchestrator relies on
them:

- **Never block.** Commands are fire-and-forget; every reply (a crypto
  verdict, a storage ack) returns as a queued event, so a long image hash
  cannot delay the judgment of a boot window elsewhere.
- **Exclusive security state.** Only the orchestrator holds the write
  capability for the lockdown latch, retry counts, and pending-update
  record; a persist with unknown outcome counts as failed and latches the
  machine locked.
- **Fail-safe resets.** Managed reset lines default to asserted in
  hardware on every controller restart, not just cold boot — a
  controller crash resets healthy running devices. That availability
  hit is deliberate: an unsupervised release is never accepted. The
  restart notice makes the orchestrator re-run the boot walk.
- **Transport without authority.** Images and attestation are
  authenticated end-to-end (signatures via crypto, SPDM sessions), so the
  transport can drop traffic but never forge it; a dropped boot signal
  just becomes a checkpoint timeout, and the boot-complete GPIO line
  remains a transport-free liveness path.

## TODO

- Define the lockdown latch: where it lives, what "locked" gates, and
  how it is cleared. Latching on a failed persist must not itself depend
  on the storage service that just failed.

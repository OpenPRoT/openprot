# PLDM-FD as IPC Server

How the PLDM Firmware Device (FD) and the orchestrator communicate when the FD
runs as an IPC server and the orchestrator is its client. Modeled on the MCTP
server/client split: the FD process owns a dispatch loop that decodes IPC
requests and routes them to the firmware-device state machine. The orchestrator
talks to it through `ServiceCall` requests, each one signalling the caller's
`WaitGroup` when it completes.

pldm-lib owns the firmware-device state machine and drives the PLDM protocol.
It holds the phases and knows nothing about the orchestrator, which keeps it
implementation agnostic. Our FdOps implementation runs in the FD process,
executes the firmware operations and is the only place that names the
orchestrator: `download_fw_data` writes chunks to flash via the device server.
`verify` delegates to the crypto service, which reads the staged image from the
device server itself. `apply` and `activate` take the board-specific logic from
the platform driver trait and run it through the device server. The platform
driver is a trait linked at build time, not a service; board-specific behavior
enters via trait implementations. Before download via AcceptOffer, before
verify and before apply, the FdOps implementation nudges the orchestrator and
waits for an order, while pldm-lib keeps polling the same callback and the
state machine stays in the phase it already reached. Activation is decided one
step earlier, see below. The orchestrator is a
gatekeeper: it can block any phase (e.g. refuse verify for an isolated component
with RejectVerify; the FD reports it to the UA as a verify failure), but it does
not execute the operations itself. The orchestrator never blocks: it must stay
responsive to async events (e.g. CompromiseDetected). All outbound operations
(IPC to FD, SVN bump, write filter) go through ServiceCall, and their
completion signals sit in the orchestrator's WaitGroup alongside event signals.
In transport, FdOps callbacks must not block for long: the UA can send
CancelUpdate at any point, and the responder path in the FD has to stay live
to answer it. Out of transport the one callback does block, for the reason the
asymmetry bullet below gives.

Design decisions:

- ServiceCall is the universal IPC primitive. Every cross-process request
  (FdOps to device server, FdOps to crypto, orchestrator to FD) goes
  through `ServiceCall<Req, Resp>`: `start()` is `channel_async_transact`,
  `signal()` is READABLE on the initiator handle, which `start()` clears and
  the handler's response raises, and `try_recv()` is
  `channel_async_transact_complete`. The caller adds that handle to its
  `WaitGroup`. No process ever blocks on another; `object_wait` on the
  WaitGroup sleeps the thread until any signal fires. A handler that dies
  never raises READABLE, which is the same hole as the missing peer-closed
  signal in the FD-death answer below.
- The kernel puts two constraints on ServiceCall. The send and receive
  buffers must stay valid until the transaction completes or is cancelled.
  That is why `channel_async_transact` is unsafe. ServiceCall takes Req and
  Resp as `&'static mut` for the call's lifetime instead of holding them
  inline: the kernel keeps raw pointers into both buffers, and buffers held
  inline would dangle if ServiceCall moved after `start()`
  (the module comment in util/ipc/async_transaction.rs). So no borrowed
  stack buffer, and no scratch buffer shared between channels: each
  in-flight channel needs its own. Separately,
  a channel carries one transaction at a time and a second `start()` on a
  busy channel returns `Unavailable`, so one outstanding ServiceCall per
  channel is the kernel's rule rather than ours. The same `Unavailable`
  comes back from `try_recv()` when the transaction is still pending or
  was already cancelled, so the caller keys off which call returned it,
  not the error value alone. `channel_async_cancel` drops a pending
  transaction. A blocking `transact()` never queues behind an in-flight
  async one; it fails immediately with `Unavailable` via the same path.
  Nothing in this tree uses the async calls yet, i2c and mctp both block
  in `channel_transact`, so the contract is spelled out here rather than
  pointed at.
- The orchestrator never blocks. Its WaitGroup multiplexes FD nudge signals,
  ServiceCall completions (SVN bump, write filter), CompromiseDetected, and
  timers. Any event gets handled on the next wake, regardless of what else
  is in flight. The FD never parks either. A polled FdOps callback returns at
  once whether or not the order has arrived, reporting 0% progress until it
  has, so the dispatch loop and the MCTP responder stay live through the
  wait.
- The FD executes download, verify, and apply through FdOps callbacks. Our
  verification code goes in FdOps::verify, which calls the crypto service
  over IPC (a separate process holds the keys). The crypto service reads
  the staged image directly from the device server, so the FD never
  relays image data. The orchestrator does not read flash or check
  signatures itself.
- Gatekeeper pattern: before verify and apply the FdOps implementation nudges
  the orchestrator and waits for an order or a rejection, and for activation it
  nudges as soon as apply finishes. The orchestrator can reject any phase (e.g.
  component is isolated, update policy violation) and the FdOps callback
  returns the rejection to the UA through its return value. This keeps the
  orchestrator lightweight and non-blocking.
- Perform gates add no PLDM states. pldm-lib moves the state machine into
  Verify and Apply automatically per DSP0267, and the gate lives inside the
  polled FdOps callback rather than in the state machine: pldm-lib calls
  verify/apply repeatedly via `fd_progress`, and our implementation returns
  success with 0% progress until the orchestrator orders. Between polls the
  dispatch loop and MCTP responder stay live, so there is no deadlock and no
  spec violation.
- Delegated verification. FdOps::verify sends a single "verify this image"
  request to the crypto service on the first poll, then checks for the
  completion signal on each subsequent poll. The crypto service owns the
  full pipeline (read from device server, hash, check signature) in its
  own process. The FD never touches image data or hash state, and there
  is no accumulated state to lose on cancel.
- Nudges are FD to orchestrator only, level-triggered USER signals. Same
  mechanism the i2c server-runtime uses to announce a latched slave receive
  (the module comment in services/i2c/server-runtime/src/lib.rs). The FdOps
  implementation raises the signal when the orchestrator needs to act
  (offer ready, order needed,
  phase complete) and clears it when it answers the orchestrator's QueryStatus.
  pldm-lib never raises it.
- All orchestrator-to-FD IPC ops get an immediate `Reply`, no
  `DispatchOutcome::Pending`.
- Dispatch follows the MCTP server pattern: `pldm_server::dispatch` decodes a
  request header, calls the appropriate method, encodes the response. The
  `dispatch_pldm_op` name belongs to the MCTP server, not this one.
- In-transport and out-of-transport are two UA commands, not one flow with a
  step removed. In-transport runs the DSP0267 update state machine;
  out-of-transport is a single ActivatePendingComponentImage answered in Idle.
  The per-phase gates are the same on both; what differs is the ops around
  them. AcceptOffer, RejectOffer and AckCancel are in-transport only, and
  ArmPendingImage is out-of-transport only, because there the decision has to
  precede the UA command.
- The two paths wait differently, and the reason is the state machine. In
  transport the FD is inside a DSP0267 update session: the UA runs its timers,
  CancelUpdate and GetStatus have to be answerable mid-phase, so the gate has
  to return at 0% rather than park. Out of transport the state machine never
  leaves Idle, no session timer is running, and the UA has one outstanding
  command whose response carries the outcome, so running the whole sequence
  inside handle_pending_component costs nothing the UA can observe except its
  own wait.
- Minimal copies: firmware lands in its final staging region and is verified
  in place. FdOps::download_fw_data writes directly to the staging address;
  FdOps::verify reads from there. No intermediate buffers or extra copies
  between download, verify, and apply.

## In-transport image transfer

The FD pulls firmware chunks from the UA over MCTP. Each chunk is written to
the staging region by FdOps::download_fw_data, which calls the flash device
server over IPC. Firmware bytes do not pass through the orchestrator. After
transfer, the FD asks the orchestrator for permission to verify, then runs
FdOps::verify (which delegates to the crypto service; the crypto service
reads the staged image directly from the device server). The orchestrator
also orders apply before the FD commits the image. The orchestrator handles
activation. It does not touch the security revision here; that is a separate
command the UA sends later, see "Security revision commit" below.

```mermaid
sequenceDiagram
    participant UA as UA (BMC)<br/>remote, over MCTP
    participant FD as PLDM-FD (server)<br/>dispatch loop + run_terminus
    participant Orch as Orchestrator (client)<br/>ServiceCall
    participant DevSrv as Device Server<br/>manages the SPI flash
    participant Crypto as Crypto Service<br/>hash + signature verification

    Note over UA, Crypto: Blue background: orchestrator IPC. Green background: FdOps service IPC.

    Note over UA, Orch: NEGOTIATION (PLDM protocol, MCTP only)

    UA->>FD: RequestUpdate (MCTP)
    FD-->>UA: RequestUpdate response (accepted)
    UA->>FD: PassComponentTable (MCTP)
    FD-->>UA: PassComponentTable response
    UA->>FD: UpdateComponent (MCTP)
    FD-->>UA: UpdateComponent response

    Note over FD, Orch: FD has an offer, nudge the orchestrator

    FD->>Orch: USER signal (nudge: offer ready)

    rect rgb(230, 240, 255)
    activate Orch
    Orch->>FD: ServiceCall: QueryStatus
    FD-->>Orch: Status::OfferPending { target, total, mode: InTransport }
    Note right of Orch: validate target + total,<br/>platform driver picks<br/>staging address,<br/>reserve staging,<br/>open staging window
    Orch->>FD: ServiceCall: AcceptOffer { base: FlashAddress }
    FD-->>Orch: Ok
    deactivate Orch
    end

    Note over UA, DevSrv: TRANSFER (FD pulls from UA, FdOps writes to flash)

    rect rgb(230, 255, 230)
    loop FdOps::download_fw_data per chunk
        FD->>UA: RequestFirmwareData (MCTP)
        UA-->>FD: firmware chunk
        FD->>DevSrv: ServiceCall: write chunk
        DevSrv-->>FD: signal: Ok
    end
    end

    Note over FD, Orch: transfer done, ask orchestrator to order verify

    FD->>UA: TransferComplete (MCTP)
    FD->>Orch: USER signal (nudge: verify pending)

    rect rgb(230, 240, 255)
    activate Orch
    Orch->>FD: ServiceCall: QueryStatus
    FD-->>Orch: Status::VerifyPending
    Note right of Orch: check isolation, update policy,<br/>close staging window
    Orch->>FD: ServiceCall: PerformVerify
    FD-->>Orch: Ok
    deactivate Orch
    end

    Note over FD, Crypto: FdOps::verify

    rect rgb(230, 255, 230)
    FD->>Crypto: ServiceCall::start(VerifyRequest { addr, size })
    Crypto->>DevSrv: read staged image
    DevSrv-->>Crypto: image data
    loop fd_progress poll
        Note over FD: verify() returns 0%, no signal yet
    end
    Crypto-->>FD: signal: Verdict
    Note over FD: verify() poll: try_recv -> 100% + verdict
    end
    FD->>UA: VerifyComplete (MCTP)

    Note over FD, Orch: verify done, ask orchestrator to order apply

    FD->>Orch: USER signal (nudge: apply pending)

    rect rgb(230, 240, 255)
    activate Orch
    Orch->>FD: ServiceCall: QueryStatus
    FD-->>Orch: Status::ApplyPending
    Orch->>FD: ServiceCall: PerformApply
    FD-->>Orch: Ok
    deactivate Orch
    end

    Note over FD, DevSrv: FdOps::apply

    rect rgb(230, 255, 230)
    FD->>DevSrv: ServiceCall::start(apply: commit staged image)
    loop fd_progress poll
        Note over FD: apply() returns 0%, no signal yet
    end
    DevSrv-->>FD: signal: Ok
    Note over FD: apply() poll: try_recv -> 100%
    end
    FD->>UA: ApplyComplete (MCTP)

    Note over FD, Orch: apply done, decide activation before the UA asks

    FD->>Orch: USER signal (nudge: activation decision)

    rect rgb(230, 240, 255)
    activate Orch
    Orch->>FD: ServiceCall: QueryStatus
    FD-->>Orch: Status::ActivationPending
    Note right of Orch: check isolation, update policy
    Orch->>FD: ServiceCall: PerformActivate
    FD-->>Orch: Ok
    deactivate Orch
    end

    Note over UA, Orch: ACTIVATION (FdOps::activate, any time later)

    UA->>FD: ActivateFirmware (MCTP)
    FD-->>UA: ActivateFirmware response (accepted, from the stored order)

    rect rgb(230, 255, 230)
    FD->>DevSrv: ServiceCall: FdOps::activate: set boot preference
    DevSrv-->>FD: signal: Ok
    end

    FD->>Orch: USER signal (nudge: activated)

    rect rgb(230, 240, 255)
    activate Orch
    Orch->>FD: ServiceCall: QueryStatus
    FD-->>Orch: Status::Idle { reason: ActivateFw }
    Note right of Orch: release staging,<br/>start judging the boot
    deactivate Orch
    end

    Note over UA, DevSrv: SVN COMMIT (later, FD back in IDLE, new image running)

    Note right of Orch: judge the boot, then<br/>TrialBoot::confirm or revert
    UA->>FD: UpdateSecurityRevision (MCTP, 0x22)
    FD->>Orch: USER signal (nudge: SVN commit requested)

    rect rgb(230, 240, 255)
    activate Orch
    Orch->>FD: ServiceCall: QueryStatus
    FD-->>Orch: Status::SvnCommitPending { component }
    Note right of Orch: a confirmed trial only,<br/>else RejectSvnCommit
    Orch->>DevSrv: ServiceCall: SvnFloor::advance<br/>(Erot binding only, a PLDM FD<br/>commits the floor it holds)
    DevSrv-->>Orch: signal: Ok
    Orch->>FD: ServiceCall: PerformSvnCommit
    FD-->>Orch: Ok
    deactivate Orch
    end

    FD-->>UA: UpdateSecurityRevision response (success)

    Note over UA, Orch: CANCEL (between AcceptOffer and activation)
    UA->>FD: CancelUpdate (MCTP)
    rect rgb(230, 255, 230)
    FD->>DevSrv: ServiceCall: FdOps::cancel_update_component
    DevSrv-->>FD: signal: Ok
    end
    FD-->>UA: CancelUpdate response
    FD->>Orch: USER signal (nudge: cancelled)
    rect rgb(230, 240, 255)
    activate Orch
    Orch->>FD: ServiceCall: QueryStatus
    FD-->>Orch: Status::Cancelled
    Note right of Orch: release staging,<br/>close staging window
    Orch->>FD: ServiceCall: AckCancel
    FD-->>Orch: Ok
    deactivate Orch
    end
```

## Out-of-transport image transfer

Out-of-transport is a different UA command, not the same session with the
transfer step removed. A third party writes the image into the staging region,
and the UA then sends ActivatePendingComponentImage (DSP0267 0x1F). pldm-lib
answers that command only in Idle and rejects it in every other state
(activate_pending_component_rsp in
pldm-interface/src/firmware_device/fd_context.rs), so there is no
RequestUpdate, no PassComponentTable, no UpdateComponent and no update mode on
this path. The FD never enters the download/verify/apply state machine.

Everything we do hangs off one callback. pldm-lib decodes the request and calls
FdOps::handle_pending_component, which is synchronous and returns a
PendingComponentResult: Activated, ActivationNotRequired, or NotPermitted. The
image is already in staging, so nothing downloads here: verify, apply and
activate all run inside that callback, on the `run_terminus` loop in the FD
process, since that loop is what reaches pldm-lib and pldm-lib is what calls
the callback. The 0x1F response goes back to the UA once the sequence has
finished.

The orchestrator still arms the FD ahead of the UA command. A blocking callback
could query the orchestrator inline, but staging has to be reserved and the
writer window closed before 0x1F arrives, so the orchestrator has already
decided by then; ArmPendingImage is the one-shot token that carries the
decision, naming the component and where the image sits.
handle_pending_component answers from that arming: armed runs the sequence, not
armed returns ActivationNotRequired,
which is Table 44's "no pending image" and is what not-armed means to the UA.
NotPermitted is for an FD that does not do pending activation at all, so we
never return it. The arming is one-shot, consumed when the sequence starts, so a
failed run does not leave an image the next 0x1F would retry; the orchestrator
has to arm again.

The per-phase gates still run. Verify, apply and activate go through the same
ServiceCalls and the same orchestrator nudges as in-transport, issued from
inside the callback. pldm-lib drives nothing here: its state machine stays Idle,
so there is no fd_progress to poll and no PLDM state to report through.

The callback holds the responder for the whole sequence, so the UA gets no
answer to CancelUpdate or GetStatus until it returns. In return the 0x1F
response carries the real outcome: a verify or apply failure reaches the UA in
the completion code instead of through our status path. A policy refusal still
reads to the UA as no pending image, because Table 44 has no other code for it;
the distinction lives orchestrator-side.

```mermaid
sequenceDiagram
    participant UA as UA (BMC)<br/>remote, over MCTP
    participant FD as PLDM-FD (server)<br/>dispatch loop + run_terminus
    participant Orch as Orchestrator (client)<br/>ServiceCall
    participant DevSrv as Device Server<br/>manages the SPI flash
    participant Crypto as Crypto Service<br/>hash + signature verification

    Note over UA, Crypto: Blue background: orchestrator IPC. Green background: FdOps service IPC.

    Note over UA, Crypto: STAGING (no PLDM session, FD state machine in Idle)

    Note right of Orch: a third party writes the image<br/>into the staging region.<br/>How it learns the address,<br/>and what tells the orchestrator<br/>the copy finished, are open

    rect rgb(230, 240, 255)
    Note right of Orch: platform driver picks<br/>the staging address,<br/>reserve staging,<br/>close the writer's window
    Orch->>FD: ServiceCall: ArmPendingImage { component, base, total }
    FD-->>Orch: Ok
    end

    Note over UA, FD: ACTIVATION REQUEST (DSP0267 0x1F, answered only in Idle)

    UA->>FD: ActivatePendingComponentImage (MCTP)
    activate FD
    Note over FD: FdOps::handle_pending_component:<br/>armed, so run verify, apply and<br/>activate before returning

    Note over FD, Crypto: VERIFY (inside the callback, the pldm-lib state machine stays Idle)

    FD->>Orch: USER signal (nudge: verify pending)

    rect rgb(230, 240, 255)
    activate Orch
    Orch->>FD: ServiceCall: QueryStatus
    FD-->>Orch: Status::VerifyPending
    Note right of Orch: check isolation, update policy
    Orch->>FD: ServiceCall: PerformVerify
    FD-->>Orch: Ok
    deactivate Orch
    end

    rect rgb(230, 255, 230)
    FD->>Crypto: ServiceCall::start(VerifyRequest { addr, size })
    Crypto->>DevSrv: read staged image
    DevSrv-->>Crypto: image data
    Crypto-->>FD: signal: Verdict
    end

    Note over FD, Orch: verify done, ask orchestrator to order apply

    FD->>Orch: USER signal (nudge: apply pending)

    rect rgb(230, 240, 255)
    activate Orch
    Orch->>FD: ServiceCall: QueryStatus
    FD-->>Orch: Status::ApplyPending
    Orch->>FD: ServiceCall: PerformApply
    FD-->>Orch: Ok
    deactivate Orch
    end

    rect rgb(230, 255, 230)
    FD->>DevSrv: ServiceCall: apply: commit staged image
    DevSrv-->>FD: signal: Ok
    end

    Note over FD, Orch: apply done, ask orchestrator to order activation

    FD->>Orch: USER signal (nudge: activation decision)

    rect rgb(230, 240, 255)
    activate Orch
    Orch->>FD: ServiceCall: QueryStatus
    FD-->>Orch: Status::ActivationPending
    Note right of Orch: check isolation, update policy
    Orch->>FD: ServiceCall: PerformActivate
    FD-->>Orch: Ok
    deactivate Orch
    end

    rect rgb(230, 255, 230)
    FD->>DevSrv: ServiceCall: set boot preference
    DevSrv-->>FD: signal: Ok
    end

    FD->>Orch: USER signal (nudge: activated)

    rect rgb(230, 240, 255)
    activate Orch
    Orch->>FD: ServiceCall: QueryStatus
    FD-->>Orch: Status::Idle { reason: ActivateFw }
    Note right of Orch: release staging,<br/>start judging the boot
    deactivate Orch
    end

    Note over FD: handle_pending_component returns
    FD-->>UA: response: Activated
    deactivate FD

    Note over UA, DevSrv: SVN COMMIT (same as in-transport)

    Note right of Orch: judge the boot, then<br/>TrialBoot::confirm or revert
    UA->>FD: UpdateSecurityRevision (MCTP, 0x22)
    Note over FD, Orch: nudge, QueryStatus, PerformSvnCommit or RejectSvnCommit

    Note over UA, Orch: FAILURE (verify or apply)

    FD->>Orch: USER signal (nudge: phase failed)

    rect rgb(230, 240, 255)
    activate Orch
    Orch->>FD: ServiceCall: QueryStatus
    FD-->>Orch: Status::PhaseFailed { phase, result }
    Note right of Orch: release staging,<br/>the arming is already spent
    deactivate Orch
    end
```

There is no CancelUpdate on this path. The UA sent one command and is still
waiting for its answer, cancel_update_rsp returns NOT_IN_UPDATE_MODE because
the FD is not in update mode, and the callback holds the responder until the
sequence ends. What stops a sequence that is already running, and how the UA
learns it stopped, is open; see the open questions.

## Write-access containment

The FD never holds a flash handle. It writes through the device server, one
ServiceCall per chunk, so the containment is the device server's to enforce.

First layer, in software. The orchestrator arms the device server at
AcceptOffer with the base address and total it just handed the FD, and disarms
it when the staging window closes, by the rule below. Writes outside that
range are rejected. This catches offset bugs in the FD. It does not catch a
compromised FD, which can still ask the device server to write anywhere inside
the window. Arming and disarming are orchestrator-to-device-server calls the
diagrams do not show yet.

Second layer, in hardware. The SMC write filter raises
`SmcInterrupt::WriteProtected` on writes outside the allowed region
(target/ast10x0/peripherals/smc/interrupts.rs). It sits in the eRoT's own
controller, so it checks every write that reaches flash through that controller
and cannot tell one eRoT-side writer from another. Whether the host can reach
staging at all depends on how the board is wired, and this filter does not
answer that. The orchestrator opens the region at AcceptOffer and closes it by
the same rule. What this needs from the hardware is one thing: whoever drives
erase and program must not reach the write-protect registers. How to get that
is open. Three candidates: a separate chip select for staging, a separate MPU
region over the filter registers, or lock-until-reset bits. The separate chip
select is the current preference, and it is also what keeps the host off
staging. Picking one needs the AST10x0 register layout, and it does not block
the rest of this design.

Both windows close before PerformVerify and stay closed until the next
AcceptOffer. Verify hashes what is on flash, and the verdict has to match the
image that apply later commits, so nothing may write staging from the order
onward. The window closes on the verify-pending nudge, on RejectVerify, on
Cancelled, on PhaseFailed, and when the backstop timer from the FD-death answer
fires. Closing twice is harmless.
Disarming the device server is itself a ServiceCall, so PerformVerify goes out
after its completion signal arrives, not after start(). A board whose apply
writes into the staging slot would break that rule, and how to handle one is
left until such a board exists.

Both layers cover staging only. Apply and activate are a separate problem: the
FD initiates both as ServiceCalls to the device server, and the order gate for
them lives inside the FD's own FdOps callback. A compromised FD skips its own
gate and calls the device server directly. Closing that means the device server
refuses apply and activate without an order it got from the orchestrator, not
from the FD. Open, and not covered by the write filter, which spans staging
only.

Out-of-transport is different. The FD writes nothing, so the first layer does
not apply to it. The writer is the third party the orchestrator handed the
staging address to, and the same two questions land on that path: what bounds
its writes, and who opens the filter for it. Not answered here. The same close
applies: whatever window that writer has shuts before ArmPendingImage, which is
what makes the image eligible for activation at all. The orchestrator closes it
when it learns the copy finished, and how it learns that is open.

## Activation reporting

The orchestrator decides activation before the UA asks for it. FdOps::activate
is synchronous, and what it returns is the completion code in the
ActivateFirmware response. fd_progress never polls Activate either, so the FD
cannot report 0% and wait there the way it does for verify and apply. Instead
the FD nudges when apply completes, the orchestrator checks its policy while
nothing is waiting on it, and sends PerformActivate or RejectActivate. The FD
stores that verdict and answers ActivateFirmware from it: an order returns
success and starts the boot-preference ServiceCall, a denial returns
INCOMPLETE_UPDATE, which per DSP0267 leaves the FD in READY XFER without
activating. The gap before the UA asks is unbounded, so the order is revocable:
a RejectActivate arriving later overrides a stored order, so CompromiseDetected
can still stop an activation the UA has not asked for yet. Once the UA does
ask, the FD has answered and the boot-preference call is running, so a refusal
racing that request loses. Closing that race needs the device server to refuse
activate without an orchestrator-side order, which is the containment gap
above. A session that ends any way but activation, a cancel or an FD_T1 timeout
alike, invalidates a stored order, and the FD nudges again when activation is
done so the orchestrator can release staging and start judging the boot.

CancelUpdate answers immediately too, from the FD's own state, the way pldm-lib
already does it. AckCancel is the orchestrator's bookkeeping, not permission.

Activation reports, it does not roll back. The ActivateFirmware response means
accepted, not done. The UA learns the outcome from GetStatus AuxStateStatus and
from GetFirmwareParameters, which shows which version is actually active. The
diagrams do not show that poll.

A failed activation leaves the boot preference unchanged, so the old image
keeps booting and the UA can retry. A bad new image is the trial boot's problem
instead: the floor stays where it is until UpdateSecurityRevision, so the
superseded image stays bootable and TrialBoot can revert to it. That is the
window the security revision commit section describes, seen from the failure
side.

## Security revision commit

Activation does not raise the anti-rollback floor. If it did, a trial boot
could never be reverted: the superseded image would sit below the new floor
and refuse to run. DSP0267 1.3.0 keeps the two apart with
UpdateSecurityRevision (command 0x22, section 12.19), which the UA sends once
it is satisfied with the image. The FD applies and activates without touching
the revision, and until 0x22 arrives a downgrade is still allowed, which is the
window the trial boot lives in. The Security Revision Number Delayed Update
option (`UpdateOptionFlags` bit 2) on UpdateComponent is how the UA announces
it will send 0x22 later. The design does not key off the bit: the orchestrator
gates 0x22 on the boot verdict whether or not the UA announced it.

The FD accepts 0x22 only in the IDLE state, so it arrives outside update mode,
minutes or days after activation. It acts on the active running image, not a
pending one, which is what makes it safe to gate on the boot verdict.

Who writes the floor depends on the binding. A component bound
`SvnFloorBinding::Erot` keeps its floor on our side, and on a confirmed trial
the orchestrator advances `SvnFloor` for it before sending PerformSvnCommit. A
PLDM Firmware Device commits its own floor internally, which is the default
binding here, so the eRoT holds no second floor for it and PerformSvnCommit
carries only the verdict. Either way the FD relays the request and the answer
and the orchestrator decides, the same split the rest of this design uses for
anything irreversible. On an unconfirmed or absent trial the orchestrator
rejects with PolicyViolation and the FD returns
UPDATE_SECURITY_REVISION_NOT_PERMITTED. DSP0267 has no code for a policy
refusal, so that capability code is the nearest fit.

## IPC operations

The orchestrator's IPC vocabulary. Each one is a `ServiceCall`: request in,
response out, nothing kept on the wire between them. The FD answers every op
from state it already holds, so the answer is back by the orchestrator's next
wake.

| Op | Direction | Purpose |
|---|---|---|
| AcceptOffer | orch -> FD | Accept with a staging base address (in-transport) |
| ArmPendingImage | orch -> FD | Out-of-transport: a staged image is eligible for activation; the FD answers the UA's ActivatePendingComponentImage from this. Not a Perform* op because it precedes the UA command rather than ordering a phase |
| RejectOffer | orch -> FD | Refuse the offer, with the reason the requester is owed (FD tells UA in the next response) |
| PerformVerify | orch -> FD | Order the FD to run FdOps::verify |
| RejectVerify | orch -> FD | Block verify (e.g. isolated component); FD returns failure to UA |
| PerformApply | orch -> FD | Order the FD to run FdOps::apply |
| RejectApply | orch -> FD | Block apply; FD returns failure to UA |
| QueryStatus | orch -> FD | Read current FD state (phase, result, error); when OfferPending, includes offer data (target, total, SVN delayed) |
| PerformActivate | orch -> FD | Order activation ahead of the UA's request; the FD stores it |
| RejectActivate | orch -> FD | Refuse activation, or revoke a stored order; FD answers the UA with INCOMPLETE_UPDATE |
| AckCancel | orch -> FD | Acknowledge cancel, release orchestrator-side resources |
| PerformSvnCommit | orch -> FD | Tell the FD the floor is raised, so it can answer the UA |
| RejectSvnCommit | orch -> FD | Block the commit, reason PolicyViolation (no confirmed trial); FD answers the UA with UPDATE_SECURITY_REVISION_NOT_PERMITTED |

A rejected offer ends with TransferComplete. The diagrams answer
UpdateComponent before the orchestrator's QueryStatus, so by the time the
orchestrator rejects there is no pending UA command to fail. The FD sends
TransferComplete with FdAbortedTransfer (0x03) instead of waiting for the
protocol to time out, and calls FdOps::cancel_update_component on its way
out.

## Status

QueryStatus answers with the condition the FD is really in. Some of those are
DSP0267 states: Idle carries the reason the FD went idle, one of the
GetStatusReasonCode values, ActivateFw after an activation, a cancel reason, a
timeout reason like VerifyTimeout, and nothing at all on a fresh start, which
is how a restarted FD is told apart from one that finished; ReadyXfer is the
spec state of the same name. Some are conditions the implementation has without
a spec name for them: PhaseFailed is a phase plus a failed request, and it
carries the phase and the DSP0267 result code the FD already sent the UA. The
rest are decisions the orchestrator owes the FD, the pending phases and
SvnCommitPending.

PhaseFailed is a real place to sit, not an error flag. A failed verify or apply
leaves the FD in that phase with its request marked failed, waiting for the UA
to cancel (process_verify_complete_rsp in
pldm-interface/src/firmware_device/fd_context.rs), and FD_T1 resets on every
message, so a polling UA can hold it there for as long as it likes. The
orchestrator therefore releases staging and closes the window on PhaseFailed
rather than on the cancel that may follow much later; a retry re-enters through
a new offer and reserves again. How the session continues is the UA's choice,
not a field: a CancelUpdateComponent puts the FD in ReadyXfer, a CancelUpdate
puts it in Idle. That exit is in-transport only. Out-of-transport the state
machine is already Idle and cancel returns NOT_IN_UPDATE_MODE, so PhaseFailed
there is a condition of the FD's own loop that the UA cannot clear.

Cancelled is a session condition with a lifetime. The FD reports it from the
UA's CancelUpdate until the orchestrator's AckCancel, and afterwards reports
Idle or ReadyXfer depending on which cancel command arrived. That is what
AckCancel does to the FD's state; it orders nothing.

A QueryStatus between nudges answers with the phase in flight, which is what
the orchestrator's backstop timer probes for. Any well-formed answer means the
FD is alive, so the probe tells a slow transfer apart from a dead FD without
needing a separate liveness signal.

ApplyPending carries no verify verdict. The FD enters Apply only from a
successful verify, in the success branch of process_verify_complete_rsp, so a
flag there would always be true. A verify that fails shows up as PhaseFailed
with the VerifyResult code instead.

## Wire format

Fixed 8-byte header. Most ops carry no payload.

```text
Request:
+----+-------+-----+----------+
| op | flags | gen | reserved |  + [args]
| 1B |  1B   | 2B  |    4B    |
+----+-------+-----+----------+

Response:
+------+-------+-----+-------------+----------+
| code | flags | gen | payload_len | reserved |  + [payload]
|  1B  |  1B   | 2B  |    2B LE    |    2B    |
+------+-------+-----+-------------+----------+
```

The reserved bytes pad both headers to 8, so args and payload start at the same
offset on either side and a future field costs no reframing. `to_bytes` writes
them as zero and `from_bytes` ignores them
(`ResponseHeader::to_bytes` in services/pldm/api/src/wire.rs).

`op` decodes through `TryFrom<u8>` and an unknown value is a decode error, not
a panic. Decode failures get their own type: Truncated, InvalidOpcode,
BufferTooSmall, PayloadTooLarge, InvalidValue. Each op declares the request
length it expects, and the server rejects a longer frame as PayloadTooLarge
rather than ignoring the tail (`expected_request_len` in
services/pldm/api/src/wire.rs and `dispatch_inner` in
services/pldm/server/src/lib.rs). MAX_REQUEST_SIZE and
MAX_RESPONSE_SIZE live in the api crate and size the buffers on both sides.

`code` is the on-wire result. The client hands it back as
ClientError::Refused, carrying the code unchanged. The reason behind a refusal
travels one way only: RejectReason (Isolated, PolicyViolation, UnknownTarget,
Busy) is an argument the orchestrator sends on a Reject* request, and the
response header has no field to carry a reason back
(`encode_with_reason` in services/pldm/api/src/wire.rs and `ResponseCode`
in services/pldm/api/src/error.rs).
The FD maps the reason it was given onto a DSP0267 completion code for the UA.
Nothing the FD refuses on its own reaches the orchestrator as more than
ResponseCode, which is transport-level; adding an FD-side reason means a new
response field.

The FD answers RequestUpdate from its own state machine: an update already in
progress gets ALREADY_IN_UPDATE_MODE and the UA retries. No nudge.

`gen` is the FD's phase generation, still open. See the open question on
whether an order carries a token.

## Nudges

The FD raises a USER signal on the orchestrator's `WaitGroup` when state
changes. Level-triggered (OR'd into active_signals, persists until lowered).
Every Signals::USER in this tree is i2c's: the server raises it on a bus
channel and the client answers with SlaveReceive. The MCTP server raises no
USER signal, and its drive_pending is a deferred channel_respond rather than a
wake.

The FD clears the signal when it answers QueryStatus, and raises it again on the
next state change. The orchestrator has no call to clear its own signal:
pw_kernel's one USER call, `object_set_peer_user_signal`, acts on the peer. i2c
does the same thing, the server raises on its bus channel and clears it when it
answers the client's SlaveReceive (the `should_wake` raise and the
`SlaveReceive` arm in services/i2c/server-runtime/src/lib.rs).

The FD clears first, before it reads out the state it returns. In one thread the
order cannot matter, since nothing can change between the two. It is the order
that stays right if the loop ever yields mid-reply, and it is what i2c does.

Events that trigger a nudge:
- Offer ready (UA sent UpdateComponent, FD has target + total)
- Verify pending (transfer complete, FD waiting for PerformVerify)
- Apply pending (verify complete, FD waiting for PerformApply)
- Activation decision needed (apply complete)
- Activated (FdOps::activate done, boot preference set, FD back in Idle)
- SVN commit requested (UA sent UpdateSecurityRevision)
- Cancelled (UA sent CancelUpdate)
- Idle on timeout (a request the FD sent went unanswered for FD_T1)
- Phase failed (verify or apply failed, FD waiting for the UA to cancel)

The nudge is dataless. The orchestrator always follows up with QueryStatus
to learn what happened. One bit, no framing, no lost
messages. The parentheticals in the diagrams (e.g. "nudge: offer ready")
name the state the orchestrator will find via QueryStatus, not data on the
signal.

## Crate layout

The crates live under services/pldm and all build on the host, so the protocol
path builds and tests without a kernel.

| Crate | Holds |
|---|---|
| pldm_api | wire format, opcodes, error types, the seam trait |
| pldm_server | `dispatch` plus the `FdIpcServer` newtype, pure |
| pldm_gate | the policy the orchestrator applies, `Decision` per phase |
| pldm_client | all the marshalling, generic over a Transport |

Nothing here is kernel-tagged. The kernel side is the shared
`util_ipc::AsyncChannelTransport` on the client and an object_wait /
channel_read / channel_respond loop on the server, which belongs in a runtime
crate of its own. The loopback is `util/service/loopback.rs`, shared with
every service rather than owned here. Keeping the kernel out of every crate in
the table is what lets that loopback run the real client encoders against the
real dispatch with no kernel. It is also the debugging seam for interfacing
problems between the two processes: malformed frames, reserved bits set, an
order with no offer, an op in the wrong phase.

Each side treats the other as untrusted, since they are separate processes.
Every fault, a malformed frame included, comes back as a well-formed rejection
and neither dispatch panics. The FD holds the live PLDM session, so a panic
there loses the update.

## FdOps and IPC services

The table below lists only the callbacks this design routes over IPC; the
trait has more. FdOps callbacks run inside the FD process and make outbound
ServiceCalls
to the services they need. The platform driver trait (linked at build time)
determines the board-specific details of each operation; the actual I/O
goes through the device server. The orchestrator does not sit in any of
these data paths.

The FD does not look up the services it calls. Its channels to the device
server and the crypto service are channel_initiator objects declared in the
board's system.json5 and fixed at build time, so nothing is discovered at
runtime and AcceptOffer carries the staging address only, not a device
identity. Another service the FD calls means another declared channel, not a
lookup. This is also what lets a restarted FD come back without rediscovering
anything: the channel is the same object.

| Callback | ServiceCall to | Purpose |
|---|---|---|
| download_fw_data | device server | Write a firmware chunk to the staging region |
| verify | crypto service | Hash and signature check; crypto reads the staged image directly from the device server |
| apply | device server | Commit the staged image (platform driver trait determines what to write) |
| activate | device server | Set boot preference (platform driver trait determines the operation) |
| cancel_update_component | device server | Abort in-flight operations, discard FD transfer state |

pldm-lib decides when cancel_update_component runs, not this design. It calls
the callback before answering either cancel command, and again when a request
the FD sent goes unanswered for FD_T1, which is the one path where the FD
cancels itself with no UA involvement. That last one leaves the FD idle with a
per-state reason, DownloadTimeout or VerifyTimeout and so on, so the
orchestrator sees it as a nudge and an Idle status rather than a Cancelled one.
Once apply has succeeded the CancelUpdate handler refuses the cancel outright
and the callback never runs, so the session ends at activation or not at all.

The crypto service runs in a separate process because it holds the
verification keys. Verification is a single ServiceCall: the crypto
service owns the full read-hash-check pipeline.

## Open questions

Whether PerformVerify/PerformApply should carry additional data (e.g. a nonce,
a policy token) or just be bare order/refusal signals.

What the orchestrator does when the UA omits SVNDelayedUpdate. The DSP0267
default is an automatic bump during the update, which spends the floor before
any boot is judged and makes revert useless. Either the FD refuses to enable
a non-delayed update for a component that carries a security revision, or the
orchestrator accepts the automatic path and its risk. Refusing needs a
completion code DSP0267 does not have for UpdateComponent, the same problem the
0x22 path solved by picking the nearest capability code. The refusal also has to
land in the UpdateComponent response, not later: by the time the orchestrator
sees the offer over QueryStatus, the FD has already answered the UA.

What the orchestrator does if the UA never sends UpdateSecurityRevision. The
trial confirms, the image runs, and the floor stays where it was, so the
superseded image remains bootable indefinitely. Per DSP0267 this is the UA's
call, and nothing in the protocol says a commit is outstanding:
CapabilitiesDuringUpdate bit 9 in GetFirmwareParameters only says the FD
supports the delayed update at all. Open whether the orchestrator should
surface the outstanding commit itself.

What the UA sees while an out-of-transport sequence runs. The whole sequence
runs inside handle_pending_component, so the responder stays busy until the
0x1F response goes out: a GetStatus sent meanwhile gets no answer, and the
pldm-lib state machine would report Idle anyway, not the phase the sequence has
reached. Open whether the sequence should yield to the responder between phases
and what the UA should see if it does.

What stops an out-of-transport sequence once it is running. CancelUpdate is
answered with NOT_IN_UPDATE_MODE because the FD is not in update mode, and the
FD cannot answer it at all while the callback holds the responder, so the UA
has no way to stop the sequence. Nothing says whether the orchestrator should
be able to.

Whether FdOps callbacks need priv_data for fw_download/verify/apply.

How the orchestrator finds out the FD died. Right now it does not. The
orchestrator sleeps until the FD nudges it, a dead FD never nudges, and there
is no "the other side went away" signal to wait on instead: the set is
READABLE, WRITEABLE, ERROR, JOINABLE, USER and the interrupt bits (the
`Signals` bitflags in pw_kernel/syscall/syscall_defs.rs). That matters because
the staging reservation stays held, from AcceptOffer in-transport and from
before ArmPendingImage out-of-transport, until the activated nudge, a cancel,
or PhaseFailed. It is released by orchestrator code that only runs
when the FD sends something, so nothing releases it until the chip resets.

The plan is to let the FD tell us after the fact. A supervisor restarts it, the
fresh FD nudges, and QueryStatus comes back Idle, or with a fresh offer, while
the orchestrator still believes an update is in flight. That mismatch is the
signal, and the orchestrator then closes the filter and frees staging. The
kernel supports the restart: a process can be terminated, joined and started
again, and the channel survives (`test_object_reset_basic` in
pw_kernel/tests/process_termination). A nudge from the old FD is cleared when
the supervisor joins it, before the restart
(`test_peer_user_signal_cleared_on_terminate`). No `gen` field is needed for
this, because a restarted FD starts Idle and a new UA session cannot get past
OfferPending without an AcceptOffer from the orchestrator.

Two gaps remain. There is no supervisor: nothing in this tree calls
`process_start` or `task_terminate`, and target/ast10x0/erot/system.json5 has
no apps yet. Without one the stale nudge is never cleared either, so the
orchestrator can wake on a nudge from an FD that is already gone. And an FD
that dies and never restarts never nudges, so the orchestrator still needs a
plain timer as a backstop. That timer has to exceed worst-case transfer plus
FD_T1 (120s, `DEFAULT_FD_T1_TIMEOUT` in pldm-interface/src/config.rs) so a live
FD always cancels first. FD_T1 is an idle timer the FD resets on every message,
not a bound on transfer time, so a polling UA can keep a live FD from ever
cancelling; that is harmless here only because staging is already released at
PhaseFailed.

What bounds the wait for an order. A deadline the FD keeps itself, so a wedged
or dead orchestrator does not hold a phase open: the FD gives the orchestrator
a fixed window and treats the expiry as a rejection, failing the phase to the
UA rather than waiting on. The UA sending CancelUpdate is the second backstop,
which the FD answers on its own. FD_T1 bounds neither: it resets on every
message, so a UA that keeps polling would otherwise hold the FD parked
indefinitely.

Whether a corruption runtime scanner should exist as a separate service, and
if so, how it signals the orchestrator (sync or async).

What happens when the crypto service dies or never answers. FdOps::verify
sends one ServiceCall and polls for the completion signal. If that signal never
arrives the FD polls forever, the UA gets no VerifyComplete, and nothing in the
design notices. The FD needs a deadline on that call and a DSP0267 code for the
timeout. The FD's other ServiceCalls to the device server have the same shape
and need the same answer. The mirror case is crypto answering late: CancelUpdate
reaches cancel_update_component, which talks to the device server only, so a
Verdict for the cancelled session can still arrive and must not be read as the
next session's answer. `channel_async_cancel` is the mechanism, and cancelling
before the channel is reused has to be part of that callback. It does not
settle the race where crypto responds just as the cancel goes in: the syscall
returns Unavailable when no transaction is pending, and the docs do not say
whether a response that already landed counts.

How the orchestrator opens and closes the SMC write filter: an IPC op on
the device server (which already manages the SPI flash), or a register it
writes directly. Currently the diagrams show it as an orchestrator note at
AcceptOffer and Activate/Cancel.

## Gaps against the tree

Where the code today does something other than this design. Each one is work
to do, not a decision taken back.

The FD blocks inside the FdOps callback instead of returning at 0%. It raises
the USER signal on the orchestrator and then serves only the orchestrator
channel until a decision lands, so the MCTP responder is not polled and
CancelUpdate cannot be answered mid-phase. The gate has to move into the
polled callback the way the Perform gates section describes.

A rejected offer comes back as CompNotSupported in the UpdateComponent
response, because the FD consults the gate before answering. That only works
while the callback blocks; with the polled gate this design specifies, the
response has gone out by then, which is why the offer rejection ends with
TransferComplete and FdAbortedTransfer here.

The orchestrator polls the FD in a step loop rather than waiting on nudges.
It declares a wait_group object and never adds a handle to it, and it blocks
on the FD channel with a timeout. Nothing else competes for its attention
yet, so the loop is adequate until events (CompromiseDetected, timers) land.

The activation gate runs inside FdOps::activate, so no verdict is stored and
a RejectActivate cannot revoke one. Activation reporting assumes the verdict
is taken at apply-complete and answered from storage. A refused activation
also answers the UA with the generic PLDM error instead of INCOMPLETE_UPDATE,
which the RejectActivate op specifies.

A refusal reason cannot travel from the FD to the orchestrator. RejectReason
rides only on a Reject* request the orchestrator sends, and the response
header has no field for one. Adding it is a wire change.

The gate type has no Reject variants, and the only policy never refuses, so
the refusal half of the gatekeeper has no implementation to exercise.

ArmPendingImage and the whole out-of-transport path do not exist.
handle_pending_component returns NotPermitted everywhere.

There is no device server and no crypto server. The FD holds the flash
mappings and programs flash in-process, and verify reads it back in-process
with the signature check stubbed. ServiceCall is the shape the design needs;
what exists is util_ipc::AsyncTransaction with no device server behind it.

There is no production composition: the eRoT board has no apps declared, so
every process above runs only in the QEMU integration test.

# pldm-notify-api

The wire the PLDM service uses to tell the orchestrator that something has
happened. Contract only, no transport and no state, so both processes depend on
it and it builds and tests on the host.

PLDM initiates and the orchestrator handles, because PLDM is where these things
are known and the orchestrator's loop already waits on several objects.

One op today, `NotifyOp::UpdateRequested`, and two answers, `Response::Accepted`
and `Response::Rejected`. All carry no payload, so a frame is 4 bytes each way:
`[op][len][reserved:2]` in, `[code][len][reserved:2]` back.

To add a notification, give it the next `NotifyOp` discriminant. If it carries
data, put the length in `len` and raise `MAX_REQUEST_SIZE`; `len` is in the
header now so that does not change the frame format. The opcode space is flat
and nothing in it is specific to firmware update, so any PLDM type can take a
range. Reserved fields are zero and the decoder rejects a frame that sets them,
so they can be given a meaning later.

`Response::Accepted` lets the transfer proceed. `Response::Rejected` tells the
PLDM service that the orchestrator will not act on this request (already
updating, recovering, locked, or policy). The verdict is the caller's, not the
adapter's.

The orchestrator-side handler is `services/orchestrator/adapters/pldm`.

Run the tests with:

    bazel test //services/pldm/notify-api:pldm_notify_api_test

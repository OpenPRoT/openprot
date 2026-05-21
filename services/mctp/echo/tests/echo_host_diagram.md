# MCTP Echo Host Test Sequence

Source test: `echo_host.rs` (`echo_path_roundtrip_via_stack_and_echo_helper`)

```mermaid
sequenceDiagram
    autonumber
    participant T as Test Harness
    participant SB as Server B (EID 42)
    participant StB as Stack B
    participant ReqB as Req Channel B->8
    participant BufB as B Packet Buffer
    participant SA as Server A (EID 8)
    participant StA as Stack A
    participant LA as Listener A (msg_type=1)
    participant BufA as A Packet Buffer

    T->>SB: Construct Server::new(EID 42)
    T->>SA: Construct Server::new(EID 8)
    T->>StB: Stack::new(client_b)
    T->>StA: Stack::new(client_a)

    T->>StA: prepare_listener()
    StA->>SA: set_eid(8)
    StA->>SA: listener(msg_type=1, timeout=0)
    SA-->>LA: listener handle

    T->>StB: req(remote_eid=8, timeout=0)
    StB-->>ReqB: request channel
    T->>ReqB: send(msg_type=1, payload="echo from host test")
    ReqB->>SB: send request
    SB->>BufB: enqueue outbound packets

    T->>SA: transfer(BufB -> SA.inbound)
    T->>BufB: clear()

    T->>LA: echo_once(listener_a, echo_buf)
    LA->>SA: recv(listener)
    SA-->>LA: (meta, msg, resp_channel)
    LA->>SA: resp_channel.send(msg)
    SA->>BufA: enqueue response packets

    T->>SB: transfer(BufA -> SB.inbound)
    T->>BufA: clear()

    T->>ReqB: recv(resp_buf)
    ReqB->>SB: recv response
    SB-->>ReqB: (meta, payload)

    T->>T: assert meta.msg_type == 1
    T->>T: assert meta.remote_eid == 8
    T->>T: assert response == original payload
```

## Notes

- `prepare_listener()` configures responder identity (`EID=8`) and opens a listener for `ECHO_MSG_TYPE=1`.
- Packet movement between endpoints is explicit in this host test via `transfer(...)` and in-memory buffers (`buf_b`, `buf_a`).
- `echo_once(...)` is one receive/send cycle: `listener.recv(...)` followed by `resp.send(msg)`.

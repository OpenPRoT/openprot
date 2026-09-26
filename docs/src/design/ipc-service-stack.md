# IPC service stack

How a service request travels from a client, through the kernel, to a
server and back. Two paths: the production kernel channel path and the
in-process loopback for host tests.

## Production path (kernel channel)

```mermaid
sequenceDiagram
    participant C as Client<br/>(event loop)
    participant CT as AsyncChannelTransport<br/>(util/ipc)
    participant AT as AsyncTransaction<br/>(util/ipc)
    participant K as pw_kernel<br/>(channel)
    participant S as Server process
    participant D as dispatch()<br/>(ipc-server)
    participant H as FdHandler

    Note over C,H: Request

    C->>CT: start(req: &[u8])
    CT->>CT: copy req into 'static send buf
    CT->>AT: start(send, send_len, recv)
    AT->>K: channel_async_transact(send_ptr, recv_ptr)
    K-->>C: returns immediately

    Note over C: event loop parks on<br/>WaitGroup (READABLE)

    K->>S: delivers request bytes
    S->>D: dispatch(request, response)
    D->>H: accept_offer() / query_status() / ...
    H-->>D: Ok(()) or Err(ResponseCode)
    D->>D: encode response frame
    D-->>S: response length
    S->>K: channel_respond(response)

    Note over C,H: Response

    K-->>C: READABLE signal fires
    C->>CT: poll(resp: &mut [u8])
    CT->>AT: try_recv()
    AT->>K: channel_async_transact_complete()
    K-->>AT: Ok(len)
    AT-->>CT: Completion { len, send, recv }
    CT->>CT: copy recv[..len] into resp
    CT-->>C: Ok(Some(len))
    C->>C: decode resp[..len]
```

## Loopback path (host tests)

A loopback answers on the first poll, so it cannot test a client's
not-ready path. `Delayed` wraps any transport and returns `Ok(None)` for a
set number of polls before forwarding, which is how a host test reaches that
path.

```mermaid
sequenceDiagram
    participant C as Client<br/>(test code)
    participant L as Loopback&lt;D, N&gt;<br/>(util/service)
    participant D as dispatch()<br/>(ipc-server)
    participant H as FdHandler

    C->>L: start(req: &[u8])
    L->>D: dispatch(req, held_buf)
    D->>H: accept_offer() / query_status() / ...
    H-->>D: Ok(()) or Err(ResponseCode)
    D-->>L: response length
    Note over L: response stored in held_buf,<br/>ready immediately

    C->>L: poll(resp: &mut [u8])
    L->>L: copy held_buf[..len] into resp
    L-->>C: Ok(Some(len))
    C->>C: decode resp[..len]
```

## Layer map

```mermaid
graph TD
    subgraph "Client process"
        CL[Service client code]
        CT[AsyncChannelTransport]
        AT[AsyncTransaction]
    end

    subgraph "Kernel"
        CH[pw_kernel channel]
    end

    subgraph "Server process"
        SV[Server main loop]
        DS[service dispatch]
        FH["handler (e.g. FdHandler)"]
    end

    subgraph "Traits (util/service)"
        TR["AsyncTransport trait"]
        DI["Dispatch trait"]
    end

    CL -->|"start/poll/cancel"| CT
    CT -->|"implements"| TR
    CT -->|"owns 'static bufs"| AT
    AT -->|"unsafe syscalls"| CH
    CH -->|"delivers frames"| SV
    SV -->|"request bytes"| DS
    DS -->|"implements"| DI
    DS -->|"decoded opcodes"| FH

    style TR fill:#f0f0f0,stroke:#666
    style DI fill:#f0f0f0,stroke:#666
```

## Buffer copies

The async path copies twice per direction:

| Step | What | Where |
|------|------|-------|
| 1 | req slice into 'static send buf | `AsyncChannelTransport::start()` |
| 2 | send buf to server process | kernel internal |
| 3 | server response to 'static recv buf | kernel internal |
| 4 | recv buf into caller's resp slice | `AsyncChannelTransport::poll()` |

Copies 1 and 4 exist because the kernel needs `'static` buffers (it holds
raw pointers for the duration of the transaction), but callers work with
ordinary stack slices. The loopback skips steps 2 and 3 since everything
runs in one process.

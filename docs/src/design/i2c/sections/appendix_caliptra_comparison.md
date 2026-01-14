# I2C Target Support: Hubris vs Caliptra-MCU

## Subscriber Model

| Aspect | Hubris (OpenPRoT) | Caliptra-MCU (Tock) |
|--------|-------------------|---------------------|
| Subscribers | Single task | Single process |
| Notification | sys_post to one task | Upcall to one process |
| Buffer ownership | One lease holder | One grant region |

Both are single-subscriber models. Only one client can register to receive target data at a time.

## Why Single Subscriber?

For MCTP over I2C, single-subscriber is correct:

- One MCTP stack owns the I2C target address
- MCTP handles demultiplexing to upper protocols (SPDM, PLDM, etc.)
- No contention for incoming packets

```
I2C Bus
    │
    ▼
┌─────────────┐
│ I2C Driver  │  ← single target address
└──────┬──────┘
       │ single notification
       ▼
┌─────────────┐
│ MCTP Stack  │  ← single subscriber
└──────┬──────┘
       │ demux by message type
    ┌──┴──┐
    ▼     ▼
  SPDM   PLDM
```

## Key Difference

| Aspect | Hubris (OpenPRoT) | Caliptra-MCU (Tock) |
|--------|-------------------|---------------------|
| Registration | Build-time (task-slots) | Runtime (subscribe syscall) |

Hubris wires the subscriber at compile time. Tock allows runtime subscription but still enforces one subscriber per driver instance.

## Backpressure: Slow Subscriber

If the MCTP task/process can't keep up:

| Scenario | Hubris | Tock |
|----------|--------|------|
| Notification | Coalesces (bitmask OR) | Upcall queued |
| Driver buffer | Overwrites or drops | Overwrites or drops |
| Hardware FIFO full | NACK or clock stretch | NACK or clock stretch |

**Coalescing**: If multiple I2C messages arrive before the subscriber task wakes up, Hubris merges the notifications via bitwise OR. The task wakes once with the combined notification mask, not once per message. This means the task must call `get_slave_message()` in a loop until empty to retrieve all pending messages.

Neither guarantees delivery if subscriber is slow. Both rely on:

1. Hardware FIFO depth (typically 1-4 packets)
2. Driver copying to software buffer before next packet
3. Subscriber draining buffer before overflow

MCTP handles this at protocol level—the requester retries on timeout. The I2C layer just does best-effort delivery with hardware flow control (NACK/clock stretch) as last resort.

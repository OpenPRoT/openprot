# Thread priorities

How `pw_kernel` schedules threads, and the priority scheme OpenPRoT images
should follow for servers and their clients. This page describes the pinned
Pigweed's behaviour, read from `pw_kernel/kernel/scheduler/` and
`pw_kernel/kernel/object/channel.rs`; check it again when the pin moves.

## How the scheduler behaves

- **Strict priority, 32 levels.** `Level0` is the idle level and a higher number
  is more important. A thread with no `priority` in its `system.json5` runs at
  `Level15`.
- **Round-robin within a level.** The scheduler tick puts the running thread at
  the back of its level's queue. A thread that was preempted goes to the front.
  A thread that was woken goes to the back. The tick is
  `KernelConfigInterface::SCHEDULER_TICK_HZ`, 100 Hz (10 ms) unless a target
  overrides it.
- **A wake preempts only a lower level.** Waking a thread reschedules at once,
  but the woken thread runs only if its level is strictly higher than the running
  thread's. At the same or a lower level the running thread keeps going until it
  blocks or the next tick rotates it out.
- **No priority inheritance.** Nothing raises a thread's level while it holds up
  a higher one.
- **Channels do not change priorities.** A transaction raises `READABLE` on the
  handler and blocks the initiator, and the reply wakes the initiator. See
  [`pw_kernel` IPC](./pw-kernel-ipc.md).

## Why it matters

1. **Latency of interrupt-driven servers.** If an interrupt wakes a server that
   shares a level with the running thread, the server waits for that thread to
   block or for the next tick, up to 10 ms at the default. A server that drains a
   hardware FIFO, such as the I3C server (256-byte TTI receive FIFO), can lose
   data in that time. At a higher level the wake preempts at once.
2. **Unbounded priority inversion.** A client blocked on a server whose level is
   below another runnable thread's can be starved by a thread whose level lies
   between the two, and nothing boosts the server. A server at or above the
   level of every client rules that out.
3. **Deadlock is a property of the call graph.** A cycle of blocking calls
   deadlocks at any priority. The images avoid cycles by using a signal where a
   call would loop: the I3C server raises `USER` on the MCTP server's channel
   and does not call it. Levels that rise along every IPC path make the absence
   of a cycle easy to check.

## The scheme

- A server's level is at or above the level of each of its clients. Levels
  increase along every IPC path, from application to service to hardware server.
- A server that an interrupt wakes, and that must run promptly, takes the
  highest level of the image.
- Applications take the lowest.
- Do not use `Level0`; it is the idle level.
- Break a call cycle with a signal, not with a call.

## Example: MCTP over I3C

`target/veer/tests/mctp_i3c/system.json5.tmpl`:

| Thread | Level | Role |
|---|---|---|
| `i3c_server_thread` | `Level20` | Serves MCTP and handles the I3C interrupt that drains the receive FIFO. |
| `mctp_server_thread` | `Level15` | Serves the echo and control apps, calls the I3C server. |
| `mctp_echo_thread`, `mctp_control_thread` | `Level10` | Applications. |

Every call goes from a lower level to a higher one (echo, then MCTP server, then
I3C server). The priority is a property of the thread object:

```json5
{
  type: "thread",
  name: "i3c_server_thread",
  kernel_stack_size_bytes: 2048,
  priority: "Level20",
},
```

## When it can be left at the default

Where only one thread is runnable at a time, a plain call chain works at any
level: a client blocks as it calls its server, so the server runs next. Simple
single-client test images are typically like that. Set the levels anyway when you add a
second runnable thread, an interrupt-driven server, or a service shared by
several clients.

## Not yet in place

Only the MCTP over I3C test image sets priorities. Other images with a server
leave every thread at the default level.

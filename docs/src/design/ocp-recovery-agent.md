# OCP Recovery: the eRoT as Recovery Agent

On this page the eRoT is the Recovery Agent and a downstream device is
the Device. The downstream device serves the OCP Recovery registers, and
the eRoT reads the registers, pushes the image and requests the
activation. [OCP Recovery Streaming Boot](./ocp-recovery.md) describes
the opposite assignment, where the eRoT serves the registers and another
component on the platform is the Recovery Agent.

The role applies to a downstream device that serves OCP Recovery and
boots from an image a Recovery Agent streams, for example a CPU or a
NIC. Such a downstream device enters recovery mode after it failed to
boot from flash, or on every boot when it holds no image in flash. The
eRoT runs the same sequence in both cases. For a downstream device whose
boot flash the eRoT writes, the orchestrator restores the image through
the `Recovery` capability, and the eRoT sends no OCP Recovery command.

The register value names on this page follow the variants of
`DeviceStatus` and `RecoveryStatus` in `hal/blocking/ocp_recovery`, the
HAL for firmware on the Device side. Every value on this page follows
OCP Secure Firmware Recovery 1.1.

The transport is out of scope here. The eRoT sends each command as a
block read or a block write over the bus that connects the eRoT to the
downstream device.

## When the role runs

A downstream device in reset serves no registers. The eRoT starts the
Recovery Agent sequence for a downstream device after the eRoT releases
that downstream device from reset through `BootControl::release`.

## The Recovery Agent sequence

The sequence follows the Recovery Agent in the Caliptra MCU SW BMC
emulator (`caliptra-mcu-sw/emulator/bmc/src/recovery.rs`). The eRoT runs
the sequence as one flow of reads and writes, and the diagram shows each
step and each poll as a state. Each state names the register the eRoT
reads next, and the value the eRoT reads decides the transition.

```mermaid
stateDiagram-v2
    [*] --> ReadProtCap
    ReadProtCap --> ReadDeviceStatus: PROT_CAP advertises device status (bit 4)
    ReadProtCap --> Done: PROT_CAP does not advertise device status
    ReadDeviceStatus --> Done: DEVICE_STATUS reads Healthy (0x1)
    ReadDeviceStatus --> WaitForRecoveryStatus: DEVICE_STATUS reads RecoveryMode (0x3),<br/>write RECOVERY_CTRL with activate idle (0x0)
    WaitForRecoveryStatus --> WaitForRecoveryPending: RECOVERY_STATUS reads AwaitingImage (0x1),<br/>push the image the recovery image index names
    WaitForRecoveryPending --> Activate: DEVICE_STATUS reads RecoveryPending (0x4),<br/>write RECOVERY_CTRL with activate 0xf
    Activate --> WaitForRecoveryStatus: DEVICE_STATUS reads RecoveryMode (0x3)
    Activate --> Done: DEVICE_STATUS reads Healthy (0x1)<br/>or RunningRecoveryImage (0x5)
    Done --> [*]
```

The Recovery Agent starts every transfer, so the eRoT polls. A state
repeats its read until the register value matches a transition.

For the push in `WaitForRecoveryStatus`, the eRoT writes
`INDIRECT_FIFO_CTRL` with the image size, then writes
`INDIRECT_FIFO_DATA` in chunks the size `INDIRECT_FIFO_STATUS`
advertises. The eRoT pushes only through the FIFO (`0x2d` to `0x2f`),
which a Caliptra subsystem serves, and sends no indirect memory window
command (`0x29` to `0x2b`). The emulator Recovery Agent checks
`PROT_CAP` bit 4 only. OCP Secure Firmware Recovery 1.1 allows the FIFO
commands only where bit 12 is set, so the eRoT checks bit 12 in
`ReadProtCap` as well and pushes no image where bit 12 is clear.

A downstream device that boots from several images returns to
`RecoveryMode` after an activation and reports the next recovery image
index in `RECOVERY_STATUS`. The transition from `Activate` back to
`WaitForRecoveryStatus` serves that downstream device, and the eRoT
pushes the image the index names. A Caliptra subsystem requests the
Caliptra firmware at index 0, the SoC manifest at index 1 and the MCU
firmware at index 2.

## Open questions

- Does the role sit behind `Recovery::restore`? The contract of
  `restore` says that the orchestrator holds the managed device in reset
  before the call, and that `restore` does not check the image. A
  downstream device in reset serves no registers, and a downstream
  device that serves OCP Recovery reports a verdict on the pushed image
  in `RECOVERY_STATUS`. An implementor of `Recovery` would have to
  release the downstream device from reset inside `restore` and return
  `Ok(Restored)` whatever verdict `RECOVERY_STATUS` reports. The
  alternative is a separate capability through which the orchestrator
  drives the role.
- How does the eRoT get the images? The images come from the BMC or a
  rack manager, and the eRoT needs every image the downstream device
  requests by index. The eRoT can store the images in flash ahead of a
  recovery, where the flash size bounds them, or fetch each image when
  the downstream device requests the image. The protocol that carries
  an image to the eRoT is open as well.
- Does the eRoT verify an image before pushing it? The downstream device
  verifies the image either way. A verify on the eRoT keeps a corrupt
  stored image off the bus and costs the eRoT a verifier for an image
  format the downstream device defines.
- Which transport carries the commands? OCP Secure Firmware Recovery 1.1
  defines the commands over SMBus, I3C and USB, and a Caliptra subsystem
  serves the registers over I3C.
- When does the sequence end for a Caliptra subsystem? The sequence ends
  on a `DEVICE_STATUS` of `Healthy` (0x1) or `RunningRecoveryImage`
  (0x5). After the download of a non-encrypted MCU firmware, the
  Caliptra runtime firmware on caliptra-sw `main` writes neither value,
  because the MCU can request additional images. At the end of cold
  boot, the MCU ROM in caliptra-mcu-sw writes a `RECOVERY_STATUS` of
  `Successful` (0x3) and a `DEVICE_STATUS` of `StatusPending` (0x0) by
  default. With the ROM parameter `recovery_status_open`, the MCU ROM
  writes a `RECOVERY_STATUS` of `BootingImage` (0x2) and no
  `DEVICE_STATUS`. Does the eRoT read `RECOVERY_STATUS` in `Activate`
  and end the sequence on `Successful`, and on a timeout while
  `RECOVERY_STATUS` reads `BootingImage`?
- Which failure paths does the sequence need? The emulator Recovery
  Agent has no transition for a `DEVICE_STATUS` of `BootFailure` (0xe)
  or `FatalError` (0xf), for a `RECOVERY_STATUS` that reports a failed
  or refused image, for a latched protocol error, or for a downstream
  device that stops answering. The sequence needs an end state for a
  failed recovery, a poll period, a timeout per state and a retry
  count, and a decision on whether the eRoT puts a downstream device
  that failed recovery back in reset.
- What does the eRoT do when the downstream device answers a push with
  a NACK? The eRoT can send the same write again, start the image again
  with an `INDIRECT_FIFO_CTRL` reset, or end the recovery as failed.
- What does the eRoT do when the recovery image index names an image
  the eRoT does not hold? The emulator Recovery Agent returns an error
  and stays in `WaitForRecoveryStatus`.

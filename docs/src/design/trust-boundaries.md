# Trust Boundaries

This page explains what OpenPRoT trusts and what it does not. It is
written for developers working on the orchestrator and its services. For
the formal threat analysis (assets, attacker profiles, mitigations), see
[Threat Model](../specification/threat_model.md).

## The one rule

Everything inside the RoT chip is trusted. Everything outside is not.

The chip boundary is the trust boundary. Code running inside the chip
(the orchestrator, the crypto service, the flash service, the PLDM
service) all trust each other. They share the same hardware, the same
boot chain, and the same update path. If any one of them is compromised,
the whole RoT is compromised, so treating them as mutually suspicious
would add complexity without adding security.

## What this means in practice

**IPC between services needs no authentication.** The kernel enforces
which processes can open which channels, and that is enough. We do not
sign or add integrity tags to internal messages.

**Flash contents are untrusted.** Firmware images on flash were written
by something outside the chip (a BMC, a host, a debug tool). The
orchestrator treats every image as potentially tampered until the crypto
service says otherwise.

**Everything arriving over MCTP is adversarial.** The PLDM service
validates every field before acting on it. A malformed or unexpected
message is dropped, not trusted.

**The BMC is not trusted.** An update is accepted because it passes
signature verification, not because the BMC sent it.

For threats the orchestrator does not defend against (compromised RoT
hardware, BMC denial of service), see the
[Threat Model](../specification/threat_model.md#threat-modeling).

## How this guides code

When you are writing code inside the RoT:

- Do not add authentication, signatures, or integrity tags to internal
  IPC. It is wasted work.
- Validate everything that crosses the chip boundary: PLDM fields,
  flash contents, SPDM payloads.
- Do not assume flash contains what you last wrote. The flash medium is
  reachable from outside the chip, so its contents can change after the
  write returns.
- Trust results from internal services. If the crypto service says
  "authenticated" or the flash service says "write succeeded", that
  answer is final. Readback exists to catch flash corruption, not to
  second-guess a service inside the chip.

<!-- Licensed under the Apache-2.0 license -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# orchestrator shell (`openprot_orchestrator_shell`)

The effect-executing layer around the orchestrator state machine. `Shell`
implements the SM's `Platform` seam with one method per `Effect`, each doc
comment stating its obligation from the platform-boundary contract
(`docs/src/design/orchestrator/orchestrator-model.md` §6).

Executors are filled in one at a time; the rest return
`ShellError::NotImplemented`, and the SM fail-closes on any effect the shell
cannot yet perform. The shell is board-blind: everything device-specific
arrives through the seams in `board.rs` — `ImageSource` (interposed flash, a
PLDM/MCTP transfer, a test double) and `Verifier` (what counts as authentic).
Verdicts and other executor-produced events flow back to the SM through
`Shell::take_event`.

<!-- Licensed under the Apache-2.0 license -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# orchestrator-hal-adapters

HAL-backed adapters for the capability traits: `HalBootControl` drives
`BootControl` over a `ResetControl` line, `GpioBootMonitor` reads `BootMonitor`
off a `GpioPort`. Kept separate so contracts never pull in the HAL.

# AST10x0 I3C init smoke test

Mirrors `tests/peripherals/i2c/i2c_init`. Brings up the I3C controller via the
`ast10x0_peripherals::i3c` driver (ported from `aspeed-rust/src/i3c/`; see
`target/ast10x0/peripherals/i3c/plans/goal.md`) and verifies the init-time
hardware state.

What runs where:

- **Build + `no_panics_test`** (`--config=virt_ast10x0`, kernel tag): the binary
  must compile and be panic-free. This is the CI gate under QEMU.
- **`i3c_init_test`** (`hardware` tag): executes the init/register-verify on real
  hardware only — `target_compatible_with` marks it incompatible when
  `qemu_enabled`, because QEMU `ast1030-evb` does not model the I3C pads/PHY.

Pass/fail is signalled by writing `TEST_RESULT:PASS` / `TEST_RESULT:FAIL` to the
console, the same sentinel protocol the I2C tests use.

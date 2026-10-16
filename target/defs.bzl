# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Shared definitions for crates that run on any pw_kernel target."""

# Compatible with every pw_kernel target defined in this repo; incompatible
# everywhere else (e.g. host). Use this instead of a single target's
# TARGET_COMPATIBLE_WITH for crates that don't depend on target-specific code.
KERNEL_TARGET_COMPATIBLE_WITH = select({
    "//target/ast10x0:target_ast10x0": [],
    "//target/veer:target_veer": [],
    "//target/earlgrey:target_earlgrey": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

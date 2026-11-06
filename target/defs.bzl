# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Shared definitions for crates that run on any pw_kernel target."""

# Compatible with every bare-metal (pw_kernel) target; incompatible on hosts.
# SoC-agnostic on purpose: SoC packages are moving to nested modules, so shared
# crates must not name any individual SoC's constraint values.
KERNEL_TARGET_COMPATIBLE_WITH = select({
    "@platforms//os:none": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

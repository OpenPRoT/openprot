# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Common definitions used by all ast10x0 targets."""

TARGET_COMPATIBLE_WITH = select({
    "//target/ast10x0:target_ast10x0": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

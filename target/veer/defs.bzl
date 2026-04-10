# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0


"""Common definitions used by all veer targets.
"""

TARGET_COMPATIBLE_WITH = select({
    "//target/veer:target_veer": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

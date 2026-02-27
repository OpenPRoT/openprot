# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0
"""Common definitions used by all earlgrey targets.
"""

TARGET_COMPATIBLE_WITH = select({
    "//target/earlgrey:target_earlgrey": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

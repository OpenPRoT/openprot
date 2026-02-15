# Licensed under the Apache-2.0 license
"""Common definitions used by all ast1060-evb targets."""

TARGET_COMPATIBLE_WITH = select({
    "//target/ast1060-evb:target_ast1060_evb": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

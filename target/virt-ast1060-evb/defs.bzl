# Licensed under the Apache-2.0 license

TARGET_COMPATIBLE_WITH = select({
    "//target/virt-ast1060-evb:target_virt_ast1060_evb": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

# Licensed under the Apache-2.0 license

TARGET_COMPATIBLE_WITH = select({
    "//target/ast1060:target_ast1060": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

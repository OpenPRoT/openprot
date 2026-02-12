# Licensed under the Apache-2.0 license

TARGET_COMPATIBLE_WITH = select({
    "//target/ast1030:target_ast1030": [],
    "//conditions:default": ["@platforms//:incompatible"],
})

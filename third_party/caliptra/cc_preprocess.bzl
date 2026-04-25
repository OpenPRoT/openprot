# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

load("@bazel_tools//tools/cpp:toolchain_utils.bzl", "find_cpp_toolchain")

def _cpp_preprocess_impl(ctx):
    cc_toolchain = find_cpp_toolchain(ctx)
    feature_configuration = cc_common.configure_features(
        ctx = ctx,
        cc_toolchain = cc_toolchain,
        requested_features = ctx.features,
        unsupported_features = ctx.disabled_features,
    )

    # Use the C compiler with -E as it is the most reliable way to preprocess
    c_compiler = cc_common.get_tool_for_action(
        feature_configuration = feature_configuration,
        action_name = "c-compile",
    )

    args = ctx.actions.args()
    args.add("-E")
    args.add(ctx.file.src.path)
    args.add("-o", ctx.outputs.out.path)

    ctx.actions.run(
        outputs = [ctx.outputs.out],
        inputs = [ctx.file.src] + cc_toolchain.all_files.to_list(),
        executable = c_compiler,
        arguments = [args],
        mnemonic = "CppPreprocess",
        use_default_shell_env = False,
    )

cc_preprocess = rule(
    implementation = _cpp_preprocess_impl,
    attrs = {
        "out": attr.output(mandatory = True),
        "src": attr.label(allow_single_file = True, mandatory = True),
        "_cc_toolchain": attr.label(default = Label("@bazel_tools//tools/cpp:current_cc_toolchain")),
    },
    toolchains = ["@bazel_tools//tools/cpp:toolchain_type"],
    fragments = ["cpp"],
    incompatible_use_toolchain_transition = True,
)

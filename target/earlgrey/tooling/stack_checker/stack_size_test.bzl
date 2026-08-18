# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Rule to validate static stack sizes in Rust binaries against system config budgets."""

load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")

def _rust_stack_size_test_impl(ctx):
    info = ctx.attr.binary[SystemImageInfo] if SystemImageInfo in ctx.attr.binary else None
    if info:
        if ctx.attr.app:
            binary = info.apps[ctx.attr.app]
        else:
            binary = info.elf
    else:
        binary = ctx.files.binary[0]

    system_config = ctx.file.system_config
    app_name = ctx.attr.app if ctx.attr.app else ctx.attr.app_name

    run_script = ctx.actions.declare_file("%s.sh" % ctx.label.name)

    script_content = """#!/bin/bash
set -e
{checker} --binary {binary} --config {config} --app-name {app_name} --headroom {headroom} --llvm-readobj {llvm_readobj} --llvm-objdump {llvm_objdump} "$@"
""".format(
        checker = ctx.executable._checker_tool.short_path,
        binary = binary.short_path,
        config = system_config.short_path,
        app_name = app_name,
        headroom = ctx.attr.headroom,
        llvm_readobj = ctx.file._llvm_readobj.short_path,
        llvm_objdump = ctx.file._llvm_objdump.short_path,
    )

    ctx.actions.write(run_script, script_content, is_executable = True)

    runfiles = ctx.runfiles(
        files = [
            binary,
            system_config,
            ctx.file._llvm_readobj,
            ctx.file._llvm_objdump,
        ],
        transitive_files = ctx.attr._checker_tool[DefaultInfo].default_runfiles.files,
    )

    return [
        DefaultInfo(
            executable = run_script,
            runfiles = runfiles,
        ),
    ]

_rust_stack_size_test = rule(
    implementation = _rust_stack_size_test_impl,
    test = True,
    attrs = {
        "app": attr.string(
            default = "",
            doc = "Check an alternate binary from the system image",
        ),
        "app_name": attr.string(
            default = "",
            doc = "Name of the app in system_config",
        ),
        "binary": attr.label(
            mandatory = True,
            providers = [[DefaultInfo], [SystemImageInfo]],
        ),
        "headroom": attr.int(
            default = 0,
            doc = "Interrupt/trap safety buffer in bytes",
        ),
        "system_config": attr.label(
            mandatory = True,
            allow_single_file = True,
        ),
        "_checker_tool": attr.label(
            default = Label("//target/earlgrey/tooling/stack_checker:check_stack_sizes"),
            executable = True,
            cfg = "exec",
        ),
        "_llvm_objdump": attr.label(
            default = Label("@llvm_toolchain//:bin/llvm-objdump"),
            allow_single_file = True,
            cfg = "exec",
        ),
        "_llvm_readobj": attr.label(
            default = Label("@llvm_toolchain//:bin/llvm-readobj"),
            allow_single_file = True,
            cfg = "exec",
        ),
    },
    doc = "Check whether the Rust binary stack sizes conform to process allocations.",
)

def rust_stack_size_test(name, binary, system_config, apps = [], headroom = 0, **kwargs):
    """Check whether binary stack sizes conform to system.json5 budgets.

    Args:
        name: Name of the target
        binary: Target to check stack sizes
        system_config: System config file defining allocations
        apps: Alternate binaries to check in a system image binary
        headroom: Safety headroom in bytes (default 0)
        **kwargs: Additional args passed to underlying rule
    """
    if not apps:
        _rust_stack_size_test(
            name = name,
            binary = binary,
            system_config = system_config,
            headroom = headroom,
            **kwargs
        )
    else:
        for app in apps:
            _rust_stack_size_test(
                name = "{}_{}".format(name, app),
                binary = binary,
                app = app,
                app_name = app,
                system_config = system_config,
                headroom = headroom,
                **kwargs
            )

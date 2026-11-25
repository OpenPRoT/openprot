# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Shared rule behind every "render one process's Rust source from system.json5" macro.

`app_regions` (`//util/region`) and `app_pin_claims` (`//target/ast10x0/pins`) both run
Pigweed's `system_generator_bin` with their own jinja template against the same manifest and
compile the result into a `rust_library` — identical mechanics, differing only in which
template, deps, and progress/mnemonic strings apply. `manifest_codegen` is that mechanism;
each caller supplies the parts that differ.
"""

load("@rules_rust//rust:defs.bzl", "rust_library")

def _manifest_codegen_src_impl(ctx):
    output = ctx.actions.declare_file(ctx.attr.name + ".rs")

    args = [
        "--template",
        "app=" + ctx.file.template.path,
        "--config",
        ctx.file.system_config.path,
        "--output",
        output.path,
        "render-app-template",
    ]

    if ctx.attr.app_name:
        args.extend(["--app-name", ctx.attr.app_name])

    if ctx.attr.process_name:
        args.extend(["--process-name", ctx.attr.process_name])

    ctx.actions.run(
        inputs = ctx.files.system_config + [ctx.file.template],
        outputs = [output],
        executable = ctx.executable.system_generator,
        mnemonic = ctx.attr.codegen_mnemonic,
        progress_message = ctx.attr.progress_message % str(ctx.label),
        arguments = args,
    )

    return [DefaultInfo(files = depset([output]))]

_manifest_codegen_src = rule(
    implementation = _manifest_codegen_src_impl,
    attrs = {
        "app_name": attr.string(
            doc = "Name of the application in the configuration file.",
            default = "",
        ),
        "codegen_mnemonic": attr.string(
            doc = "Mnemonic reported for this generation action.",
            mandatory = True,
        ),
        "process_name": attr.string(
            doc = "Name of the process in the configuration file.",
            default = "",
        ),
        "progress_message": attr.string(
            doc = "Progress message template; `%s` is replaced with the target label.",
            mandatory = True,
        ),
        "system_config": attr.label(
            doc = "System config file which defines the system.",
            allow_single_file = True,
        ),
        "system_generator": attr.label(
            executable = True,
            cfg = "exec",
            default = "@pigweed//pw_kernel/tooling/system_generator:system_generator_bin",
        ),
        "template": attr.label(
            doc = "Jinja template rendered against the process's manifest entry.",
            allow_single_file = True,
            mandatory = True,
        ),
    },
    doc = "Render one process's Rust source from the system manifest.",
)

def manifest_codegen(name, template, codegen_mnemonic, progress_message, deps, app_name = "", process_name = "", system_config = None, **kwargs):
    """Generates and compiles one process's Rust source from the system manifest.

    Args:
        name: Name of the generated crate.
        template: Jinja template rendered against the process's manifest entry.
        codegen_mnemonic: Mnemonic reported for the generation action.
        progress_message: Progress message template; `%s` is replaced with the target label.
        deps: Deps forwarded to the generated `rust_library`.
        app_name: Name of the app in the system manifest (single-process apps only).
        process_name: Name of the process in the system manifest.
        system_config: System config file which defines the system.
        **kwargs: forwarded to rust_library; tags also reaches the generator rule.
    """

    # The generator takes these as a clap arg-group: exactly one, never both.
    if bool(app_name) == bool(process_name):
        fail("manifest_codegen requires exactly one of app_name or process_name")

    tags = kwargs.get("tags", [])

    _manifest_codegen_src(
        name = name + ".src",
        app_name = app_name,
        codegen_mnemonic = codegen_mnemonic,
        process_name = process_name,
        progress_message = progress_message,
        system_config = system_config,
        template = template,
        tags = tags,
    )

    rust_library(
        name = name,
        srcs = [":{}.src".format(name)],
        deps = deps,
        **kwargs
    )

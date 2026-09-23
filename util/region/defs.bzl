# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Generates a process's Rust memory mappings from the system manifest.

Runs pigweed's `system_generator_bin` with our own template, so the addresses
in `system.json5` reach Rust without being restated by hand.
"""

load("@rules_rust//rust:defs.bzl", "rust_library")

def _app_regions_src_impl(ctx):
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
        mnemonic = "AppRegionsSrc",
        progress_message = "Generating memory mappings for %s" % ctx.label,
        arguments = args,
    )

    return [DefaultInfo(files = depset([output]))]

_app_regions_src = rule(
    implementation = _app_regions_src_impl,
    attrs = {
        "app_name": attr.string(
            doc = "Name of the application in the configuration file.",
            default = "",
        ),
        "process_name": attr.string(
            doc = "Name of the process in the configuration file.",
            default = "",
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
            doc = "Memory mapping table template.",
            allow_single_file = True,
            default = "//util/region:regions.rs.jinja",
        ),
    },
    doc = "Generate a process's Rust memory mappings.",
)

def app_regions(name, app_name = "", process_name = None, system_config = None, **kwargs):
    """Generates and compiles a process's Rust memory mappings.

    Args:
        name: Name of the generated crate.
        app_name: Name of the app in the system manifest.
        process_name: Name of the process in the system manifest.
        system_config: System config file which defines the system.
        **kwargs: forwarded to rust_library; tags also reaches the generator rule.
    """

    # The generator takes these as a clap arg-group: exactly one, never both.
    if bool(app_name) == bool(process_name):
        fail("app_regions requires exactly one of app_name or process_name")

    tags = kwargs.get("tags", [])

    _app_regions_src(
        name = name + ".src",
        app_name = app_name,
        process_name = process_name if process_name else "",
        system_config = system_config,
        tags = tags,
    )

    rust_library(
        name = name,
        srcs = [":{}.src".format(name)],
        deps = ["//util/region"],
        **kwargs
    )

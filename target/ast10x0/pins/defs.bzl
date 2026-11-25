# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Generates one process's pin claims from the system manifest.

A `pin_claims_<process_name>` string constant in the manifest's `apps[].constants` (the same
`AppConfig.constants` Pigweed already renders into Rust, no schema patch needed) names the pins
that process may drive, comma-separated `pin:capability` pairs (e.g. `scu410_0:gpio`). The
capability tag only matters to the kernel's own boot-time routing (see
`//target/ast10x0/pins:kernel_pin_routes.rs.jinja`) — it picks which `Routes<C>` impl to flush for
a pin that could carry more than one capability; `app_pin_claims` drops the tag and turns the pin
names alone into a `PinClaims` struct and a `take_pin_claims()` mint, handing back exactly those
pins as real, driveable `ast10x0_peripherals::scu::pins` tokens — holding one is the only authority
`into_output`/`into_input` require. `//util/hw_claim:manifest_check` reads the same manifest
constant independently to check for cross-app conflicts — no build-time record is needed for that.
"""

load("@rules_rust//rust:defs.bzl", "rust_library")
load("//util/manifest_codegen:defs.bzl", "manifest_codegen")

def app_pin_claims(name, app_name = "", process_name = "", system_config = None, **kwargs):
    """Generates and compiles one process's pin claims.

    Args:
        name: Name of the generated crate.
        app_name: Name of the app in the system manifest (single-process apps only).
        process_name: Name of the process in the system manifest.
        system_config: System config file which defines the system.
        **kwargs: forwarded to rust_library; tags also reaches the generator rule.
    """
    manifest_codegen(
        name = name,
        template = "//target/ast10x0/pins:pin_claims.rs.jinja",
        codegen_mnemonic = "AppPinClaimsSrc",
        progress_message = "Generating pin claims for %s",
        deps = [
            "//target/ast10x0/peripherals",
        ],
        app_name = app_name,
        process_name = process_name,
        system_config = system_config,
        **kwargs
    )

def _kernel_pin_routes_src_impl(ctx):
    output = ctx.actions.declare_file(ctx.attr.name + ".rs")

    ctx.actions.run(
        inputs = ctx.files.system_config + [ctx.file.template],
        outputs = [output],
        executable = ctx.executable.system_generator,
        mnemonic = "KernelPinRoutesSrc",
        progress_message = "Generating kernel pin routing for %s" % ctx.label,
        arguments = [
            "--template",
            "system=" + ctx.file.template.path,
            "--config",
            ctx.file.system_config.path,
            "--output",
            output.path,
            "render-target-template",
        ],
    )

    return [DefaultInfo(files = depset([output]))]

_kernel_pin_routes_src = rule(
    implementation = _kernel_pin_routes_src_impl,
    attrs = {
        "system_config": attr.label(
            doc = "System config file which defines the system.",
            allow_single_file = True,
            mandatory = True,
        ),
        "system_generator": attr.label(
            executable = True,
            cfg = "exec",
            default = "@pigweed//pw_kernel/tooling/system_generator:system_generator_bin",
        ),
        "template": attr.label(
            doc = "Kernel pin-routing template.",
            allow_single_file = True,
            default = "//target/ast10x0/pins:kernel_pin_routes.rs.jinja",
        ),
    },
    doc = "Generate the kernel's pin-routing function from every app's pin claims.",
)

def kernel_pin_routes(name, system_config, **kwargs):
    """Generates and compiles the kernel's pin-routing function from the whole system manifest.

    Routes exactly the pins at least one app's `pin_claims_<process>` manifest entry names — the
    union across every app in the system — so the kernel's routing list can never drift from
    what apps actually claim to drive. This renders against the whole manifest (`render-target-
    template`, not `render-app-template`), unlike `app_pin_claims`, since routing isn't scoped to
    one process.

    Args:
        name: Name of the generated crate.
        system_config: System config file which defines the system.
        **kwargs: forwarded to rust_library; tags also reaches the generator rule.
    """
    tags = kwargs.get("tags", [])

    _kernel_pin_routes_src(
        name = name + ".src",
        system_config = system_config,
        tags = tags,
    )

    rust_library(
        name = name,
        srcs = [":{}.src".format(name)],
        deps = [
            "//target/ast10x0/peripherals",
        ],
        **kwargs
    )

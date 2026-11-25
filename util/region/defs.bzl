# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Generates a process's Rust memory mappings from the system manifest.

Runs pigweed's `system_generator_bin` with our own template, so the addresses
in `system.json5` reach Rust without being restated by hand.
"""

load("//util/manifest_codegen:defs.bzl", "manifest_codegen")

def app_regions(name, app_name = "", process_name = None, system_config = None, **kwargs):
    """Generates and compiles a process's Rust memory mappings.

    Args:
        name: Name of the generated crate.
        app_name: Name of the app in the system manifest.
        process_name: Name of the process in the system manifest.
        system_config: System config file which defines the system.
        **kwargs: forwarded to rust_library; tags also reaches the generator rule.
    """
    manifest_codegen(
        name = name,
        template = "//util/region:regions.rs.jinja",
        codegen_mnemonic = "AppRegionsSrc",
        progress_message = "Generating memory mappings for %s",
        deps = ["//util/region"],
        app_name = app_name,
        process_name = process_name if process_name else "",
        system_config = system_config,
        **kwargs
    )

# Copyright 2025 The Pigweed Authors
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not
# use this file except in compliance with the License. You may obtain a copy of
# the License at
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
# WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
# License for the specific language governing permissions and limitations under
# the License.
"""A rule for running a pigweed system_image on an opentitan board."""

load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")
load("//target/earlgrey/tooling/signing:defs.bzl", "KeySetInfo", "sign_binary")

def _target_type_transition_impl(_, attr):
    if attr.interface == "hyper310" or attr.interface == "hyper340":
        return {"//target/earlgrey:target_type": "fpga"}
    if attr.interface == "verilator":
        return {"//target/earlgrey:target_type": "verilator"}

    return {"//target/earlgrey:target_type": "silicon"}

_target_type_transition = transition(
    implementation = _target_type_transition_impl,
    inputs = [],
    outputs = ["//target/earlgrey:target_type"],
)

def _opentitan_runner_impl(ctx):
    system_image_info = ctx.attr.target[0][SystemImageInfo]
    elf_file = system_image_info.elf
    bin_file = system_image_info.bin
    runner = ctx.executable._opentitan_runner

    if ctx.attr.ecdsa_key:
        result = sign_binary(
            ctx,
            ctx.executable._opentitantool,
            bin = bin_file,
            basename = ctx.attr.name,
        )
        bin_file = result["signed"]

    optional_args = ""
    if hasattr(ctx.attr, "exit_success") and ctx.attr.exit_success:
        optional_args += " --exit-success='{}'".format(ctx.attr.exit_success)
    if hasattr(ctx.attr, "exit_failure") and ctx.attr.exit_failure:
        optional_args += " --exit-failure='{}'".format(ctx.attr.exit_failure)

    if ctx.attr.interface == "hyper310" or ctx.attr.interface == "hyper340":
        load_bitstream = "--load-bitstream"
        mechanism = "--mechanism=bootstrap"
    elif ctx.attr.interface == "teacup":
        load_bitstream = ""
        mechanism = "--mechanism=rescue"
    else:
        load_bitstream = ""
        mechanism = ""

    run_script = ctx.actions.declare_file(ctx.attr.name + ".sh")
    ctx.actions.write(
        output = run_script,
        is_executable = True,
        content = """#!/bin/bash
{runner} --interface {interface} {load_bitstream} {mechanism} --elf {elf} --bin {bin} {optional_args}
""".format(
            runner = runner.short_path,
            interface = ctx.attr.interface,
            load_bitstream = load_bitstream,
            mechanism = mechanism,
            elf = elf_file.short_path,
            bin = bin_file.short_path,
            optional_args = optional_args,
        ),
    )

    runner_files_depset = ctx.attr._opentitan_runner[DefaultInfo].default_runfiles.files

    return [DefaultInfo(
        runfiles = ctx.runfiles(
            files = [elf_file, bin_file, runner],
            transitive_files = runner_files_depset,
        ),
        executable = run_script,
    )]

_BASE_ATTRS = {
    "ecdsa_key": attr.label_keyed_string_dict(
        allow_files = True,
        providers = [KeySetInfo],
        doc = "ECDSA public key to validate this image",
    ),
    "interface": attr.string(
        doc = "The interface to use.",
        mandatory = True,
    ),
    "manifest": attr.label(
        allow_single_file = True,
        doc = "A json manifest to apply to the image being signed",
    ),
    "spx_key": attr.label_keyed_string_dict(
        allow_files = True,
        providers = [KeySetInfo],
        doc = "SPX public key to validate this image",
    ),
    "target": attr.label(
        doc = "The system_image target to run.",
        mandatory = True,
        providers = [SystemImageInfo],
        cfg = _target_type_transition,
    ),
    "_opentitan_runner": attr.label(
        executable = True,
        cfg = "exec",
        default = "//target/earlgrey/tooling:opentitan_runner",
    ),
    "_opentitantool": attr.label(
        executable = True,
        allow_single_file = True,
        cfg = "exec",
        default = "@opentitan_devbundle//:opentitantool/opentitantool",
        doc = "opentitantool",
    ),
}

opentitan_runner = rule(
    implementation = _opentitan_runner_impl,
    executable = True,
    attrs = _BASE_ATTRS,
)

opentitan_test = rule(
    implementation = _opentitan_runner_impl,
    test = True,
    attrs = _BASE_ATTRS | {
        "exit_failure": attr.string(
            default = "FAIL: \\d+\\n",
            doc = "The regex to look for in the output to determine failure.",
        ),
        "exit_success": attr.string(
            default = "PASS\\n",
            doc = "The regex to look for in the output to determine success.",
        ),
    },
)

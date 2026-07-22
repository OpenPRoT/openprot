# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

load("@pigweed//pw_kernel/tooling:system_image.bzl", "SystemImageInfo")
load("//target/earlgrey/env:environments.bzl", "TestEnvironmentInfo")
load("//target/earlgrey/tooling/signing:defs.bzl", "KeySetInfo", "sign_binary")

def _target_type_transition_impl(_, attr):
    if attr.interface == "hyper310" or attr.interface == "hyper340":
        return {"//target/earlgrey:target_type": "fpga"}
    if attr.interface == "verilator":
        return {"//target/earlgrey:target_type": "verilator"}
    if attr.interface == "qemu":
        return {"//target/earlgrey:target_type": "qemu"}

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

    opentitantool = ctx.executable._opentitantool

    test_harness = opentitantool
    is_custom_harness = False
    if ctx.attr.test_harness:
        test_harness = ctx.executable.test_harness
        is_custom_harness = True

    if ctx.attr.ecdsa_key:
        result = sign_binary(
            ctx,
            opentitantool,
            bin = bin_file,
            basename = ctx.attr.name,
        )
        bin_file = result["signed"]

    env = ctx.attr.environment[TestEnvironmentInfo]
    if ctx.attr.interface != env.interface:
        fail("interface attribute ({}) does not match environment interface ({})".format(ctx.attr.interface, env.interface))

    clear_bitstream = getattr(ctx.attr, "clear_bitstream", False)
    is_fpga = env.interface in ["hyper310", "hyper340"]
    if clear_bitstream and not is_fpga:
        fail("clear_bitstream is only supported on FPGA platforms (hyper310, hyper340)")

    run_script = ctx.actions.declare_file(ctx.attr.name + ".sh")
    runfiles_list = [elf_file, bin_file, opentitantool]
    if is_custom_harness:
        runfiles_list.append(test_harness)

    harness_runfiles = None
    if is_custom_harness:
        harness_runfiles = ctx.attr.test_harness[DefaultInfo].default_runfiles

    exit_success = ctx.attr.exit_success if hasattr(ctx.attr, "exit_success") else "PASS\\n"
    exit_failure = ctx.attr.exit_failure if hasattr(ctx.attr, "exit_failure") else "FAIL: .+\\n"

    tools = struct(
        flashgen = ctx.attr._flashgen,
        opentitantool = opentitantool,
        elf = elf_file,
    )
    prep = env.prepare(ctx, env, bin_file, tools)
    boot_image = prep.boot_image

    runfiles_list.extend(prep.extra_runfiles)
    for td in ctx.attr.target_data:
        runfiles_list.extend(td[DefaultInfo].files.to_list())
    output_groups = dict(prep.output_groups)

    # Create base_runfiles once runfiles_list is fully populated
    base_runfiles = ctx.runfiles(
        files = runfiles_list,
        transitive_files = env.runfiles,
    )
    if harness_runfiles:
        base_runfiles = base_runfiles.merge(harness_runfiles)

    if env.rom_ext:
        boot_cmd = env.boot_cmd.format(boot_image = boot_image.short_path)
    elif env.boot_cmd:
        boot_cmd = env.boot_cmd.format(firmware = bin_file.short_path)
    else:
        boot_cmd = ""

    flash_path = boot_image.short_path if env.need_flash else ""

    if env.use_custom_runner:
        formatted_runner_args = []
        for arg in env.runner_args:
            formatted_runner_args.append(arg.format(
                flash = flash_path,
                elf = elf_file.short_path,
                exit_success = exit_success,
                exit_failure = exit_failure,
            ))

        ctx.actions.write(
            output = run_script,
            is_executable = True,
            content = """#!/bin/bash
exec {runner} {args}
""".format(
                runner = env.runner.short_path,
                args = " ".join(formatted_runner_args),
            ),
        )

        return [
            DefaultInfo(
                runfiles = base_runfiles,
                executable = run_script,
            ),
            OutputGroupInfo(**output_groups),
        ]

    else:
        setup_cmds = list(env.setup_cmds)
        if clear_bitstream:
            if "transport init" in setup_cmds:
                idx = setup_cmds.index("transport init")
                setup_cmds.insert(idx + 1, "fpga clear-bitstream")
            else:
                setup_cmds.insert(0, "fpga clear-bitstream")

        setup_args_list = []
        for c in setup_cmds:
            setup_args_list.append('--exec="{}"'.format(c))
        if boot_cmd:
            setup_args_list.append(boot_cmd)
        setup_args_str = " ".join(setup_args_list)
        extra_args = " ".join(env.opentitantool_args) if env.opentitantool_args else ""

        test_cmd_str = ctx.attr.test_cmd
        if test_cmd_str:
            test_cmd_str = ctx.expand_location(test_cmd_str, targets = ctx.attr.target_data)
        if not is_custom_harness and not test_cmd_str:
            test_cmd_str = "console --non-interactive --exit-success='{}' --exit-failure='{}'".format(exit_success, exit_failure)

        if is_custom_harness:
            test_exec = test_harness.short_path
            test_args = "--interface={interface}".format(interface = env.interface)
            if test_cmd_str:
                test_args += " " + test_cmd_str
        else:
            test_exec = opentitantool.short_path
            if env.test_args:
                formatted_test_args = [arg.format(firmware = bin_file.short_path) for arg in env.test_args]
                test_args = " ".join(formatted_test_args)
            else:
                test_args = "--interface={interface}".format(interface = env.interface)

            test_args = "--rcfile= " + test_args
            if test_cmd_str:
                test_args += " " + test_cmd_str

        script_content = "#!/bin/bash\nset -e\n"
        if setup_args_str:
            script_content += """
echo "=== Performing Board Setup ==="
echo {opentitantool} --rcfile= --interface={interface} {extra_args} {setup_args}
{opentitantool} --rcfile= --interface={interface} {extra_args} {setup_args}
""".format(
                opentitantool = opentitantool.short_path,
                interface = env.interface,
                extra_args = extra_args,
                setup_args = setup_args_str,
            )

        if clear_bitstream:
            script_content += """
echo "=== Running Test ==="
echo {test_exec} {test_args} "$@"
set +e
{test_exec} {test_args} "$@"
TEST_EXIT_CODE=$?
set -e
echo "=== Clearing Bitstream (Post-Test) ==="
echo {opentitantool} --rcfile= --interface={interface} {extra_args} --exec="fpga clear-bitstream" no-op
{opentitantool} --rcfile= --interface={interface} {extra_args} --exec="fpga clear-bitstream" no-op
exit $TEST_EXIT_CODE
""".format(
                test_exec = test_exec,
                test_args = test_args,
                opentitantool = opentitantool.short_path,
                interface = env.interface,
                extra_args = extra_args,
            )
        else:
            script_content += """
echo "=== Running Test ==="
echo {test_exec} {test_args} "$@"
exec {test_exec} {test_args} "$@"
""".format(
                test_exec = test_exec,
                test_args = test_args,
            )

        ctx.actions.write(
            output = run_script,
            is_executable = True,
            content = script_content,
        )

        return [
            DefaultInfo(
                runfiles = base_runfiles,
                executable = run_script,
            ),
            OutputGroupInfo(**output_groups),
        ]

_BASE_ATTRS = {
    "ecdsa_key": attr.label_keyed_string_dict(
        allow_files = True,
        providers = [KeySetInfo],
        doc = "ECDSA public key to validate this image",
    ),
    "environment": attr.label(
        providers = [TestEnvironmentInfo],
        mandatory = True,
        doc = "The test environment to run in.",
    ),
    "interface": attr.string(
        values = ["hyper310", "hyper340", "qemu", "teacup", "verilator"],
        mandatory = True,
    ),
    "manifest": attr.label(
        allow_files = True,
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
    "target_data": attr.label_list(
        doc = "Data files under the target transition",
        allow_files = True,
        cfg = _target_type_transition,
    ),
    "test_cmd": attr.string(
        default = "",
        doc = "Custom command to run after setup (for FPGA/Silicon) or instead of console (for Verilator)",
    ),
    "test_harness": attr.label(
        executable = True,
        cfg = "exec",
        doc = "Alternative test harness binary",
    ),
    "_flashgen": attr.label(
        executable = True,
        cfg = "exec",
        default = "//third_party/qemu:flashgen",
    ),
    "_opentitantool": attr.label(
        executable = True,
        allow_single_file = True,
        cfg = "exec",
        # TODO: update to opentitantool on master when everything is merged over.
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
        "clear_bitstream": attr.bool(
            default = False,
            doc = "If True, clear the FPGA bitstream before and after test execution.",
        ),
        "exit_failure": attr.string(
            default = "FAIL: .+\\n",
            doc = "The regex to look for in the output to determine failure.",
        ),
        "exit_success": attr.string(
            default = "PASS\\n",
            doc = "The regex to look for in the output to determine success.",
        ),
    },
)

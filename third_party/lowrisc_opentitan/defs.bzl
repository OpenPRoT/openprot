load("@rules_rust//rust/private:providers.bzl", "CrateInfo", "DepInfo", "DepVariantInfo")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library")

def _opentitan_transition_impl(settings, attr):
    return {
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_chipwhisperer": True,
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_dediprog": False,
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_ftdi": False,
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_hyperdebug": True,
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_proxy": False,
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_ti50emulator": False,
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_verilator": True,
        "@lowrisc_opentitan//sw/host/opentitantool:ot_certs": False,
    }

opentitan_transition = transition(
    implementation = _opentitan_transition_impl,
    inputs = [],
    outputs = [
        "@lowrisc_opentitan//sw/host/opentitantool:ot_certs",
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_chipwhisperer",
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_hyperdebug",
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_verilator",
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_dediprog",
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_ftdi",
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_proxy",
        "@lowrisc_opentitan//sw/host/opentitanlib:transport_ti50emulator",
    ],
)

# Wrapper rule for opentitantool (binary)
def _opentitantool_binary_impl(ctx):
    actual = ctx.attr.actual[0]
    actual_exec = actual[DefaultInfo].files_to_run.executable

    out_exec = ctx.actions.declare_file(ctx.label.name)

    ctx.actions.symlink(
        output = out_exec,
        target_file = actual_exec,
        is_executable = True,
    )

    return [
        DefaultInfo(
            executable = out_exec,
            files = depset([out_exec]),
            runfiles = actual[DefaultInfo].default_runfiles,
        ),
    ]

_opentitan_rust_binary_rule = rule(
    implementation = _opentitantool_binary_impl,
    attrs = {
        "actual": attr.label(
            cfg = opentitan_transition,
            mandatory = True,
        ),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
    executable = True,
)

# Wrapper rule for opentitanlib (library)
def _opentitanlib_library_impl(ctx):
    actual = ctx.attr.actual[0]

    providers = []
    providers.append(actual[DefaultInfo])

    if CcInfo in actual:
        providers.append(actual[CcInfo])

    if CrateInfo in actual:
        providers.append(actual[CrateInfo])
    if DepInfo in actual:
        providers.append(actual[DepInfo])
    if DepVariantInfo in actual:
        providers.append(actual[DepVariantInfo])

    if OutputGroupInfo in actual:
        providers.append(actual[OutputGroupInfo])

    return providers

_opentitan_rust_library_rule = rule(
    implementation = _opentitanlib_library_impl,
    attrs = {
        "actual": attr.label(
            cfg = opentitan_transition,
            mandatory = True,
        ),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)

def opentitan_rust_binary(name, actual = None, **kwargs):
    if actual:
        _opentitan_rust_binary_rule(
            name = name,
            actual = actual,
            testonly = kwargs.pop("testonly", None),
            visibility = kwargs.pop("visibility", None),
            tags = kwargs.pop("tags", None),
        )
        if kwargs:
            fail("unexpected arguments when actual is set: {}".format(kwargs.keys()))
    else:
        raw_name = name + "_raw"
        rust_binary(
            name = raw_name,
            **kwargs
        )
        _opentitan_rust_binary_rule(
            name = name,
            actual = ":" + raw_name,
            testonly = kwargs.get("testonly"),
            visibility = kwargs.get("visibility"),
            tags = kwargs.get("tags"),
        )

def opentitan_rust_library(name, actual = None, **kwargs):
    if actual:
        _opentitan_rust_library_rule(
            name = name,
            actual = actual,
            testonly = kwargs.pop("testonly", None),
            visibility = kwargs.pop("visibility", None),
            tags = kwargs.pop("tags", None),
        )
        if kwargs:
            fail("unexpected arguments when actual is set: {}".format(kwargs.keys()))
    else:
        raw_name = name + "_raw"
        rust_library(
            name = raw_name,
            **kwargs
        )
        _opentitan_rust_library_rule(
            name = name,
            actual = ":" + raw_name,
            testonly = kwargs.get("testonly"),
            visibility = kwargs.get("visibility"),
            tags = kwargs.get("tags"),
        )

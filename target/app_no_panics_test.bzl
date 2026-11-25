# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Test that the userspace apps of a pw_kernel system image cannot panic.

Pigweed's `rust_binary_no_panics_test` looks for the kernel's
`panic_is_possible` symbol, so it passes for code linked into a userspace app.
Every panic path calls a function in `core::panicking`, and the linker drops
those functions from an app that has no panic path. This test lists the symbols
of each app ELF with the toolchain's `nm` and fails if any is in that module.
"""

load("@bazel_tools//tools/cpp:toolchain_utils.bzl", "use_cpp_toolchain")
load("@pigweed//pw_build:binary_tools.bzl", "run_action_on_executable")
load("@pigweed//pw_toolchain/action:action_names.bzl", "PW_ACTION_NAMES")

_SCRIPT = """\
#!/bin/bash
status=0
for symbols in {symbols}; do
  app="$(basename "$symbols" .symbols)"
  if ! [ -s "$symbols" ]; then
    echo "$app: no symbols, cannot check for panics"
    status=1
  elif panics="$(grep -F 'core::panicking::' "$symbols")"; then
    echo "$app: panic is possible, the app links:"
    echo "$panics"
    status=1
  fi
done
exit $status
"""

def _app_no_panics_test_impl(ctx):
    symbols = []
    for elf in ctx.files.apps:
        output = ctx.actions.declare_file("{}/{}.symbols".format(ctx.label.name, elf.basename))
        run_action_on_executable(
            ctx = ctx,
            action_name = PW_ACTION_NAMES.nm,
            action_args = "--demangle {} > {}".format(elf.path, output.path),
            inputs = [elf],
            output = output,
            additional_outputs = [],
        )
        symbols.append(output)

    script = ctx.actions.declare_file(ctx.label.name + ".sh")
    ctx.actions.write(
        output = script,
        content = _SCRIPT.format(symbols = " ".join([f.short_path for f in symbols])),
        is_executable = True,
    )
    return [DefaultInfo(executable = script, runfiles = ctx.runfiles(symbols))]

_app_no_panics_test = rule(
    implementation = _app_no_panics_test_impl,
    test = True,
    attrs = {
        "apps": attr.label(
            doc = "The `<image>.apps` target that compiles the image's apps.",
            mandatory = True,
        ),
    },
    toolchains = use_cpp_toolchain(),
    fragments = ["cpp"],
    doc = "Checks that no app ELF links a `core::panicking` function.",
)

def app_no_panics_test(name, image, **kwargs):
    """Checks that the userspace apps of a system image cannot panic.

    Args:
        name: Name of the test target.
        image: Label of a `system_image` with `apps`, in the same package.
        **kwargs: Arguments passed through to the test rule.
    """
    _app_no_panics_test(
        name = name,
        # `system_image` compiles its apps in a private `<name>.apps` target.
        apps = image + ".apps",
        **kwargs
    )

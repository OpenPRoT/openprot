# Licensed under the Apache-2.0 license
# SPDX-License-Identifier: Apache-2.0

"""Skylark macros for building Caliptra firmware targets.

These macros encapsulate the common patterns used when building Caliptra ROM,
FMC, Runtime, and MCU ROM binaries from upstream sources.  They replace
inline genrule shell commands with documented, parameterised helpers that
are easier to review, maintain, and uprev.
"""

load("@rules_rust//rust:defs.bzl", "rust_binary")

# ---------------------------------------------------------------------------
# Shared compiler flags for size-optimised firmware binaries (ROM, FMC, RT).
# ---------------------------------------------------------------------------

# Common rustc flags shared by all Caliptra embedded firmware targets.
# Maintains LTO, opt-level=s, codegen-units=1 for code-size parity with
# upstream Cargo builds.
CALIPTRA_FIRMWARE_RUSTC_FLAGS = [
    "-C",
    "panic=abort",
    "-C",
    "opt-level=s",
    "-C",
    "codegen-units=1",
    "-C",
    "lto=fat",
    "-C",
    "embed-bitcode=yes",
    "-C",
    "link-arg=-Wl,--relax",
]

# ---------------------------------------------------------------------------
# caliptra_objcopy
# ---------------------------------------------------------------------------

def caliptra_objcopy(name, src, out = None, format = "binary", **kwargs):
    """Convert an ELF target to a raw binary via llvm-objcopy.

    Wraps the llvm-objcopy invocation.

    Note: requires tags = ["no-sandbox"] since the LLVM toolchain binary
    is not exposed as a proper Bazel tool target.

    Args:
        name: Target name.
        src: Label of the ELF binary to convert.
        out: Output filename (defaults to <name>.bin).
        format: objcopy output format (default: "binary").
        **kwargs: Extra arguments forwarded to genrule (e.g. tags, visibility).
    """
    if out == None:
        out = name + ".bin"

    # The Pigweed-managed LLVM toolchain exposes llvm-objcopy as a
    # native_binary target.  Using it via tools + $(execpath ...) is
    # the correct Bazel-portable approach (no hardcoded paths).
    _LLVM_OBJCOPY = "@@pigweed++pw_cxx_toolchain+llvm_toolchain//:llvm-objcopy"

    native.genrule(
        name = name,
        srcs = [src],
        outs = [out],
        cmd = "$(execpath {objcopy}) -O {fmt} $(location {src}) $@".format(
            objcopy = _LLVM_OBJCOPY,
            fmt = format,
            src = src,
        ),
        tools = [_LLVM_OBJCOPY],
        **kwargs
    )

# ---------------------------------------------------------------------------
# caliptra_rom_package
# ---------------------------------------------------------------------------

def caliptra_rom_package(name, rom_elf, out = None, packager = None, **kwargs):
    """Package a Caliptra ROM ELF into the raw boot image via elf2rom.

    Runs the caliptra_rom_packager host tool which packs the ELF into the
    0x18000-byte image expected by boot, and patches the SHA-256 digest in
    CALIPTRA_ROM_INFO so the on-chip rom_integrity_test passes.

    Args:
        name: Target name.
        rom_elf: Label of the Caliptra ROM ELF (rust_binary).
        out: Output filename (defaults to cptra-rom.bin).
        packager: Label of the elf2rom packager tool.
            Defaults to //third_party/caliptra/caliptra-sw:caliptra_rom_packager.
        **kwargs: Extra arguments forwarded to genrule.
    """
    if out == None:
        out = "cptra-rom.bin"
    if packager == None:
        packager = "//third_party/caliptra/caliptra-sw:caliptra_rom_packager"

    native.genrule(
        name = name,
        srcs = [rom_elf],
        outs = [out],
        cmd = "$(location {packager}) $(location {elf}) $@".format(
            packager = packager,
            elf = rom_elf,
        ),
        tools = [packager],
        **kwargs
    )

# ---------------------------------------------------------------------------
# caliptra_firmware_bundle
# ---------------------------------------------------------------------------

def caliptra_firmware_bundle(name, fmc, runtime, out = None, bundler = None, **kwargs):
    """Sign and bundle FMC + Runtime ELFs into the Caliptra firmware image.

    Runs the caliptra_firmware_bundler host tool which signs both ELFs with
    fake keys and packs them into the cptra-firmware.bin image consumed by
    the emulator and hardware boot flow.

    Args:
        name: Target name.
        fmc: Label of the FMC ELF (rust_binary).
        runtime: Label of the Runtime ELF (rust_binary).
        out: Output filename (defaults to cptra-firmware.bin).
        bundler: Label of the bundler tool.
            Defaults to //third_party/caliptra/caliptra-sw:caliptra_firmware_bundler.
        **kwargs: Extra arguments forwarded to genrule.
    """
    if out == None:
        out = "cptra-firmware.bin"
    if bundler == None:
        bundler = "//third_party/caliptra/caliptra-sw:caliptra_firmware_bundler"

    native.genrule(
        name = name,
        srcs = [fmc, runtime],
        outs = [out],
        tools = [bundler],
        cmd = "CALIPTRA_IMAGE_NO_GIT_REVISION=1 $(location {bundler}) ".format(
            bundler = bundler,
        ) + "--fmc $(location {fmc}) --runtime $(location {runtime}) --output $@".format(
            fmc = fmc,
            runtime = runtime,
        ),
        **kwargs
    )

# ---------------------------------------------------------------------------
# caliptra_linker_script
# ---------------------------------------------------------------------------

def caliptra_linker_script(name, generator, out, **kwargs):
    """Generate a linker script using a Rust host binary.

    Runs a host tool that writes a linker script to stdout, capturing the
    output as a build artifact.

    Args:
        name: Target name.
        generator: Label of the host binary that produces the linker script.
        out: Output filename for the linker script.
        **kwargs: Extra arguments forwarded to genrule.
    """
    native.genrule(
        name = name,
        outs = [out],
        cmd = "$(location {gen}) > $@".format(gen = generator),
        tools = [generator],
        **kwargs
    )

# ---------------------------------------------------------------------------
# caliptra_firmware_binary
# ---------------------------------------------------------------------------

def caliptra_firmware_binary(
        name,
        srcs,
        crate_root,
        deps,
        linker_scripts,
        map_file = None,
        crate_features = [],
        compile_data = [],
        proc_macro_deps = [],
        aliases = {},
        extra_rustc_flags = [],
        **kwargs):
    """Build a Caliptra firmware ELF with size-optimised LTO settings.

    Wraps rust_binary with the common Caliptra firmware compiler flags
    (fat LTO, opt-level=s, codegen-units=1, panic=abort) and linker
    script arguments.

    Args:
        name: Target name.
        srcs: Source files or prepared source labels.
        crate_root: Path to the crate root (main.rs).
        deps: Rust dependencies.
        linker_scripts: Dict of {label: execpath_ref} for linker scripts to
            pass via -Tlink-arg.  E.g. {"fmc_memory.x": None, "link.x": None}
            where the value is unused (the label itself is used in execpath).
            Or simply a list of labels.
        map_file: Optional map file name for -Wl,-Map.
        crate_features: Crate feature flags.
        compile_data: Additional compile_data files (linker scripts are
            added automatically).
        proc_macro_deps: Proc-macro dependencies.
        aliases: Crate aliases.
        extra_rustc_flags: Additional rustc flags beyond the standard set.
        **kwargs: Extra arguments forwarded to rust_binary.
    """

    # Build link-arg flags for each linker script.
    link_flags = []
    if type(linker_scripts) == type([]):
        linker_scripts_list = linker_scripts
    else:
        linker_scripts_list = linker_scripts.keys() if hasattr(linker_scripts, "keys") else list(linker_scripts)

    for ls in linker_scripts_list:
        link_flags += ["-C", "link-arg=-T$(execpath {})".format(ls)]

    if map_file:
        link_flags += ["-C", "link-arg=-Wl,-Map={}".format(map_file)]

    all_rustc_flags = (
        ["-A", "mismatched_lifetime_syntaxes"] +
        CALIPTRA_FIRMWARE_RUSTC_FLAGS +
        link_flags +
        extra_rustc_flags
    )

    rust_binary(
        name = name,
        srcs = srcs,
        crate_root = crate_root,
        edition = "2021",
        crate_features = crate_features,
        target_compatible_with = ["//third_party/caliptra/platforms:target_caliptra"],
        compile_data = list(linker_scripts_list) + compile_data,
        deps = deps,
        proc_macro_deps = proc_macro_deps,
        aliases = aliases,
        rustc_flags = all_rustc_flags,
        **kwargs
    )

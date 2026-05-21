# Setting up QEMU for local OpenPRoT development

By default, the build system downloads a pre-built release of the [lowRISC QEMU fork](https://github.com/lowRISC/qemu/).
For local development and debugging, you can build QEMU from source instead.

## Building from source using Bazel

Check out the lowRISC QEMU fork and switch to the correct branch:

```bash
git clone https://github.com/lowRISC/qemu
cd qemu
git checkout ot-10.2.0   # branch corresponding to the pinned release
```

Perform this setup step once at the root of your QEMU checkout:

```bash
touch REPO.bazel
ln -s "/path/to/openprot/third_party/qemu/BUILD.qemu_opentitan.bazel" "BUILD.bazel"
```

Then tell Bazel to use your local checkout instead of the downloaded archive by passing
`--override_repository` on every Bazel invocation:

```bash
bazelisk build --override_repository="+qemu+qemu_opentitan_src=/path/to/qemu" \
    //third_party/qemu:qemu-system-riscv32
```

To avoid repeating this flag, add it to your `.bazelrc-site` file at the repo root:

```
common --override_repository=+qemu+qemu_opentitan_src=/path/to/qemu
```

## Troubleshooting

### Finding the canonical repository name

If Bazel reports that `+qemu+qemu_opentitan_src` is not a valid repository name, run:

```bash
bazelisk mod dump_repo_mapping "" | jq .qemu_opentitan_src
```

Use the reported canonical name in `--override_repository`.

### How the override detection works

`extensions.bzl`'s `qemu_bazel_build_or_forward` rule checks for a marker file
(`.this.is.the.archive`) that is injected into the pre-built release archive.
If the marker is absent (local checkout), the rule runs `build_qemu.sh` inside your
source directory, which configures with:

```
--target-list=riscv32-softmmu
--without-default-features
--enable-tcg
--enable-tools
--enable-trace-backends=log
```

These flags are the minimum needed for the `ot-earlgrey` machine.
Do not add other flags — they either bloat the build or break OT-specific TCG plugins.

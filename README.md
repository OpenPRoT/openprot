# OpenPRoT

## Technical Charter

The OpenPRoT Technical Charter can be found at
[<u>https://github.com/OpenPRoT/.github/blob/main/GOVERNANCE.md</u>](https://github.com/OpenPRoT/.github/blob/main/GOVERNANCE.md)

## Getting Started

NOTE: We are converting our build system to [bazel](https://bazel.build/).  We recommend installing [bazelisk](https://github.com/bazelbuild/bazelisk) to automatically manage bazel versions.

### Available Tasks


You can run tasks using the Pigweed workflow launcher `pw` or `bazel`.

- `./pw presubmit` - Run presubmit checks: formatting, license checks, C/C++ header checks and `clippy`.
- `./pw format` - Run the code formatters.
- `bazel test //...` - Run all tests.
- `bazel build //docs` - Build documentation.

### Development

The project is structured as a bazel module.

## Requirements

- [Bazel](https://bazel.build/).  We recommend installing [bazelisk](https://github.com/bazelbuild/bazelisk) to automatically manage bazel versions.

No additional tools are required to build and test - all dependencies are managed by bazel.
The sections below are optional and only describe editor setup.

## VS Code setup

### 1. Install tools

- **Rust toolchain and rust-analyzer.** Builds do not use `rustup`, but the editor does.
  Install [rustup](https://rustup.rs/). The repository's `rust-toolchain.toml` pins the
  nightly channel and lists the `rust-analyzer`, `rust-src`, `rustfmt` and `clippy`
  components, so rustup installs them automatically the first time `rustc` or
  `cargo` runs in the repository. To install them eagerly (or to add the component to an
  older checkout), run:

  ```bash
  rustup show                              # installs the pinned toolchain + components
  rustup component add rust-analyzer       # only if it is missing
  ```

- **Buildifier** formats `BUILD.bazel`/`.bzl` files. Download a prebuilt binary from the
  [buildtools releases](https://github.com/bazelbuild/buildtools/releases) and put it on
  your `PATH` (use `buildifier-linux-arm64` on ARM Linux, or `buildifier-darwin-amd64` /
  `buildifier-darwin-arm64` on macOS):

  ```bash
  sudo curl -fsSL -o /usr/local/bin/buildifier \
    https://github.com/bazelbuild/buildtools/releases/latest/download/buildifier-linux-amd64
  sudo chmod +x /usr/local/bin/buildifier
  buildifier --version
  ```

  Alternatively, `go install github.com/bazelbuild/buildtools/buildifier@latest`.
  `./pw format` also runs buildifier, so the editor tool is only for format-on-save.

### 2. Install extensions

- [Bazel](https://marketplace.visualstudio.com/items?itemName=BazelBuild.vscode-bazel)
  (`BazelBuild.vscode-bazel`): Starlark syntax, target browsing, and buildifier integration.
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
  (`rust-lang.rust-analyzer`).

### 3. Generate `rust-project.json`

There is no Cargo workspace, so rust-analyzer gets its crate graph from a generated
`rust-project.json` (git-ignored) at the repository root:

```bash
bazel run @rules_rust//tools/rust_analyzer:gen_rust_project
```

This covers `//...`, which is large (hundreds of crates) and can use more than 10 GB of
memory in rust-analyzer, which may get it killed on small machines or WSL. Prefer
generating for only the targets you are working on:

```bash
bazel run @rules_rust//tools/rust_analyzer:gen_rust_project -- \
  //services/orchestrator/... //drivers/usart/...
```

Crates that only build for an embedded target (those with
`target_compatible_with = TARGET_COMPATIBLE_WITH`, e.g. everything under
`target/ast10x0/`) are skipped when generating for the host, so their files get no
highlighting or hover. Pass the target's Bazel config to the generator's inner build
with `--config`:

```bash
bazel run @rules_rust//tools/rust_analyzer:gen_rust_project -- \
  --config=virt_ast10x0 //target/ast10x0/peripherals:peripherals
```

The same applies to Earl Grey:

```bash
bazel run @rules_rust//tools/rust_analyzer:gen_rust_project -- \
  --config=k_earlgrey //target/earlgrey/...
```

#### Troubleshooting: rust-analyzer shows nothing for a file or a new target

If a file has no highlighting, hover or go-to-definition, its crate is most likely not
in `rust-project.json`. Check, in order:

1. **Is the target listed?** Search `rust-project.json` for the crate name or file path.
   If it is missing, add the target (or a parent package such as `//path/to/pkg/...`)
   to the `gen_rust_project` command. Remember that each run overwrites the file.
2. **Is it embedded-only?** If the target uses `target_compatible_with`, it is skipped
   on the host platform, with no error. Pass the config that selects its platform
   after the `--`. The configs are defined in `.bazelrc`:

   | Directory               | Config                          |
   |-------------------------|---------------------------------|
   | `target/ast10x0/`       | `--config=virt_ast10x0`         |
   | `target/earlgrey/`      | `--config=k_earlgrey`           |
   | `target/veer/`          | `--config=k_veer`               |

3. **No config exists for your target?** Add one to `.bazelrc` next to the existing
   ones. It must use the `common:` prefix, not just `build:`, so the setting also
   reaches the generator's inner build:

   ```
   common:k_mytarget --platforms=//target/mytarget:mytarget
   build:k_mytarget --build_tag_filters=-do_not_build,-kernel_doc_test
   ```

4. **Reload.** After regenerating, run "rust-analyzer: Reload Workspace". Double-check
   the spelling of the command and config names, since a typo in either fails the run.

Notes:

- Put `--config` after the `--`. Using `--platforms` on `bazel run` itself builds the
  generator for the embedded target and fails.
- Each run overwrites `rust-project.json`, so list every target you need in one command.
- Regenerate after adding crates or changing dependencies, then run
  "rust-analyzer: Reload Workspace".

### 4. Configure `.vscode/settings.json`

`.vscode/settings.json` is git-ignored, so each developer keeps their own copy. A
suggested starting point:

```json
{
    "rust-analyzer.linkedProjects": ["rust-project.json"],
    "rust-analyzer.cachePriming.enable": false,
    "rust-analyzer.numThreads": 4,
    "rust-analyzer.checkOnSave": false,
    "rust-analyzer.files.exclude": ["third_party/**", "bazel-*/**"],
    "bazel.buildifier.executable": "/usr/local/bin/buildifier",
    "bazel.codeLens.enable": true,
    "[starlark]": {
        "editor.defaultFormatter": "BazelBuild.vscode-bazel",
        "editor.formatOnSave": true
    }
}
```

- `linkedProjects` is required; it points rust-analyzer at the generated file.
- `cachePriming.enable`, `numThreads`, `checkOnSave` and `files.exclude` limit
  rust-analyzer's memory use; relax them if your machine has plenty of RAM.
- Set `bazel.buildifierExecutable` to the output of `which buildifier`.
- `checkOnSave` is disabled because the default `cargo check` does not work here. Use
  `./pw presubmit` (which runs clippy through Bazel) instead.

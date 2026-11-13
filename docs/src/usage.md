# Usage

openprot is built with Bazel and driven through Pigweed's workflow launcher.
The repository is a Bazel module (`MODULE.bazel`); there is no Cargo
workspace. All everyday tasks go through either `./pw <group>` or `bazel`
directly.

## Workflow launcher (`./pw`)

`./pw` is a thin wrapper around `bazelisk run //:pw -- "$@"`. It dispatches
to one of the named groups defined in `workflows.json`:

```bash
./pw presubmit         # format check + presubmit_checks analyzer + clippy
./pw format            # apply rustfmt / black / buildifier / clang-format
./pw default           # wildcard build (//... minus //third_party/caliptra/...)
./pw ci                # run CI tests (skips hardware + verilator targets)
./pw upstream_pigweed  # ci tests + Earlgrey verilator tests
```

`./pw presubmit` is what you run before every commit. It is also the gate
on CI.

## Building and testing with Bazel directly

Use `bazelisk` (the wrapper accepts every standard `bazel` flag).

```bash
bazel test //...                                                # host tests
bazel build //... --build_tag_filters=-hardware,-disabled       # practical build
bazel build //... --build_tag_filters=-hardware,-disabled,-verilator
bazel test //path/to:target_name                                # single target
```

The tag filters above are not optional for a wildcard build: `//...` includes
hardware-only and verilator targets that will fail on a developer machine
without the matching environment. The `ci` and `default` workflows already
apply these filters.

Note that `//third_party/caliptra/...` is intentionally excluded from
wildcard builds (see `workflows.json:34`, `:55`, `:78`). The Caliptra
crate-universe workspaces collide with the host workspace when both are
built unconditionally; build into Caliptra paths explicitly when you need
them.

## Clippy

Clippy runs as a `rules_rust` aspect, not as a separate target:

```bash
bazelisk build \
    --aspects=@rules_rust//rust:defs.bzl%rust_clippy_aspect \
    --output_groups=clippy_checks \
    //... -//third_party/caliptra/...
```

`./pw presubmit` invokes the same aspect via the `clippy` build entry.

## Documentation

mdbook sources live under `docs/`. Build with:

```bash
bazel build //docs
```

The book is written to the `bazel-bin/docs/docs` directory; open `index.html`
there to read it. To rebuild whenever a file changes:

```bash
bazel run //:watch build //docs
```

`//:watch` is Pigweed's `pw_watch`. It runs `bazelisk` with the arguments it is
given, once at startup and again after each change to a source file in the
repository. `./pw watch` is a shortcut for `bazel run //:watch`. Press Enter to
rerun by hand and Ctrl-C to stop. The watcher also exits when stdin closes, so
run it in an interactive terminal and use `bazel build //docs` for a single
build in a script.

To run several commands in order, put a comma between them as a separate
argument:

```bash
bazel run //:watch build //docs , test //services/...
```

To preview the book in a browser, run:

```bash
bazel run //docs:serve
```

This serves the book on `http://localhost:8000`. When a source file changes it
rebuilds the book and reloads the page, without the watcher.

## Tag reference

Targets can carry tags that gate which environment they run in. The common
ones used in this tree are:

- `hardware` — requires real silicon; never built locally.
- `verilator` — requires a verilator simulator install.
- `disabled` — temporarily excluded.
- `manual` — never built by `//...`; only when explicitly named.
- `kernel` — `pw_kernel`-bound system images and supporting targets.

Combine these via `--build_tag_filters` (build-time) and `--test_tag_filters`
(test-time). The `workflows.json` file is the canonical reference for which
filters each workflow applies.

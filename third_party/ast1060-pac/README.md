# ast1060-pac Third-Party Integration

[`ast1060-pac`](https://github.com/OpenPRoT/ast1060-pac) is an svd2rust-generated PAC for the ASPEED AST1060. It is fetched from git and built via a Bazel overlay — the upstream repo has no `BUILD.bazel`.

## Files

| File | Purpose |
|---|---|
| `versions.bzl` | Git commit pin |
| `extensions.bzl` | Bzlmod module extension (`git_repository` fetch) |
| `overlay.BUILD` | Injected `rust_library` target |
| `BUILD.bazel` | Exports `overlay.BUILD` for use as `build_file` |

## Integration Points

**`MODULE.bazel`** — registers the extension and imports the repo:
```starlark
ast1060_pac_repos_ext = use_extension("//third_party/ast1060-pac:extensions.bzl", "ast1060_pac_repos")
use_repo(ast1060_pac_repos_ext, "ast1060_pac")
```

**`third_party/crates_io/Cargo.toml`** — adds `vcell = "0.1.2"` (PAC runtime dep).

## Notes

- `overlay.BUILD` passes `-Amismatched-lifetime-syntaxes` because svd2rust emits older lifetime syntax that triggers `-D warnings`. The proper fix is `#![allow(mismatched_lifetime_syntaxes)]` in the PAC's `src/lib.rs`.
- To use: `deps = ["@ast1060_pac//:ast1060_pac"]`, build with `--platforms=//target/ast10x0:ast10x0`.
- To upgrade: update the SHA in `versions.bzl`, then regenerate `MODULE.bazel.lock`.

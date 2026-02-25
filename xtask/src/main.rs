// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

use std::{
    env,
    path::{Path, PathBuf},
};

use xshell::{cmd, Shell};

mod cargo_lock;
mod header;
mod precheckin;

type DynError = Box<dyn std::error::Error>;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{e}");
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    // nosemgrep: rust.lang.security.args.args
    let task = env::args().nth(1);
    match task.as_deref() {
        Some("build") => build()?,
        Some("test") => test()?,
        Some("check") => check()?,
        Some("clippy") => clippy()?,
        Some("fmt") => fmt()?,
        Some("clean") => clean()?,
        Some("dist") => dist()?,
        Some("deny") => cargo_deny()?,
        Some("cargo-lock") => cargo_lock::cargo_lock()?,
        Some("precheckin") => precheckin::precheckin()?,
        Some("header-check") => header::check()?,
        Some("header-fix") => header::fix()?,
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:

build           Build the project
test            Run all tests
check           Run cargo check
clippy          Run clippy lints
fmt             Format code with rustfmt
clean           Clean build artifacts
dist            Build a distribution (release build)
deny            Run cargo deny checks (licenses, advisories, bans)
cargo-lock      Manage Cargo.lock file
precheckin      Run all pre-checkin validation checks
header-check    Check license headers in source files
header-fix      Fix missing license headers in source files
"
    )
}

fn build() -> Result<(), DynError> {
    let sh = Shell::new()?;
    sh.change_dir(project_root());

    cmd!(sh, "cargo build").run()?;

    Ok(())
}

fn test() -> Result<(), DynError> {
    let sh = Shell::new()?;
    sh.change_dir(project_root());

    cmd!(sh, "cargo test").run()?;

    Ok(())
}

fn check() -> Result<(), DynError> {
    let sh = Shell::new()?;
    sh.change_dir(project_root());

    cmd!(sh, "cargo check").run()?;

    Ok(())
}

fn clippy() -> Result<(), DynError> {
    let sh = Shell::new()?;
    sh.change_dir(project_root());

    cmd!(sh, "cargo clippy -- -D warnings").run()?;

    Ok(())
}

fn cargo_deny() -> Result<(), DynError> {
    let sh = Shell::new()?;
    sh.change_dir(project_root());

    // Check if specific subcommand is passed
    // NOTE: This is a development tool, not security-critical code
    #[allow(clippy::disallowed_methods)] // Allow env::args() for CLI parsing in dev tools
    // nosemgrep: rust.lang.security.args.args
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        if let Some(subcommand) = args.get(2) {
            match subcommand.as_str() {
                "licenses" => cmd!(sh, "cargo deny check licenses").run()?,
                "advisories" => cmd!(sh, "cargo deny check advisories").run()?,
                "bans" => cmd!(sh, "cargo deny check bans").run()?,
                "sources" => cmd!(sh, "cargo deny check sources").run()?,
                _ => {
                    eprintln!("Unknown deny subcommand: {subcommand}");
                    eprintln!("Available: licenses, advisories, bans, sources");
                    std::process::exit(1);
                }
            }
        }
    } else {
        // Run all checks by default
        cmd!(sh, "cargo deny check").run()?;
    }

    Ok(())
}

fn fmt() -> Result<(), DynError> {
    let sh = Shell::new()?;
    sh.change_dir(project_root());

    // Check if --check flag is passed
    // NOTE: This is a development tool, not security-critical code
    #[allow(clippy::disallowed_methods)] // Allow env::args() for CLI parsing in dev tools
    // nosemgrep: rust.lang.security.args.args
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 && args.get(2).map(|s| s == "--check").unwrap_or(false) {
        cmd!(sh, "cargo fmt -- --check").run()?;
    } else {
        cmd!(sh, "cargo fmt").run()?;
    }

    Ok(())
}

fn clean() -> Result<(), DynError> {
    let sh = Shell::new()?;
    sh.change_dir(project_root());

    cmd!(sh, "cargo clean").run()?;

    Ok(())
}

fn dist() -> Result<(), DynError> {
    let sh = Shell::new()?;
    sh.change_dir(project_root());

    // Clean first
    cmd!(sh, "cargo clean").run()?;

    // Build release
    cmd!(sh, "cargo build --release").run()?;

    // Create dist directory
    let dist_dir = dist_dir();
    if dist_dir.exists() {
        sh.remove_path(&dist_dir)?;
    }
    sh.create_dir(&dist_dir)?;

    // Copy binary to dist
    let binary_name = "openprot";
    let src = project_root().join("target/release").join(binary_name);
    let dst = dist_dir.join(binary_name);
    sh.copy_file(&src, &dst)?;

    println!("Distribution created in: {}", dist_dir.display());

    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn dist_dir() -> PathBuf {
    project_root().join("target/dist")
}

// Licensed under the Apache-2.0 license

use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use xshell::{cmd, Shell};

type DynError = Box<dyn std::error::Error>;

pub(crate) fn docs() -> Result<(), DynError> {
    check_mdbook()?;
    check_mermaid()?;

    println!("Running: mdbook");
    let sh = Shell::new()?;
    let project_root = project_root();
    let docs_dir = project_root.join("docs");
    let dest_dir = project_root.join("target/book");

    // Create docs directory if it doesn't exist
    if !docs_dir.exists() {
        create_default_docs_structure(&docs_dir)?;
    }

    sh.change_dir(&docs_dir);
    cmd!(sh, "mdbook build --dest-dir {dest_dir}").run()?;

    println!(
        "Docs built successfully: view at {}/index.html",
        dest_dir.display()
    );

    Ok(())
}

fn check_mdbook() -> Result<(), DynError> {
    let status = Command::new("mdbook")
        .args(["--help"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if status.is_ok() {
        return Ok(());
    }

    println!("mdbook not found; installing...");
    let sh = Shell::new()?;
    cmd!(sh, "cargo install mdbook").run()?;

    Ok(())
}

fn check_mermaid() -> Result<(), DynError> {
    let status = Command::new("mdbook-mermaid")
        .args(["--help"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if status.is_ok() {
        return Ok(());
    }

    println!("mdbook-mermaid not found; installing...");
    let sh = Shell::new()?;
    cmd!(sh, "cargo install mdbook-mermaid").run()?;

    Ok(())
}

fn create_default_docs_structure(docs_dir: &Path) -> Result<(), DynError> {
    let sh = Shell::new()?;
    sh.create_dir(docs_dir)?;

    // Create book.toml
    let book_toml = docs_dir.join("book.toml");
    sh.write_file(
        &book_toml,
        r#"[book]
authors = ["OpenProt Team"]
language = "en"
src = "src"
title = "OpenProt Documentation"

[preprocessor.mermaid]
command = "mdbook-mermaid"

[output.html]
"#,
    )?;

    // Create src directory and SUMMARY.md
    let src_dir = docs_dir.join("src");
    sh.create_dir(&src_dir)?;

    let summary = src_dir.join("SUMMARY.md");
    sh.write_file(
        &summary,
        r#"# OpenProt Documentation

- [Introduction](./introduction.md)
- [Getting Started](./getting-started.md)
- [Usage](./usage.md)
- [Contributing](./contributing.md)
"#,
    )?;

    // Create a basic introduction file
    let intro = src_dir.join("introduction.md");
    sh.write_file(
        &intro,
        r#"# Introduction

Welcome to OpenProt Documentation.
"#,
    )?;

    Ok(())
}

fn project_root() -> PathBuf {
    let mut xtask_dir = env::current_exe().expect("current_exe failed");
    xtask_dir.pop(); // pop /target/debug
    xtask_dir.pop(); // pop /target
    xtask_dir
}

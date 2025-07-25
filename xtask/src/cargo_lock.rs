// Licensed under the Apache-2.0 license

use std::process::Command;

use crate::DynError;

pub(crate) fn cargo_lock() -> Result<(), DynError> {
    println!("Checking Cargo lock");
    let status = Command::new("cargo")
        .current_dir(crate::project_root())
        .args(["tree", "--locked"])
        .env("RUSTFLAGS", "-Cpanic=abort -Zpanic_abort_tests")
        .stdout(std::process::Stdio::null())
        .status()?;

    if !status.success() {
        return Err("cargo tree --locked failed; Please include required changes to Cargo.lock in your pull request".into());
    }
    Ok(())
}

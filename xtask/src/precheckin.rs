// Licensed under the Apache-2.0 license

use crate::DynError;

pub(crate) fn precheckin() -> Result<(), DynError> {
    println!("Running pre-checkin validation...");

    // Check Cargo.lock consistency
    println!("Checking Cargo.lock...");
    crate::cargo_lock::cargo_lock()?;

    // Format code
    println!("Checking code formatting...");
    crate::fmt()?;

    // Run clippy lints
    println!("Running clippy lints...");
    crate::clippy()?;

    // Check license headers
    println!("Checking license headers...");
    crate::header::check()?;

    // Run tests
    println!("Running tests...");
    crate::test()?;

    // Run cargo check
    println!("Running cargo check...");
    crate::check()?;

    // Build the project
    println!("Building project...");
    crate::build()?;

    println!("✅ All pre-checkin checks passed!");
    Ok(())
}

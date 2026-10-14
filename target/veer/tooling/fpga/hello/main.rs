// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! Smoke test for the aarch64 Linux cross toolchain: run on the VCK190's ARM
//! cores to confirm the binary loads and links against the board's glibc.

fn main() {
    println!(
        "Hello from OpenPRoT on {}-{}!",
        std::env::consts::ARCH,
        std::env::consts::OS
    );
}

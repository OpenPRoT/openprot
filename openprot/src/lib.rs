// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("OpenProt"), "Hello, OpenProt!");
    }
}

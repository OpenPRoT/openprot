// Licensed under the Apache-2.0 license
// SPDX-License-Identifier: Apache-2.0

//! `openprot-verify` — host-side CLI OCP-EAT token verifier.
//!
//! ```text
//! openprot-verify --trust-anchor <DER_FILE> [--trust-anchor <DER_FILE> ...]
//!                 [--nonce <HEX>] [--max-age <SECS>] [--max-chain-depth <N>]
//!                 [--json] <TOKEN_FILE>
//! ```
//!
//! `<TOKEN_FILE>` is a CBOR-encoded COSE_Sign1 OCP-EAT token.
//!
//! Exit codes: 0=Pass, 1=error, 2=Fail, 3=Indeterminate

use std::fs;
use std::process;
use std::time::Duration;

use openprot_attest_verifier::{ComponentDisposition, Disposition, Verifier, VerifyConfig};

fn main() {
    // nosemgrep
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let mut trust_anchor_files: Vec<String> = Vec::new();
    let mut nonce_hex: Option<String> = None;
    let mut max_age_secs: u64 = 60;
    let mut max_chain_depth: usize = 8;
    let mut json_output: bool = false;
    let mut token_file: Option<String> = None;

    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--trust-anchor" => {
                i += 1;
                trust_anchor_files.push(require_arg(&args, i, "--trust-anchor"));
            }
            "--nonce" => {
                i += 1;
                nonce_hex = Some(require_arg(&args, i, "--nonce"));
            }
            "--max-age" => {
                i += 1;
                let s = require_arg(&args, i, "--max-age");
                max_age_secs = s.parse().unwrap_or_else(|_| {
                    die(&format!("--max-age: '{s}' is not a valid integer"));
                });
            }
            "--max-chain-depth" => {
                i += 1;
                let s = require_arg(&args, i, "--max-chain-depth");
                max_chain_depth = s.parse().unwrap_or_else(|_| {
                    die(&format!("--max-chain-depth: '{s}' is not a valid integer"));
                });
            }
            "--json" => {
                json_output = true;
            }
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            other if other.starts_with('-') => {
                die(&format!("unknown flag: {other}"));
            }
            other => {
                token_file = Some(other.to_string());
            }
        }
        i += 1;
    }

    let token_path = token_file.unwrap_or_else(|| {
        die("a token file argument is required");
    });

    let token_bytes = read_file(&token_path);

    let trust_anchors: Vec<Vec<u8>> = trust_anchor_files.iter().map(|p| read_file(p)).collect();
    if trust_anchors.is_empty() {
        die("at least one --trust-anchor <file> is required");
    }

    let nonce = nonce_hex
        .as_deref()
        .map(|h| {
            hex_decode(h).unwrap_or_else(|e| {
                die(&format!("--nonce: {e}"));
            })
        })
        .unwrap_or_default();

    let config = VerifyConfig {
        trust_anchors,
        max_token_age: Duration::from_secs(max_age_secs),
        max_chain_depth,
    };

    let verifier = Verifier::new(config).unwrap_or_else(|e| {
        die(&format!("Verifier initialisation error: {e}"));
    });

    match verifier.verify_token(&token_bytes, &nonce) {
        Ok(evidence) => {
            print_result(
                &evidence.disposition,
                &evidence.component_results,
                json_output,
            );
            if !json_output {
                println!("Peer UEID: {}", hex_encode(&evidence.peer_ueid));
            }
            process::exit(exit_code(&evidence.disposition));
        }
        Err(e) => {
            if json_output {
                println!("{{\"error\":\"{e}\"}}");
            } else {
                eprintln!("Verification failed: {e}");
            }
            process::exit(2);
        }
    }
}

fn print_result(
    disposition: &Disposition,
    components: &std::collections::HashMap<String, ComponentDisposition>,
    json_output: bool,
) {
    let disp_str = disposition_str(disposition);
    if json_output {
        print!("{{\"disposition\":\"{disp_str}\",\"components\":{{");
        let mut sorted: Vec<_> = components.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());
        let mut first = true;
        for (name, disp) in sorted {
            if !first {
                print!(",");
            }
            print!("\"{}\":\"{}\"", json_escape(name), comp_disp_str(disp));
            first = false;
        }
        println!("}}}}");
    } else {
        println!("Appraisal result: {disp_str}");
        let mut sorted: Vec<_> = components.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());
        for (name, disp) in sorted {
            println!("  {name}: {}", comp_disp_str(disp));
        }
    }
}

fn disposition_str(d: &Disposition) -> &'static str {
    match d {
        Disposition::Pass => "Pass",
        Disposition::Fail => "Fail",
        Disposition::Indeterminate => "Indeterminate",
    }
}

fn comp_disp_str(d: &ComponentDisposition) -> &'static str {
    match d {
        ComponentDisposition::Pass => "Pass",
        ComponentDisposition::Fail => "Fail",
        ComponentDisposition::Unknown => "Unknown",
    }
}

fn exit_code(d: &Disposition) -> i32 {
    match d {
        Disposition::Pass => 0,
        Disposition::Fail => 2,
        Disposition::Indeterminate => 3,
    }
}

fn read_file(path: &str) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|e| die(&format!("Cannot read '{path}': {e}")))
}

fn require_arg(args: &[String], i: usize, flag: &str) -> String {
    if i >= args.len() {
        die(&format!("{flag} requires an argument"));
    }
    args[i].clone()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim_start_matches("0x");
    if !s.len().is_multiple_of(2) {
        return Err("odd-length hex string".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn die(msg: &str) -> ! {
    eprintln!("error: {msg}");
    process::exit(1);
}

fn print_usage() {
    eprintln!(
        "openprot-verify — OpenPRoT OCP-EAT attestation token verifier\n\
         \n\
         Usage:\n\
           openprot-verify --trust-anchor <DER_FILE> [--trust-anchor <DER_FILE> ...]\n\
                           [--nonce <HEX>] [--max-age <SECS>] [--max-chain-depth <N>]\n\
                           [--json] <TOKEN_FILE>\n\
         \n\
         Exit codes: 0=Pass, 1=error, 2=Fail, 3=Indeterminate"
    );
}

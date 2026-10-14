//! Minimal CLI: create / append / snapshot / dead / verify / view / checkpoint.
//! Usage:
//!   axiomere create <file.axmr>
//!   axiomere append <file> <type> <payload-json> [actor]
//!   axiomere snapshot <file> <record-id> <content-json> [actor]
//!   axiomere dead <file> <record-id> <reason> [actor]
//!   axiomere verify <file> [--trusted actor=pubkey_hex ...] [--trusted-file map.json]
//!                          [--expect-head HASH] [--expect-events N] [--expect-root ROOT]
//!   axiomere view <file> [--dead]
//!   axiomere checkpoint <file> [actor]
//! Signing: set AXIOMERE_SEED_HEX to a 32-byte hex seed to sign appends.
//!
//! `verify` exit codes (FR-VP-11): 0 = verified, 1 = verification
//! failed (a verdict was reached and it was no), 2 = file unreadable
//! (no verdict exists). Parse failures and verification failures are
//! never the same output.

use axiomere::{ExpectedHead, Ledger, VerifyPolicy};
use serde_json::Value;
use std::collections::BTreeMap;
use std::process::ExitCode;

fn seed() -> Option<[u8; 32]> {
    let hex_seed = std::env::var("AXIOMERE_SEED_HEX").ok()?;
    let bytes = hex::decode(hex_seed).ok()?;
    bytes.try_into().ok()
}

fn open(path: &str) -> Result<Ledger, Box<dyn std::error::Error>> {
    let ledger = Ledger::open(path)?;
    Ok(match seed() {
        Some(s) => ledger.with_signer(&s),
        None => ledger,
    })
}

/// `verify` does not go through `Ledger::open`: opening is a gate for
/// appends, verification is a question, and conflating them is what
/// FR-VP-11 forbids. Exit codes: 0 verified / 1 failed / 2 unreadable.
fn verify_cmd(path: &str, flags: &[String]) -> ExitCode {
    // Unreadable = no verdict exists (missing file, permissions, IO).
    if let Err(e) = std::fs::File::open(path) {
        eprintln!("unreadable: {path}: {e}");
        return ExitCode::from(2);
    }
    let mut trusted: Option<BTreeMap<String, String>> = None;
    let mut head = ExpectedHead::default();
    let mut use_head = false;
    let mut i = 0;
    while i < flags.len() {
        match flags[i].as_str() {
            "--trusted" => {
                let kv = flags.get(i + 1).cloned().unwrap_or_default();
                match kv.split_once('=') {
                    Some((actor, pubkey)) => {
                        trusted
                            .get_or_insert_with(BTreeMap::new)
                            .insert(actor.to_string(), pubkey.to_string());
                    }
                    None => {
                        eprintln!("usage: --trusted actor=pubkey_hex");
                        return ExitCode::from(2);
                    }
                }
                i += 2;
            }
            "--trusted-file" => {
                let p = flags.get(i + 1).cloned().unwrap_or_default();
                match std::fs::read_to_string(&p)
                    .ok()
                    .and_then(|t| serde_json::from_str::<BTreeMap<String, String>>(&t).ok())
                {
                    Some(map) => trusted = Some(map),
                    None => {
                        eprintln!("unreadable trusted-file: {p}");
                        return ExitCode::from(2);
                    }
                }
                i += 2;
            }
            "--expect-head" => {
                head.head_hash = flags.get(i + 1).cloned();
                use_head = true;
                i += 2;
            }
            "--expect-events" => {
                head.min_events = flags.get(i + 1).and_then(|s| s.parse().ok());
                use_head = true;
                i += 2;
            }
            "--expect-root" => {
                head.checkpoint_root = flags.get(i + 1).cloned();
                use_head = true;
                i += 2;
            }
            other => {
                eprintln!("unknown verify flag: {other}");
                return ExitCode::from(2);
            }
        }
    }
    let policy = VerifyPolicy {
        trusted,
        expected_head: if use_head { Some(head) } else { None },
    };
    let v = axiomere::verify_file(path, &policy);
    println!(
        "ok={} events={} signatures_checked={} signatures_failed={}",
        v.ok, v.events, v.signatures_checked, v.signatures_failed
    );
    if !v.unsigned_events.is_empty() {
        println!(
            "unsigned_events={}",
            v.unsigned_events
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    if !v.untrusted_actors.is_empty() {
        println!("untrusted_actors={}", v.untrusted_actors.join(","));
    }
    if v.ok {
        ExitCode::SUCCESS
    } else {
        println!(
            "break_seq={} reason={}",
            v.break_seq.map(|s| s.to_string()).unwrap_or_else(|| "-".into()),
            v.reason.as_deref().unwrap_or("-")
        );
        ExitCode::FAILURE
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("verify") {
        return match args.get(2) {
            Some(path) => verify_cmd(path, &args[3..]),
            None => {
                eprintln!("usage: axiomere verify <file> [--trusted actor=pubkey_hex ...] [--expect-head HASH] [--expect-events N] [--expect-root ROOT]");
                ExitCode::from(2)
            }
        };
    }
    let run = || -> Result<(), Box<dyn std::error::Error>> {
        match args.get(1).map(|s| s.as_str()) {
            Some("create") => {
                Ledger::create(&args[2])?;
                println!("created {}", args[2]);
            }
            Some("append") => {
                let mut l = open(&args[2])?;
                let payload: Value = serde_json::from_str(&args[4])?;
                let ev = l.append(&args[3], payload, args.get(5).map(|s| s.as_str()).unwrap_or("cli"))?;
                println!("{}", serde_json::to_string(&ev)?);
            }
            Some("snapshot") => {
                let mut l = open(&args[2])?;
                let content: Value = serde_json::from_str(&args[4])?;
                let ev = l.append_snapshot(&args[3], content, "live", None, None, args.get(5).map(|s| s.as_str()).unwrap_or("cli"))?;
                println!("{}", serde_json::to_string(&ev)?);
            }
            Some("dead") => {
                let mut l = open(&args[2])?;
                let ev = l.mark_dead(&args[3], &args[4], args.get(5).map(|s| s.as_str()).unwrap_or("cli"))?;
                println!("{}", serde_json::to_string(&ev)?);
            }
            Some("verify") => unreachable!("verify is dispatched before the closure"),
            Some("view") => {
                let l = Ledger::open(&args[2])?;
                let view = l.current_view(args.iter().any(|a| a == "--dead"))?;
                println!("{}", serde_json::to_string_pretty(&view)?);
            }
            Some("checkpoint") => {
                let mut l = open(&args[2])?;
                let ev = l.checkpoint(args.get(3).map(|s| s.as_str()).unwrap_or("cli"))?;
                println!("{}", serde_json::to_string(&ev)?);
            }
            _ => {
                eprintln!("usage: axiomere <create|append|snapshot|dead|verify|view|checkpoint> ...");
                return Ok(());
            }
        }
        Ok(())
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

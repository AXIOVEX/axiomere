//! Minimal CLI: create / append / snapshot / dead / verify / view / checkpoint.
//! Usage:
//!   axiomere create <file.axmr>
//!   axiomere append <file> <type> <payload-json> [actor]
//!   axiomere snapshot <file> <record-id> <content-json> [actor]
//!   axiomere dead <file> <record-id> <reason> [actor]
//!   axiomere verify <file>
//!   axiomere view <file> [--dead]
//!   axiomere checkpoint <file> [actor]
//! Signing: set AXIOMERE_SEED_HEX to a 32-byte hex seed to sign appends.

use axiomere::Ledger;
use serde_json::Value;
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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
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
            Some("verify") => {
                let l = Ledger::open(&args[2])?;
                let v = l.verify()?;
                println!("ok={} events={} signatures_checked={} signatures_failed={}", v.ok, v.events, v.signatures_checked, v.signatures_failed);
                if !v.ok {
                    return Err(format!("verification failed: {:?}", v.reason).into());
                }
            }
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

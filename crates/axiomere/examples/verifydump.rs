//! Baseline verdict dump (public API only): per file, the v0.1.0
//! verification outcome via read_events + verify_events, trusted-mode
//! outcome where a map is given, and Ledger::open accept/reject.
use axiomere::{read_events, verify_events, verify_events_trusted, Ledger};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // usage: verifydump [--trusted actor=pubhex ...] file...
    let mut trusted: BTreeMap<String, String> = BTreeMap::new();
    let mut files: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--trusted" {
            let kv = args[i + 1].split_once('=').unwrap();
            trusted.insert(kv.0.to_string(), kv.1.to_string());
            i += 2;
        } else {
            files.push(args[i].clone());
            i += 1;
        }
    }
    for f in &files {
        let open_ok = Ledger::open(f).is_ok();
        match read_events(f) {
            Ok(events) => {
                let v = verify_events(&events);
                let t = if trusted.is_empty() {
                    String::new()
                } else {
                    let tv = verify_events_trusted(&events, Some(&trusted));
                    format!(
                        " trusted_ok={} trusted_break={:?} trusted_reason={:?}",
                        tv.ok, tv.break_seq, tv.reason
                    )
                };
                println!(
                    "{}\tread=ok\tok={}\tevents={}\tbreak_seq={:?}\treason={:?}\tsig_checked={}\tsig_failed={}\topen_ok={}{}",
                    f, v.ok, v.events, v.break_seq, v.reason, v.signatures_checked,
                    v.signatures_failed, open_ok, t
                );
            }
            Err(e) => {
                println!("{}\tread=ERR({})\topen_ok={}", f, e, open_ok);
            }
        }
    }
}

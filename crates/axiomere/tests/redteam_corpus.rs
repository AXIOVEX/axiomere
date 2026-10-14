//! Spec 003 T303/T315 — the adopted red-team corpus as a regression
//! suite (AXM-003-14). This file deliberately uses ONLY the public
//! API surface that already existed in v0.1.0 (`read_events`,
//! `verify_events`, `verify_events_trusted`, `Ledger::open`), so the
//! suite can be run against the pre-fix tree unchanged: there, every
//! forgery expectation FAILS — which is the Constitution Article VI
//! demonstration that these tests can fail. (The `Verification`
//! struct gained fields in Spec 003; this suite touches only the
//! fields that already existed.)
//!
//! Expected verdicts live in the fixture manifest, recorded from the
//! T302 baselines: each entry carries the measured pre-fix outcome
//! (`prefix`) and the expected post-fix outcome (`postfix`).

use axiomere::{read_events, verify_events, verify_events_trusted, Ledger};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/redteam-2026-10-13")
}

fn manifest() -> Value {
    let raw = std::fs::read_to_string(corpus_dir().join("manifest.json")).unwrap();
    serde_json::from_str(&raw).unwrap()
}

fn trusted_map(m: &Value) -> BTreeMap<String, String> {
    m["trusted_map"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
        .collect()
}

#[test]
fn redteam_corpus_verdicts() {
    let m = manifest();
    let map = trusted_map(&m);
    let mut failures: Vec<String> = Vec::new();
    for entry in m["files"].as_array().unwrap() {
        let name = entry["file"].as_str().unwrap();
        let post = &entry["postfix"];
        let path = corpus_dir().join(name);
        // Semantic verdict over the leniently parsed events.
        match post["events_ok"] {
            Value::Null => {
                // The lenient reader itself must refuse this file.
                if read_events(&path).is_ok() {
                    failures.push(format!("{name}: expected lenient read to fail"));
                }
            }
            Value::Bool(want_ok) => match read_events(&path) {
                Ok(events) => {
                    let v = verify_events(&events);
                    if v.ok != want_ok {
                        failures.push(format!(
                            "{name}: events_ok want {want_ok}, got {} (reason {:?})",
                            v.ok, v.reason
                        ));
                    }
                    if let Some(want_reason) = post["events_reason"].as_str() {
                        if v.reason.as_deref() != Some(want_reason) {
                            failures.push(format!(
                                "{name}: events_reason want {want_reason}, got {:?}",
                                v.reason
                            ));
                        }
                    }
                    // Trusted-mode verdict where the manifest pins one.
                    if let Value::Bool(want_t) = post["trusted_ok"] {
                        let tv = verify_events_trusted(&events, Some(&map));
                        if tv.ok != want_t {
                            failures.push(format!(
                                "{name}: trusted_ok want {want_t}, got {} (reason {:?})",
                                tv.ok, tv.reason
                            ));
                        }
                        if let Some(want_tr) = post["trusted_reason"].as_str() {
                            if tv.reason.as_deref() != Some(want_tr) {
                                failures.push(format!(
                                    "{name}: trusted_reason want {want_tr}, got {:?}",
                                    tv.reason
                                ));
                            }
                        }
                    }
                }
                Err(e) => failures.push(format!("{name}: lenient read failed unexpectedly: {e}")),
            },
            _ => unreachable!(),
        }
        // Ledger::open gate.
        let want_open = post["open_ok"].as_bool().unwrap();
        let got_open = Ledger::open(&path).is_ok();
        if got_open != want_open {
            failures.push(format!("{name}: open_ok want {want_open}, got {got_open}"));
        }
    }
    assert!(failures.is_empty(), "corpus verdict mismatches:\n{}", failures.join("\n"));
}

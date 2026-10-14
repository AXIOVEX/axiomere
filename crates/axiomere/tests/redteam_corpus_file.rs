//! Spec 003 — the file-level half of the corpus suite: `verify_file`
//! under the default policy must return the manifest's file verdict
//! for every fixture, without ever raising (FR-VP-08, FR-VP-11 /
//! AXM-003-09, AXM-003-12). Malformed files are failed verdicts, not
//! exceptions — including the ones whose lenient read fails outright.

use axiomere::{verify_file, VerifyPolicy};
use serde_json::Value;
use std::path::PathBuf;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/redteam-2026-10-13")
}

#[test]
fn redteam_corpus_file_verdicts() {
    let raw = std::fs::read_to_string(corpus_dir().join("manifest.json")).unwrap();
    let m: Value = serde_json::from_str(&raw).unwrap();
    let policy = VerifyPolicy::default();
    let mut failures: Vec<String> = Vec::new();
    for entry in m["files"].as_array().unwrap() {
        let name = entry["file"].as_str().unwrap();
        let post = &entry["postfix"];
        let v = verify_file(corpus_dir().join(name), &policy);
        let want_ok = post["file_ok"].as_bool().unwrap();
        if v.ok != want_ok {
            failures.push(format!(
                "{name}: file_ok want {want_ok}, got {} (reason {:?}, break {:?})",
                v.ok, v.reason, v.break_seq
            ));
        }
        if let Some(want_reason) = post["file_reason"].as_str() {
            if v.reason.as_deref() != Some(want_reason) {
                failures.push(format!(
                    "{name}: file_reason want {want_reason}, got {:?}",
                    v.reason
                ));
            }
        }
        if !v.ok && v.reason.is_none() {
            failures.push(format!("{name}: failed without a named reason"));
        }
    }
    assert!(failures.is_empty(), "file verdict mismatches:\n{}", failures.join("\n"));
}

#[test]
fn verify_file_on_missing_file_returns_verdict_not_panic() {
    let v = verify_file(corpus_dir().join("does-not-exist.axmr"), &VerifyPolicy::default());
    assert!(!v.ok);
    assert_eq!(v.reason.as_deref(), Some("unreadable-file"));
}

#[test]
fn victim_unsigned_events_and_head_pin() {
    // F-VP-11's file-level half: the truncated prefix passes bare
    // verification and fails under the victim's recorded head pin.
    let m: Value =
        serde_json::from_str(&std::fs::read_to_string(corpus_dir().join("manifest.json")).unwrap())
            .unwrap();
    let head = m["victim_head_hash"].as_str().unwrap().to_string();
    let root = m["victim_checkpoint_root"].as_str().unwrap().to_string();
    let trunc = corpus_dir().join("surg-truncate.axmr");
    let bare = verify_file(&trunc, &VerifyPolicy::default());
    assert!(bare.ok, "bare verification of an honest prefix must pass");
    let pinned = verify_file(
        &trunc,
        &VerifyPolicy {
            trusted: None,
            expected_head: Some(axiomere::ExpectedHead {
                head_hash: Some(head.clone()),
                min_events: Some(6),
                checkpoint_root: Some(root.clone()),
            }),
        },
    );
    assert!(!pinned.ok, "pinned verification of the truncation must fail");
    // And the full victim file passes under its own pin.
    let victim = verify_file(
        corpus_dir().join("victim.axmr"),
        &VerifyPolicy {
            trusted: None,
            expected_head: Some(axiomere::ExpectedHead {
                head_hash: Some(head),
                min_events: Some(6),
                checkpoint_root: Some(root),
            }),
        },
    );
    assert!(victim.ok, "victim under its own pin must verify: {:?}", victim.reason);
    assert!(victim.unsigned_events.is_empty());
    assert_eq!(victim.signatures_checked, 6);
}

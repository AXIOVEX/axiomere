//! Spec 003 — policy-surface tests: the VerifyPolicy contract
//! (FR-VP-02/03/04/10), the write-path death rules (FR-VP-06), and
//! checkpoint recomputation against a control case (FR-VP-01).
//! Each test names the falsifier it implements.

use axiomere::{
    verify_events, verify_events_trusted, ExpectedHead, Ledger, VerifyPolicy,
};
use serde_json::json;
use std::collections::BTreeMap;

fn tmp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("axiomere-policy-{}-{}", std::process::id(), name));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{name}.axmr"))
}

fn seeded() -> ([u8; 32], String) {
    let (seed_hex, pub_hex) = axiomere::generate_keypair();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&hex::decode(&seed_hex).unwrap());
    (seed, pub_hex)
}

fn signed_ledger(path: &std::path::Path) -> (Ledger, String) {
    let (seed, pub_hex) = seeded();
    let mut l = Ledger::create(path).unwrap().with_signer(&seed);
    l.append_snapshot("r1", json!({"v": 1}), "live", Some(0.5), None, "alice")
        .unwrap();
    (l, pub_hex)
}

/// F-VP-3 control: an honestly *unsigned* ledger carries no signature
/// material, so presence enforcement does not fire — but the unsigned
/// state is reported, not silent.
#[test]
fn unsigned_ledger_is_visible_not_failed() {
    let path = tmp("unsigned-visible");
    let mut l = Ledger::create(&path).unwrap();
    l.append_snapshot("r1", json!({"v": 1}), "live", Some(0.5), None, "alice")
        .unwrap();
    let v = l.verify().unwrap();
    assert!(v.ok);
    assert_eq!(v.unsigned_events, vec![1]);
    assert_eq!(v.signatures_checked, 0);
}

/// F-VP-4: `None` and `Some(empty map)` are distinct inputs — an empty
/// map trusts nobody, so even the honest signer is untrusted.
#[test]
fn empty_trusted_map_trusts_nobody() {
    let path = tmp("empty-map");
    let (l, _pk) = signed_ledger(&path);
    let events = l.events().unwrap();
    let none = verify_events_trusted(&events, None);
    assert!(none.ok);
    let empty: BTreeMap<String, String> = BTreeMap::new();
    let v = verify_events_trusted(&events, Some(&empty));
    assert!(!v.ok);
    assert_eq!(v.reason.as_deref(), Some("untrusted-actor"));
    assert_eq!(v.untrusted_actors, vec!["alice".to_string()]);
}

/// F-VP-4: in trusted mode an unsigned event cannot establish
/// identity, even when its actor is mapped.
#[test]
fn trusted_mode_rejects_unsigned_events() {
    let path = tmp("trusted-unsigned");
    let (mut l, pk) = signed_ledger(&path);
    // second event appended without the signer (seedless reopen is
    // refused post-fix, so build the mixed file through a fresh
    // unsigned ledger sharing the first event is complex; instead
    // strip the second event's signature in the file text).
    l.append("note", json!({"x": 1}), "alice").unwrap();
    drop(l);
    let text = std::fs::read_to_string(&path).unwrap();
    let mut lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
    // remove sig/signer from the last event line textually
    let last = lines.pop().unwrap();
    let mut ev: serde_json::Value = serde_json::from_str(&last).unwrap();
    ev.as_object_mut().unwrap().remove("sig");
    ev.as_object_mut().unwrap().remove("signer");
    lines.push(axiomere::canonical(&ev));
    std::fs::write(&path, lines.join("\n") + "\n").unwrap();
    let events = axiomere::read_events(&path).unwrap();
    let mut map = BTreeMap::new();
    map.insert("alice".to_string(), pk);
    let v = verify_events_trusted(&events, Some(&map));
    assert!(!v.ok);
    assert_eq!(v.reason.as_deref(), Some("unsigned-event"));
    assert_eq!(v.unsigned_events, vec![2]);
}

/// F-VP-2 control: a checkpoint whose payload root was honestly
/// produced verifies; the recomputation is over the preceding events.
#[test]
fn checkpoint_recomputation_accepts_honest_checkpoint() {
    let path = tmp("ckpt-honest");
    let (mut l, _pk) = signed_ledger(&path);
    l.checkpoint("alice").unwrap();
    let v = l.verify().unwrap();
    assert!(v.ok, "honest checkpoint must verify: {:?}", v.reason);
}

/// F-VP-7 (write side, a): append_snapshot refuses a dead snapshot
/// without a reason — in the core, not only via mark_dead/wrappers.
#[test]
fn write_path_dead_requires_reason() {
    let path = tmp("dead-reason-write");
    let mut l = Ledger::create(&path).unwrap();
    l.append_snapshot("r1", json!({"v": 1}), "live", Some(0.5), None, "alice")
        .unwrap();
    let err = l
        .append_snapshot("r1", json!({"v": 1}), "dead", None, None, "alice")
        .unwrap_err();
    assert!(err.to_string().contains("non-empty reason"), "got: {err}");
    let err2 = l
        .append_snapshot("r1", json!({"v": 1}), "dead", None, Some(""), "alice")
        .unwrap_err();
    assert!(err2.to_string().contains("non-empty reason"), "got: {err2}");
}

/// F-VP-7 (write side, d): death is terminal at write time too.
#[test]
fn write_path_death_is_terminal() {
    let path = tmp("dead-terminal-write");
    let mut l = Ledger::create(&path).unwrap();
    l.append_snapshot("r1", json!({"v": 1}), "live", Some(0.5), None, "alice")
        .unwrap();
    l.mark_dead("r1", "withdrawn", "alice").unwrap();
    let err = l
        .append_snapshot("r1", json!({"v": 2}), "live", Some(0.9), None, "alice")
        .unwrap_err();
    assert!(err.to_string().contains("death is terminal"), "got: {err}");
    // A second death (corrected reason) remains possible.
    l.mark_dead("r1", "withdrawn — corrected reason", "alice")
        .unwrap();
    assert!(l.verify().unwrap().ok);
}

/// F-VP-11: each pin dimension fails with its own named reason, and
/// an unpinned verify of the same file still passes.
#[test]
fn expected_head_named_failures() {
    let path = tmp("head-pins");
    let (mut l, _pk) = signed_ledger(&path);
    l.checkpoint("alice").unwrap();
    let events = l.events().unwrap();
    let pin = |h: ExpectedHead| VerifyPolicy {
        trusted: None,
        expected_head: Some(h),
    };
    let v = axiomere::verify_events_with_policy(
        &events,
        &pin(ExpectedHead {
            head_hash: Some("f".repeat(64)),
            ..Default::default()
        }),
    );
    assert_eq!(v.reason.as_deref(), Some("head-mismatch"));
    let v = axiomere::verify_events_with_policy(
        &events,
        &pin(ExpectedHead {
            min_events: Some(99),
            ..Default::default()
        }),
    );
    assert_eq!(v.reason.as_deref(), Some("shortfall"));
    let v = axiomere::verify_events_with_policy(
        &events,
        &pin(ExpectedHead {
            checkpoint_root: Some("f".repeat(64)),
            ..Default::default()
        }),
    );
    assert_eq!(v.reason.as_deref(), Some("root-mismatch"));
    // Root pin compares against the RECOMPUTED root: the true root passes.
    let cp = events.last().unwrap();
    let true_root = cp.payload["merkle_root"].as_str().unwrap().to_string();
    let v = axiomere::verify_events_with_policy(
        &events,
        &pin(ExpectedHead {
            checkpoint_root: Some(true_root),
            ..Default::default()
        }),
    );
    assert!(v.ok);
    assert!(verify_events(&events).ok);
}

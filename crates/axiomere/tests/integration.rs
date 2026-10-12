//! Integration tests, built around seeded defects: each defect test
//! names the failure it exists to catch. A verifier that cannot fail
//! these is decorative.

use axiomere::{read_events, verify_events, Ledger};
use serde_json::json;
use std::io::Write;

fn tmp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("axiomere-test-{}-{}", std::process::id(), name));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{name}.axmr"))
}

fn sample(path: &std::path::Path) -> Ledger {
    let mut l = Ledger::create(path).unwrap();
    l.append_snapshot("rec-1", json!({"claim": "X holds", "score": 0.8}), "live", Some(0.8), None, "alice")
        .unwrap();
    l.append_snapshot("rec-1", json!({"claim": "X holds, revised", "score": 0.6}), "live", Some(0.6), None, "alice")
        .unwrap();
    l.append_snapshot("rec-2", json!({"claim": "Y fails"}), "live", Some(0.3), None, "bob")
        .unwrap();
    l
}

#[test]
fn chain_verifies_and_view_folds() {
    let path = tmp("fold");
    let l = sample(&path);
    let v = l.verify().unwrap();
    assert!(v.ok && v.events == 3);
    let view = l.current_view(false).unwrap();
    assert_eq!(view.len(), 2);
    assert_eq!(view["rec-1"]["content"]["claim"], "X holds, revised");
    // lineage: second snapshot's parent is the first snapshot's id
    assert_eq!(
        view["rec-1"]["parent"],
        json!(axiomere::snapshot_id(
            &json!({"claim": "X holds", "score": 0.8}),
            None
        ))
    );
}

#[test]
fn dead_is_append_only_and_scores_zero() {
    let path = tmp("dead");
    let mut l = sample(&path);
    l.mark_dead("rec-2", "retracted", "alice").unwrap();
    // history keeps everything, including the dead record and its death
    let events = read_events(&path).unwrap();
    assert_eq!(events.len(), 4);
    // current view excludes the dead; audit view includes it at 0.0
    let live = l.current_view(false).unwrap();
    assert!(!live.contains_key("rec-2"));
    let all = l.current_view(true).unwrap();
    assert_eq!(all["rec-2"]["status"], "dead");
    assert_eq!(all["rec-2"]["score"], json!(0.0));
    assert_eq!(all["rec-2"]["reason"], "retracted");
    assert!(l.verify().unwrap().ok);
}

#[test]
fn tampered_payload_breaks_verification() {
    let path = tmp("tamper");
    let _ = sample(&path);
    let text = std::fs::read_to_string(&path).unwrap();
    let forged = text.replace("X holds, revised", "X holds, FORGED!");
    std::fs::write(&path, forged).unwrap();
    let events = read_events(&path).unwrap();
    let v = verify_events(&events);
    assert!(!v.ok && v.reason.as_deref() == Some("hash-mismatch"));
}

#[test]
fn deleted_line_breaks_verification() {
    let path = tmp("delete");
    let _ = sample(&path);
    let lines: Vec<String> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(|s| s.to_string())
        .collect();
    let mut kept: Vec<&String> = lines.iter().collect();
    kept.remove(2); // drop the middle event
    let mut f = std::fs::File::create(&path).unwrap();
    for line in kept {
        writeln!(f, "{line}").unwrap();
    }
    let events = read_events(&path).unwrap();
    assert!(!verify_events(&events).ok);
}

#[test]
fn signed_appends_verify_and_forged_signature_fails() {
    let path = tmp("signed");
    let (seed_hex, _pk) = axiomere::generate_keypair();
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&hex::decode(&seed_hex).unwrap());
    let mut l = Ledger::create(&path).unwrap().with_signer(&seed);
    l.append("note", json!({"a": 1}), "alice").unwrap();
    let v = l.verify().unwrap();
    assert!(v.ok && v.signatures_checked == 1 && v.signatures_failed == 0);
    // forge: flip a character in the stored signature
    let text = std::fs::read_to_string(&path).unwrap();
    let ev = read_events(&path).unwrap().pop().unwrap();
    let sig = ev.sig.unwrap();
    let forged_sig = format!("{}{}", if sig.starts_with('a') { "b" } else { "a" }, &sig[1..]);
    std::fs::write(&path, text.replace(&sig, &forged_sig)).unwrap();
    let events = read_events(&path).unwrap();
    let v2 = verify_events(&events);
    assert!(!v2.ok && v2.reason.as_deref() == Some("signature-invalid"));
}

#[test]
fn checkpoint_root_matches_recomputation() {
    let path = tmp("checkpoint");
    let mut l = sample(&path);
    let before = read_events(&path).unwrap();
    let hashes: Vec<&str> = before.iter().map(|e| e.hash.as_str()).collect();
    let expected = axiomere::merkle_root(&hashes);
    let ev = l.checkpoint("alice").unwrap();
    assert_eq!(ev.payload["merkle_root"], json!(expected));
    assert!(l.verify().unwrap().ok);
}

#[test]
fn open_rejects_a_broken_log_for_appends() {
    let path = tmp("broken");
    let _ = sample(&path);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("Y fails", "Y passes")).unwrap();
    assert!(Ledger::open(&path).is_err());
}

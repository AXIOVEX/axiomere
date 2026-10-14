//! Spec 003 T303/T310 — the frozen encoding vectors (spec.md §5).
//! These pin the canonical encoding so a from-spec implementation can
//! reproduce the reference byte-for-byte (AXM-003-10). The fixture was
//! generated from the reference implementation and cross-checked
//! against the spec's measured values; if a test here fails, either
//! the implementation changed the encoding (a format change — not
//! permitted under Spec 003 constraint C2) or the fixture/spec drifted
//! and the drift must be resolved explicitly, never silently.

use axiomere::{canonical, merkle_root, snapshot_id};
use serde_json::{json, Value};

fn vectors() -> Value {
    let raw = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/vectors-003.json"),
    )
    .unwrap();
    serde_json::from_str(&raw).unwrap()
}

#[test]
fn string_escaping_vector() {
    let v = vectors();
    let input = v["string_escape"]["input"].as_str().unwrap();
    let got = canonical(&json!(input));
    assert_eq!(got, v["string_escape"]["canonical"].as_str().unwrap());
}

#[test]
fn event_body_and_hash_vectors() {
    let v = vectors();
    let b = &v["event_body"];
    let body = json!({
        "seq": b["seq"], "type": b["type"], "ts": b["ts"], "actor": b["actor"],
        "payload": b["payload"], "prev_hash": b["prev_hash"],
    });
    assert_eq!(canonical(&body), b["canonical"].as_str().unwrap());
    // Event hash = SHA-256(prev_hash ‖ canonical body), with
    // prev_hash entering as its 64-char lowercase hex text (§5).
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b["prev_hash"].as_str().unwrap().as_bytes());
    h.update(b["canonical"].as_str().unwrap().as_bytes());
    assert_eq!(hex::encode(h.finalize()), b["hash"].as_str().unwrap());
}

#[test]
fn float_rendering_vectors() {
    let v = vectors();
    for pair in v["float_renderings"].as_array().unwrap() {
        let (input, expect) = (pair[0].as_str().unwrap(), pair[1].as_str().unwrap());
        let parsed: Value = serde_json::from_str(input).unwrap();
        assert_eq!(canonical(&parsed), expect, "float rendering for {input}");
    }
}

#[test]
fn merkle_root_vectors() {
    let v = vectors();
    let m = &v["merkle_roots"];
    let aa = "aa".repeat(32);
    let bb = "bb".repeat(32);
    let cc = "cc".repeat(32);
    assert_eq!(merkle_root(&[]), m["empty"].as_str().unwrap());
    assert_eq!(merkle_root(&[&aa]), m["one_aa"].as_str().unwrap());
    assert_eq!(merkle_root(&[&aa, &bb]), m["two_aa_bb"].as_str().unwrap());
    assert_eq!(
        merkle_root(&[&aa, &bb, &cc]),
        m["three_aa_bb_cc"].as_str().unwrap()
    );
}

#[test]
fn snapshot_id_vectors() {
    let v = vectors();
    let s = &v["snapshot_ids"];
    let content = &s["content"];
    assert_eq!(
        snapshot_id(content, Some(&"d".repeat(64))),
        s["with_parent_dddd"].as_str().unwrap()
    );
    assert_eq!(
        snapshot_id(content, None),
        s["null_parent"].as_str().unwrap()
    );
}

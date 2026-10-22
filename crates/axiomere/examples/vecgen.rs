//! Generate the Spec 003 §5 encoding vectors from the reference impl.
use axiomere::{canonical, merkle_root, snapshot_id};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn sha(s: &str) -> String { hex::encode(Sha256::digest(s.as_bytes())) }

fn main() {
    // string escaping vector (spec §5)
    let s = "café ☕ \"q\" \\ \u{1}\u{1f}\n\t €";
    println!("string_vector_input_canon = {}", canonical(&json!(s)));
    // event body + hash vector
    let payload: Value = serde_json::from_str("{\"text\":\"café ☕\",\"x\":1e-7}").unwrap();
    let body = json!({"seq":1,"type":"note","ts":1792000000,"actor":"vect","payload":payload,"prev_hash":"0".repeat(64)});
    let cb = canonical(&body);
    println!("event_body_canonical = {cb}");
    println!("event_hash = {}", sha(&format!("{}{}", "0".repeat(64), cb)));
    // float renderings
    for (name, v) in [
        ("f_0_30000000000000004", json!(0.30000000000000004f64)),
        ("f_123456789_12345679", json!(123456789.12345679f64)),
        ("f_1_0", json!(1.0f64)),
        ("f_neg_zero", json!(-0.0f64)),
        ("f_1e-7", json!(1e-7f64)),
        ("f_5e-324", json!(5e-324f64)),
        ("f_1_0086398161248257e149", json!(1.0086398161248257e149f64)),
        ("f_1e21", json!(1e21f64)),
    ] { println!("{name} = {}", canonical(&v)); }
    // merkle vectors
    let aa = "aa".repeat(32); let bb = "bb".repeat(32); let cc = "cc".repeat(32);
    println!("merkle_empty = {}", merkle_root(&[]));
    println!("merkle_1 = {}", merkle_root(&[&aa]));
    println!("merkle_2 = {}", merkle_root(&[&aa, &bb]));
    println!("merkle_3 = {}", merkle_root(&[&aa, &bb, &cc]));
    // snapshot id vectors
    println!("snap_parent = {}", snapshot_id(&json!({"k":"v"}), Some(&"d".repeat(64))));
    println!("snap_null = {}", snapshot_id(&json!({"k":"v"}), None));
    // canon.axmr float spellings actually on disk
    let raw = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    for probe in ["1e-7", "1e-07", "1e+16", "1e+100", "5e-324"] {
        println!("disk_contains[{probe}] = {}", raw.contains(probe));
    }
}

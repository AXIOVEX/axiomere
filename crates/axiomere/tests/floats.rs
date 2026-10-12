//! Float canonicalization tests (Spec 002). Each test names the claim
//! it exists to falsify: a verifier that cannot fail these is
//! decorative (Constitution Article VI).
//!
//! Background: serde_json's default decimal->f64 parser was not
//! correctly rounded, so payload floats whose canonical form needs
//! >= 17 significant digits could re-read 1 ULP away from the hashed
//! bits and fail verification on an untampered log (AXM-002-01/02).

use axiomere::{canonical, read_events, verify_events, Ledger};
use serde_json::{json, Number, Value};

fn tmp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("axiomere-test-{}-{}", std::process::id(), name));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{name}.axmr"))
}

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// The exact value from the downstream report (Tekmerion E1,
/// 2026-10-12): a microsecond-precision Unix timestamp as f64.
const REPRO: f64 = 1791810119.6349843;

#[test]
fn reported_value_round_trips_through_reopen() {
    // AXM-002-04: append -> reopen -> verify, production path.
    let path = tmp("repro");
    {
        let mut l = Ledger::create(&path).unwrap();
        l.append(
            "probe.event",
            json!({"ts_like": REPRO, "plain": 0.5, "nested": {"v": REPRO}, "arr": [REPRO]}),
            "tester",
        )
        .unwrap();
    }
    let l = Ledger::open(&path).unwrap(); // open itself verifies
    let v = l.verify().unwrap();
    assert!(v.ok && v.events == 1);
    let events = l.events().unwrap();
    let bits = |v: &Value| v.as_f64().unwrap().to_bits();
    assert_eq!(bits(&events[0].payload["ts_like"]), REPRO.to_bits());
    assert_eq!(bits(&events[0].payload["nested"]["v"]), REPRO.to_bits());
    assert_eq!(bits(&events[0].payload["arr"][0]), REPRO.to_bits());
}

#[test]
fn defect_class_shape_matrix_round_trips() {
    // AXM-002-06: the characterized trigger class plus edge shapes.
    let values: [f64; 14] = [
        REPRO,
        f64::from_bits(0x5edf8d85dcc5b771), // 1.0086398161248257e+149 — fuzz-found failure
        f64::from_bits(0xe5b49f9f1e19b449), // -8.55776243443869e+181 — fuzz-found failure
        f64::from_bits(0x4b8ae9556619490b), // 8.248338285090229e+55 — fuzz-found failure
        5e-324,                             // minimum subnormal
        -5e-324,
        f64::MIN_POSITIVE,                  // minimum normal
        f64::MAX,
        0.0,
        -0.0,
        1e16,                               // integral-valued, exponent rendering
        9007199254740992.0,                 // 2^53
        0.1,
        std::f64::consts::PI,
    ];
    let path = tmp("shapes");
    {
        let mut l = Ledger::create(&path).unwrap();
        for (i, v) in values.iter().enumerate() {
            l.append("shape", json!({"i": i, "v": v}), "tester").unwrap();
        }
    }
    let l = Ledger::open(&path).unwrap();
    assert!(l.verify().unwrap().ok);
    let events = l.events().unwrap();
    for (i, v) in values.iter().enumerate() {
        assert_eq!(
            events[i].payload["v"].as_f64().unwrap().to_bits(),
            v.to_bits(),
            "shape {i} changed bits across reopen"
        );
    }
}

#[test]
fn directed_decimals_parse_correctly_rounded() {
    // AXM-002-01/06: decimal texts whose correctly-rounded values are
    // known independently (Python / IEEE ties-to-even). The old parser
    // got every one of these wrong by 1 ULP.
    let cases: [(&str, u64); 4] = [
        ("1791810119.6349843", 0x41dab33691e8a395),
        ("1.0086398161248257e+149", 0x5edf8d85dcc5b771),
        // 2^53 + 1 is an exact tie; ties-to-even selects 2^53.
        ("9007199254740993", 0x4340000000000000),
        // 1 + 2^-53 exactly: tie between 1.0 and its successor; even wins.
        ("1.00000000000000011102230246251565404236316680908203125", 0x3ff0000000000000),
    ];
    for (text, want_bits) in cases {
        let v: Value = serde_json::from_str(text).unwrap();
        assert_eq!(v.as_f64().unwrap().to_bits(), want_bits, "parse of {text}");
    }
}

#[test]
fn canonical_format_parse_sweep_is_identity() {
    // AXM-002-05/06: canonical rendering is byte-stable and parse is
    // its exact inverse — 250,000 deterministic random bit patterns
    // (the pre-fix failure rate on this distribution was ~30%, so a
    // zero here cannot be luck). The full 2,000,000-pattern sweep is
    // recorded in the spec's assessment evidence.
    let mut state: u64 = 0x1234_5678_9abc_def1;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545F4914F6CDD1D)
    };
    let mut checked = 0u32;
    for _ in 0..250_000 {
        let bits = next();
        let f = f64::from_bits(bits);
        if !f.is_finite() {
            continue;
        }
        let num = Number::from_f64(f).unwrap();
        let text = canonical(&Value::Number(num));
        let back: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(back.as_f64().unwrap().to_bits(), bits, "round trip of {text}");
        assert_eq!(canonical(&back), text, "canonical form moved for {text}");
        checked += 1;
    }
    assert!(checked > 200_000);
}

#[test]
fn legacy_safe_fixture_verifies_unchanged() {
    // AXM-002-03: generated with the PRE-FIX build; verified then,
    // must verify identically now (same events, signatures intact).
    let events = read_events(fixture("legacy-floats-safe.axmr")).unwrap();
    let v = verify_events(&events);
    assert!(v.ok, "safe legacy fixture broke: {:?}", v.reason);
    assert_eq!(v.events, 3);
    assert_eq!(v.signatures_checked, 3);
    assert_eq!(v.signatures_failed, 0);
}

#[test]
fn legacy_defect_fixture_now_verifies_and_is_appendable() {
    // AXM-002-04: generated with the PRE-FIX build; failed then with
    // hash-mismatch at seq 1 — its only defect. Now it verifies, and
    // (copied to scratch, fixture untouched) accepts new appends.
    let src = fixture("legacy-floats-defect.axmr");
    let events = read_events(&src).unwrap();
    let v = verify_events(&events);
    assert!(v.ok, "defect fixture still fails: {:?}", v.reason);
    assert_eq!(v.events, 1);

    let path = tmp("defectcopy");
    std::fs::copy(&src, &path).unwrap();
    let mut l = Ledger::open(&path).unwrap();
    l.append("probe.followup", json!({"ok": true}), "tester").unwrap();
    let v2 = l.verify().unwrap();
    assert!(v2.ok && v2.events == 2);
}

#[test]
fn altered_float_digit_fails_verification() {
    // AXM-002-07: the fix must not teach the verifier to accept
    // tampering. One digit changed in a defect-class float payload.
    let path = tmp("tamperfloat");
    {
        let mut l = Ledger::create(&path).unwrap();
        l.append("probe.event", json!({"ts_like": REPRO}), "tester").unwrap();
    }
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("1791810119.6349843"));
    let tampered = text.replacen("1791810119.6349843", "1791810119.6349844", 1);
    let tpath = tmp("tamperfloat-bad");
    std::fs::write(&tpath, tampered).unwrap();
    let events = read_events(&tpath).unwrap();
    let v = verify_events(&events);
    assert!(!v.ok);
    assert_eq!(v.reason.as_deref(), Some("hash-mismatch"));
    // Control: the untampered original verifies.
    let events = read_events(&path).unwrap();
    assert!(verify_events(&events).ok);
}

#[test]
fn snapshot_content_float_bits_survive_reopen() {
    // AXM-002-04: the snapshot path (content + score) carries floats
    // through the same canonical body; bits must survive.
    let path = tmp("snapfloat");
    {
        let mut l = Ledger::create(&path).unwrap();
        l.append_snapshot(
            "rec-1",
            json!({"reading": REPRO, "label": "defect-class content"}),
            "live",
            Some(0.675),
            None,
            "alice",
        )
        .unwrap();
    }
    let l = Ledger::open(&path).unwrap();
    assert!(l.verify().unwrap().ok);
    let view = l.current_view(false).unwrap();
    assert_eq!(
        view["rec-1"]["content"]["reading"].as_f64().unwrap().to_bits(),
        REPRO.to_bits()
    );
}

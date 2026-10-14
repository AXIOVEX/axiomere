//! Spec 003 T311 — run the from-spec verifier (tests/from_spec_verify.py),
//! an independent Python-stdlib reader written only from DESIGN §4 and
//! the frozen vectors. It must reproduce every vector and verify the
//! honest in-repo fixtures (victim.axmr, canon.axmr). Skips when no
//! python3 interpreter is available; the script itself is the artifact
//! and can be run directly against any ledger files.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn from_spec_verifier_reproduces_vectors_and_honest_fixtures() {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("from_spec_verify.py");
    let out = match Command::new("python3").arg(&script).output() {
        Ok(o) => o,
        Err(_) => {
            eprintln!("python3 unavailable — from-spec run skipped");
            return;
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "from-spec verifier failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("vectors: all reproduced"), "{stdout}");
    assert!(stdout.contains("OK   victim.axmr"), "{stdout}");
    assert!(stdout.contains("OK   canon.axmr"), "{stdout}");
}

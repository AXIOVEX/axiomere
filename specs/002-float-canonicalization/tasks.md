# Spec 002 — Tasks

- [x] T201 Reproduce the defect end-to-end (Python package: append → in-session verify false → reopen raises hash-mismatch) and at the primitive level (parse/format probes).
- [x] T202 Characterize the trigger class (bit-pattern sweep, shape sweep, ±1 ULP error bound, 17-significant-digit onset) — spec §2.
- [x] T203 Generate legacy fixtures with the pre-fix build (safe + defect) and record pre-fix verification baselines, incl. the 5 real Tekmerion study ledgers.
- [x] T204 Claims bundle (`aee-claims.json`) + after_specify assessment.
- [ ] T205 Enable serde_json `float_roundtrip` in `crates/axiomere` and `crates/axiomere-py`.
- [ ] T206 Regression tests: reproduction round trip (Rust + bindings smoke), legacy fixtures, shape matrix, canonical-stability sweep, float-digit tamper.
- [ ] T207 Real-ledger evidence run on copies (Tekmerion study + bundles) with the post-fix build; outcomes compared to T203 baselines.
- [ ] T208 DESIGN §4 number-rule clarification (canonical rendering + correctly-rounded parsing requirement).
- [ ] T209 Full suite green locally + branch CI green by read-back; after_implement assessment; claims updated on evidence only.

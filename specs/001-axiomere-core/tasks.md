# Spec 001 — Tasks (as-built, all complete at v0.1)

- [x] T101 Core ledger: event model, canonical JSON, SHA-256 hash chain, fsync appends.
- [x] T102 Snapshots: content addressing, parent lineage, current-view fold.
- [x] T103 Dead semantics: `mark_dead` (status + reason + score 0.0); audit view includes the dead.
- [x] T104 Ed25519 signed appends + verification (embedded keys; trusted-key map).
- [x] T105 Merkle checkpoints over event ranges.
- [x] T106 CLI: create / append / snapshot / dead / verify / view / checkpoint.
- [x] T107 Seeded-defect integration tests (7): tamper, line deletion, forged signature, broken-log open rejection, lineage, dead, checkpoint recomputation.
- [x] T108 PyO3 bindings crate with crate-local pyproject (module `_axiomere`); maturin build verified.
- [x] T109 Docs: README, DESIGN (incl. §6 erasure-certificate protocol, specified not implemented), NAMING (uniqueness evidence).
- [x] T110 CI: `cargo test -p axiomere` + `cargo check -p axiomere-py` green on GitHub.
- [ ] T111 External anchoring transport (layer 5) — future spec; specified in DESIGN §3/§5, honestly labeled unimplemented.
- [ ] T112 Payload encryption at ingest + erasure-certificate execution (DESIGN §6) — future spec.

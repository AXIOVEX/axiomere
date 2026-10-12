# Spec 001 — Axiomere Core v0.1 (as-built record)

**Date:** 2026-10-12 · **Status:** Implemented (v0.1, shipped 2026-10-11) · **Process note:** this spec was written in the 2026-10-12 spec-kit + AEE catch-up, after the v0.1 build — the deviation is recorded here and in the assessment, per Constitution Article VII. From Spec 002 onward, claims precede code.

## 1. What was built

A Rust core crate (`axiomere`) implementing a truth ledger: one `.axmr` file per ledger, events appended as canonical-JSON lines under a SHA-256 hash chain; records as content-addressed snapshot lineages; withdrawal as dead snapshots (status + reason + score 0.0); Ed25519-signed appends; Merkle checkpoints over event ranges; a minimal CLI; and PyO3 bindings (`crates/axiomere-py`, module `_axiomere`) consumed by the separate AXIOVEX/axiomere-python package.

## 2. Requirements

- **FR-AXM-01** The public API contains no update, delete, remove, or redact operation.
- **FR-AXM-02** Every event carries seq, prev_hash, and hash = SHA-256(prev_hash ‖ canonical body); verification names the first failing seq and reason.
- **FR-AXM-03** Snapshot IDs are content addresses: SHA-256(content + parent snapshot ID); lineage is traversable and tamper-evident.
- **FR-AXM-04** `mark_dead` appends a snapshot with status dead, a mandatory reason, and score 0.0; dead records remain fully readable in the audit view.
- **FR-AXM-05** Appends signed with an actor's Ed25519 key verify against the embedded public key (and, when supplied, a trusted-key map); a forged signature fails verification.
- **FR-AXM-06** A checkpoint appends a Merkle root over all event hashes to date, recomputable independently.
- **FR-AXM-07** `Ledger::open` refuses a log that fails verification: a broken record is evidence, never a writing surface.
- **FR-AXM-08** The `.axmr` format is documented completely enough for independent verification (DESIGN §4).
- **FR-AXM-09** The crate builds the `_axiomere` PyO3 extension via maturin, with the crate-local pyproject pinning the module name.

## 3. Non-goals (v0.1)

No query language, secondary indexes, networking/server mode, multi-writer concurrency, compression, external-anchor transport, or payload encryption (all specified in DESIGN §7/§6 as later work, and labeled as such).

## 4. Verification

7 integration tests in `crates/axiomere/tests/integration.rs`, including seeded defects (tampered payload, deleted line, forged signature, broken-log open rejection); `cargo test` green locally and in GitHub CI. Bindings smoke-verified from Python (signed snapshot, dead semantics, signature-checked verify).

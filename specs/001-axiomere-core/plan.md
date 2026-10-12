# Spec 001 — Plan (as-built)

**Date:** 2026-10-12 · Companion to `spec.md`. The design authority is `docs/DESIGN.md` (v0.1 design, 2026-10-11), which itself implements the owner's DN-005 decision recorded in the Tekmerion repository.

## Build order (as executed)

```
Core ledger (Event model, canonical JSON, hash chain)        [lib.rs]
Snapshots + content addressing + dead semantics               [lib.rs]
Ed25519 signing (ed25519-dalek) + verification incl. trusted map
Merkle checkpoints                                            [merkle_root + checkpoint]
CLI (create/append/snapshot/dead/verify/view/checkpoint)      [src/bin/axiomere.rs]
Seeded-defect integration tests                               [tests/integration.rs]
PyO3 bindings crate (_axiomere)                               [crates/axiomere-py]
Docs (README, DESIGN, NAMING) + CI (cargo test, cargo check)  [.github/workflows/ci.yml]
```

## Decisions recorded

- Canonical JSON (sorted keys, compact separators) is the hashing substrate — two writers can never disagree about the bytes of the same event (DESIGN §4).
- One writer per ledger (single sequencing point); concurrency is an embedding concern, not a core feature (§7 non-goals).
- The bindings crate carries its own `pyproject.toml` pinning the module name `_axiomere` — without it, maturin adopts metadata from ancestor projects and mis-names the wheel (caught in CI integration with axiomere-python, 2026-10-11).

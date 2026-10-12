<h1 align="center">Axiomere</h1>

<p align="center">
  <strong>A truth ledger — the record store where nothing is ever deleted.</strong>
</p>

<p align="center">
  🌐 <a href="https://axiovexsystems.com"><strong>axiovexsystems.com</strong></a>
  &nbsp;·&nbsp; <a href="docs/DESIGN.md">Design</a>
  &nbsp;·&nbsp; <a href="docs/NAMING.md">Naming record</a>
</p>

Axiomere is an append-only, signed, hash-chained record store designed for **truthfulness, provenance, and change management**. A conventional database's core verbs are *insert, update, delete*. Axiomere removes two of them — permanently, by construction: its API contains no update and no delete. Records change only by appending new **snapshots** of older records, git-style. A withdrawn record is not erased; it is declared **dead**, with its reason on the record and its score set to 0.0, in full view of history.

Every event is hash-chained, every snapshot is content-addressed to its parent, appends can be Ed25519-signed by their actor, and Merkle checkpoints let a whole history be committed to one root — ready to anchor outside the operator's control, where rewriting history becomes detectable by anyone.

## Quickstart

```rust
use axiomere::Ledger;
use serde_json::json;

let mut ledger = Ledger::create("study.axmr")?;
ledger.append_snapshot("claim-1", json!({"text": "X holds", "score": 0.8}),
                       "live", Some(0.8), None, "researcher")?;
ledger.append_snapshot("claim-1", json!({"text": "X holds, narrowed", "score": 0.6}),
                       "live", Some(0.6), None, "researcher")?;   // supersedes — the original stays
ledger.mark_dead("claim-1", "retracted", "adjudicator")?;          // dead at 0.0 — still readable, never deleted

assert!(ledger.verify()?.ok);
let live = ledger.current_view(false)?;   // dead records excluded
let audit = ledger.current_view(true)?;   // everything, with reasons
```

Python: the native bindings live in `crates/axiomere-py` (PyO3), and the idiomatic Python module is its own repository — [AXIOVEX/axiomere-python](https://github.com/AXIOVEX/axiomere-python).

CLI: `axiomere create|append|snapshot|dead|verify|view|checkpoint file.axmr …` (set `AXIOMERE_SEED_HEX` to sign).

## Format

One file, extension **`.axmr`**, magic header `AXMR1`, one canonical-JSON event per line. Inspectable with generic tools; verifiable by independent implementations byte-for-byte. Spec: [docs/DESIGN.md](docs/DESIGN.md) §4.

## Status

**v0.1 (October 2026).** Core ledger, snapshots, dead semantics, signatures, checkpoints, CLI, and Python bindings are implemented and tested — including seeded-defect tests: tampered payloads, deleted lines, and forged signatures must fail verification, and do. External-anchor transport and payload encryption (for the narrow erasure-certificate protocol, DESIGN §6) are specified, not yet implemented, and are labeled as such wherever mentioned.

## Why the name

*Axiomere* = axiom + Greek *meros*, a unit: one indivisible unit of asserted truth; a ledger is units chained in order. Uniqueness checks (crates.io, PyPI, GitHub, web) and the rejected candidates are recorded in [docs/NAMING.md](docs/NAMING.md). Axiomere is the store beneath **Tekmerion**, Axiovex's defensible-research framework.

## License

MIT. © 2026 Axiovex Systems, LLC. See [LICENSE](LICENSE).

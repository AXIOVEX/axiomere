# Spec 003 — Plan

**Date:** 2026-10-13 · Companion to `spec.md`. The defect characterization (spec §1) is probe-verified evidence from the 2026-10-13 red team, not conjecture; the design rule throughout is the smallest change that makes the reader check what the writer already guarantees.

## Build order

```
Spec package: spec.md / plan.md / tasks.md / aee-claims.json   [this directory]
  + after_specify assessment (assessor run on the draft claims)
Baselines BEFORE code (T302): v0.1.0 outcomes for the Tekmerion
  compatibility corpus + expected-verdict manifest for the
  red-team corpus, drafted from rt-b/outputs
Fixtures (T303): rt-b corpus adopted (force-added past the *.axmr
  gitignore — Spec 002's lesson), encoding vectors frozen (§5)
Core policy object + result extensions (T304)
  VerifyPolicy { trusted, expected_head }; Verification gains
  unsigned_events + untrusted_actors
Core checks, structural first (T305–T310):
  checkpoint recomputation → signature presence → trusted
  strictness → validating fold (snapshot lineage, dead rules)
  → write-path dead rules → strict file reader → expected head
From-spec verifier (T311): an independent reader written from
  DESIGN §4 + vectors only, run against the honest corpus
Compatibility run (T312): both corpora on copies, diffed
  against T302 baselines; exceptions named, never absorbed
CLI (T313) → Bindings (T314): policy + full result shape,
  py_to_json fidelity, verify_file no-raise
Red-team probe re-runs + Article VI defective-build runs (T315)
Docs: DESIGN §4 pinned encoding + policy/result documentation (T316)
after_implement assessment + claim updates on evidence only (T317)
Release: 0.2.0 (behavior change, format unchanged), CHANGELOG,
  tag on the exact build commit — owner's standing semver rule
```

## Decisions recorded

- **Structural checks are unconditional; only trust and pins are policy.** Checkpoint recomputation, signature presence, snapshot/death validation, and canonical bytes restate the reference writer's own guarantees, so they hold in every verification (spec constraint C1). The caller still chooses whose keys to trust (trusted map) and what head to pin — those were always caller knowledge. This split is what lets the defaults change safely: nothing honest changes outcome.
- **Byte enforcement, not a docs correction (AXM-003-09).** RTB-08/09's root is *parsed ≠ inspected*. Byte equality between each line and `canonical(event)` is also the mechanism that subsumes duplicate keys and unknown fields — the canonical form contains each key once and only the format's fields, so no separate scanning pass exists to get wrong.
- **Death is terminal (FR-VP-06d).** Decided in this spec rather than left as the red team's open "decide and document": a silent resurrection path contradicts Article II's "withdrawal is death, in the open". Honest history is protected by C1's named-exception rule, not by keeping the hole.
- **Non-finite values are errors at the bindings (FR-VP-07).** This takes the decision Spec 002 §6 explicitly deferred. The core's text path cannot represent non-finite values at all; the bindings mapping them to `null` made the Python surface *less* faithful than the core it wraps, silently. Loud beats silent for a truth ledger.
- **The pinned exponent rule was measured, not assumed.** §5's vectors were generated from the reference implementation before this spec was finalized — and the measurement falsified the drafter's first assumption (positive exponents are signed: `1e+21`, `1.0086398161248257e+149`). A spec written from intuition would have shipped the very RTB-10 class of error it exists to close; the vectors in T303 are therefore generated artifacts, never hand-typed values.
- **No new magic version.** The files do not change (C2); strictness is a property of the reader. `AXMR2` would fragment the corpus to label a reading discipline.
- **`verify_file` stops raising; `Ledger::open` keeps raising.** Standalone verification is a question ("does this file verify?") and must always have an answer; opening for appends is a gate and must fail hard (C3). Conflating the two is RTB-13; separating them is the fix.
- **Companions are specified, not executed, here (AXM-003-15).** Tekmerion's payload-root reads (freeze.py, export.py) and axiomere-python's idiomatic surface are real obligations with real owners in their own repos — the Spec 002 §7 pattern, which kept the float fix honest about what it had not yet fixed downstream.

# Spec 002 — Plan

**Date:** 2026-10-12 · Companion to `spec.md`. The defect characterization (spec §2) and the validity-preservation argument (spec §3) were established before this plan; the fix is deliberately the smallest change that makes the reader the exact inverse of the writer.

## Build order

```
Characterization probes (scratch, deleted after use)          [done pre-spec]
Legacy fixtures generated with the PRE-FIX build              [tests/fixtures/]
  legacy-floats-safe.axmr    (verifies today; signed, checkpointed)
  legacy-floats-defect.axmr  (fails today: hash-mismatch, seq 1)
Claims bundle + after_specify assessment                      [this directory]
Fix: serde_json float_roundtrip feature                       [crates/*/Cargo.toml]
Regression tests (floats.rs): repro, fixtures, shape matrix,
  canonical stability, sweep, float-digit tamper              [crates/axiomere/tests/]
Real-ledger evidence run (copies; originals untouched)        [recorded in assessment]
DESIGN §4 number-rule clarification                           [docs/DESIGN.md]
after_implement assessment + claim updates                    [this directory]
```

## Decisions recorded

- **Parser fix, not format fix.** The canonical bytes were never wrong; changing them would invalidate history (FR-FLT-01). The only lever the constraint leaves is the reader, and it is sufficient: a correctly-rounded parser inverts the Ryū writer exactly.
- **`float_roundtrip` over a hand-rolled parser.** Correct rounding is a solved problem with an upstream, maintained implementation; a local re-implementation is the likeliest way to trade this defect for a subtler one.
- **Feature enabled in both crates explicitly.** `crates/axiomere-py` must not silently depend on Cargo feature unification through the core crate; its own `serde_json` dependency names the feature too.
- **Fixtures before fix.** Both legacy fixtures were generated with the pre-fix build and committed with the spec, so the validity tests compare against genuinely pre-fix artifacts rather than reconstructions.
- **Performance cost accepted.** Correctly-rounded parsing is ~2× slower on float-heavy input (serde_json's own documentation). Verification is an audit operation; its correctness is the product.

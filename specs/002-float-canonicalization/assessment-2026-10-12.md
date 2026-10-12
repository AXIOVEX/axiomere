# AEE Assessment — Spec 002 Float Canonicalization (after_specify)

**Date:** 2026-10-12 · **Tooling:** spec-kit 1.0.10, AEE extension v1.2.0, AEE engine 1.0.4, threshold 0.70
**Claims assessed:** 8 (`aee-claims.json` v0.1.0) · **Machine record:** `aee-assessment.json`
**Outcome: ITERATE** — the characterization claims are fully evidenced; the fix claims carry their before-states and await implementation evidence.

| Claim | Score | State |
|-------|-------|-------|
| AXM-002-01 defect is reader-side (non-correctly-rounded parse) | 1.00 | supported — reproduction + primitive probes |
| AXM-002-02 trigger class: ≥ 17 significant digits, ±1 ULP | 1.00 | supported — bit-pattern + shape sweeps |
| AXM-002-03 validity preservation (hard constraint) | 1.00 | draft — injectivity argument + pre-fix baselines recorded; the demonstration (fixtures + real ledgers post-fix) is T206/T207 |
| AXM-002-04 defect-class logs verify post-fix | 0.90 | draft — before-state frozen in the defect fixture; repair test is T206 |
| AXM-002-05 format + canonical encoding unchanged | 0.48 | draft — asserted by design (§5); demonstrated by canonical-stability sweep + diff at T206/T209 |
| AXM-002-06 round-trip totality | 0.90 | draft — pre-fix baseline (593,459 / 2,000,000 failures) is the number the post-fix sweep must zero |
| AXM-002-07 tamper detection undiminished | 0.48 | draft — asserted; demonstrated by the seeded-defect suite + float-digit tamper test at T206 |
| AXM-002-08 axiomere-python: companion item, no code change | 1.00 | supported — bindings source inspection |

## Reading

Claims preceded code, per Article VII — the sequence this repo adopted after Spec 001's recorded deviation. The two sub-threshold claims (002-05, 002-07) are exactly the ones whose evidence can only exist after the change lands; they are stated now so the implementation is falsifiable against them, not retro-fitted to whatever the implementation happens to do. AXM-002-03's score reflects its argument and baselines; its bundle status stays **draft** until T207's real-ledger run demonstrates it — a score is not a demonstration.

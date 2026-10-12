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

---

# Addendum — after_implement (2026-10-12)

**Outcome: ITERATE** (assessor's standing outcome label; every claim is at/above threshold and marked supported on shipped evidence — the after_specify machine record is preserved as `aee-assessment-after_specify.json`; this phase's is `aee-assessment.json`, claims bundle now v0.2.0).

| Claim | Score | Evidence that moved it |
|-------|-------|------------------------|
| AXM-002-03 validity preservation | 1.00 | Real-ledger run on copies with the fixed build: Tekmerion study + bundle-v1..v4 → (ok, 20/11/14/18/20 events), identical to pre-fix baselines on every file; signed safe fixture identical (3 events, 3 signatures checked) |
| AXM-002-04 defect repaired | 1.00 | Defect fixture verifies + is appendable again; repro round-trips in Rust tests and through the Python bindings (append → open → verify, bits exact) |
| AXM-002-05 format unchanged | 1.00 | Canonical text byte-identical across the 250k-pattern sweep; delivered diff touches two Cargo.toml feature flags, tests, docs — not `canonical()`, the magic, or the hash construction |
| AXM-002-06 round-trip totality | 1.00 | Full release sweep, same seed as baseline: **0 failures / 1,998,976** finite f64 (was 593,459 / 2,000,000); shape matrix + directed decimals (incl. ties-to-even) green |
| AXM-002-07 tamper undiminished | 1.00 | New float-digit tamper test fails with `hash-mismatch`; Spec 001 seeded-defect suite 7/7 green post-fix |
| AXM-002-01 / -02 / -08 | 1.00 / 1.00 / 1.00 | Unchanged — characterization and repo-boundary claims, evidenced at specify time |

**Suite:** 15/15 locally (`cargo test -p axiomere`: 8 floats + 7 integration) and branch CI green by read-back (run on tip `7b60ba6`, PR #1).

**Process incidents, recorded not hidden:** (1) The first branch CI run failed — the legacy fixtures, present locally, had never been committed: the repo's `.gitignore` carries `*.axmr`, and the spec commit's `git add` skipped them silently. Local tests passed against untracked files — a false-green this assessment does not count; the CI run is the evidence of record. Fixtures are now force-added as deliberate artifacts. (2) The first implementation commit also swept in a 540-file scratch evidence venv via `git add -A`; the branch (unmerged, hours old) was rewritten to exclude it and force-pushed with lease, and `.venv-evidence/` is now ignored. Both incidents are why the final tree — not intermediate states — is what the CI read-back above certifies.

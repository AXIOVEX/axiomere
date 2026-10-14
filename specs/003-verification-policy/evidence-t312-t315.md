# Spec 003 — Build evidence: T312 compatibility, T311 from-spec, T315 runs

Recorded 2026-10-13 during the build on branch `build/003-verification-policy`.
Raw outputs are preserved under `~/workspace/remediation-2026-10-13/evidence-003/`
(baselines in `~/workspace/remediation-2026-10-13/baseline-*.txt`).

## T312 — Compatibility run (AXM-003-13)

Method: the post-fix CLI (`axiomere verify`, release build of the build
branch) was run over copies of all 17 corpus files; `(ok, events)` compared
with the T302 pre-fix baselines (`baselines-t302.md`).

| File | Pre-fix (ok, events) | Post-fix (ok, events) | Match |
|---|---|---|---|
| tekmerion-eval-study.axmr | true, 45 | true, 45 | identical |
| tekmerion-eval-bundle-v1.axmr | true, 41 | true, 41 | identical |
| tekmerion-eval-bundle-v2.axmr | true, 45 | true, 45 | identical |
| tekmerion-e6-audit.axmr | true, 8 | true, 8 | identical |
| phase5-study.axmr | true, 20 | true, 20 | identical |
| phase5-bundle-v1.axmr | true, 11 | true, 11 | identical |
| phase5-bundle-v2.axmr | true, 14 | true, 14 | identical |
| phase5-bundle-v3.axmr | true, 18 | true, 18 | identical |
| phase5-bundle-v4.axmr | true, 20 | true, 20 | identical |
| math-study.axmr | true, 95 | true, 95 | identical |
| math-bundle-v1.axmr | true, 93 | true, 93 | identical |
| math-bundle-v2.axmr | true, 95 | true, 95 | identical |
| zenodo-23311479.axmr | true, 11 | true, 11 | identical |
| zenodo-23317023.axmr | true, 18 | true, 18 | identical |
| zenodo-23317259.axmr | true, 93 | true, 93 | identical |
| zenodo-23319126.axmr | true, 41 | true, 41 | identical |
| osf-84dk7-study.axmr | true, 45 | true, 45 | identical |

**Result: 17/17 identical. No exceptions, no migrations.** (All 17 are
unsigned ledgers; the post-fix result now also reports
`unsigned_events = all seqs` for each — the visible-state reporting of
FR-VP-02 — while `ok` and the event count are unchanged.)

## T311 — From-spec verifier (AXM-003-10 / FR-VP-09)

`crates/axiomere/tests/from_spec_verify.py` — an independent reader in pure
Python stdlib, written only from DESIGN §4 (as expanded by T316) and the
frozen vectors; it shares no code with the Rust core (canonical JSON,
chain hashes, checkpoint recomputation, snapshot lineage/death rules,
signature presence, and a pure-Python Ed25519 check are all
reimplemented from the document).

Observed run (2026-10-13):

- Every frozen vector reproduced: string escaping, the §5 event body and
  its hash `cbaa00f5…`, all 8 float renderings (incl. `1e-7`, `5e-324`,
  `1e+21`, `1.0086398161248257e+149`), Merkle roots for 0/1/2/3 leaves,
  snapshot ids with parent (`1a270a90…`) and null parent (`ed346a25…`).
- Honest corpus verified: `victim.axmr` (6 events, 6 signatures checked
  in pure Python), `canon.axmr`, and **all 17 compatibility-corpus
  ledgers** (T312 table above), each `OK` with matching event counts.
- The run is wired into the Rust suite as `tests/from_spec.rs` (skips
  only if no python3 interpreter exists).

## T315 — Probe re-runs, Article VI, differential

### Probe re-runs (Team B probes against the post-fix build)

Method: the probe scripts were re-executed unmodified in a fresh venv
holding the post-fix bindings (maturin build of this branch) plus the
shipped axiomere-python wrapper, in a scratch copy of the corpus
(`evidence-003/probe-run/`, outputs in `outputs-postfix/`). Where a
probe only *generates* attack files, verdicts were taken from the
post-fix CLI/bindings on those regenerated outputs
(`outputs-postfix/surface-checks.txt`, `p10p11-surface.txt`).

| Probe | Finding | Post-fix outcome |
|---|---|---|
| p01 genesis recompute | AK-1 (admitted residual) | Attacker-keyed rebuild still verifies in bare mode — the design's stated residual; anchoring + trusted maps are the mitigation. Its transplant step (b) is p01b. (The probe's own `verify_file` tuple unpack predates FR-VP-11's full-dict return and stops the script there; the transplanted file was verified directly — next row.) |
| p01b anchor transplant | **RTB-01 (Critical)** | **FAILS.** `Database.open` refuses: `verification failed at seq 5: checkpoint-root-mismatch`. CLI on `forged-anchor.axmr` and the re-signed `forged-anchor2.axmr`: `ok=false, break_seq=5, reason=checkpoint-root-mismatch`, exit 1. |
| p02 signature strip | RTB-02 | strip-sig / strip-signer fail bare AND trusted (`unsigned-event` @1). strip-both passes bare (no signature material remains) with `unsigned_events=[1..6]` reported — FR-VP-02's recorded design — and fails trusted (`unsigned-event`). |
| p03 unsigned append | RTB-03 | **FAILS.** `verify()` → `ok=False, break_seq=7, reason=unsigned-event`; reopening the poisoned file is refused with the same error. |
| p04 attacker appends | RTB-04 | Bare self-verify passes (by design — identity is trusted mode's job). Trusted map via bindings: atk-as-victim → `signer-key-mismatch` @7 (held defense); **atk-as-mallory → `untrusted-actor` @7, `untrusted_actors=['mallory']`**. |
| p06 snapshot forge | RTB-06 | **FAILS.** `verify()` → `snapshot-id-mismatch` @7; `current()` now refuses with the same error instead of projecting the forged snapshot. |
| p07 dead battery | RTB-12 | **Write path refuses the resurrection**: `record r1 is dead; death is terminal — a live snapshot cannot follow a dead one`. Stored-fault variants are covered by fixtures dead-c (`dead-reason-missing`) / dead-d (`snapshot-id-mismatch`) and the write-path test `write_path_dead_requires_reason`. |
| p08 canonicalization + conversion | RTB-07/08/10 | Honest battery round-trips exactly (NFC≠NFD preserved, `1e-7`/`5e-324`/`-0.0`/`u64_max` exact) and verifies; the lossy append **raises** `ValueError: integer outside the exactly representable range [i64::MIN, u64::MAX]` (pre-fix: silently rounded; NaN→null is likewise rejected — bindings test). Mangled re-serialization fails file verification (`non-canonical-line`). |
| p09 naive spec-only verifier | RTB-10 | The probe's three readings as written still diverge on the honest float file — its float rendering is Python `repr` (`1e-07`), i.e. the probe itself embodies the ambiguity. The ambiguity is what FR-VP-09 removed: DESIGN §4.1 pins raw-UTF-8 escaping, hex-text `prev_hash`, and signed never-zero-padded exponents; the T311 verifier written from the pinned text reproduces every vector and verifies the honest corpus (incl. the regenerated canon.axmr and victim.axmr: `OK`). |
| p10 surgery (held H-1) | — | All regenerated surgery files still fail at the named seq (`seq-gap`/`hash-mismatch`); `surg-truncate` passes bare (by design) and fails under the head pin (bindings test + fixture suite). |
| p11 file-level | RTB-08 | Regenerated variants: blanks/CRLF/extra-field/spaces → `non-canonical-line`; BOM → `bad-magic`; mid-file magic → `unparseable-line`; no-EOL honest file still verifies (the one recorded tolerance). |
| p13 duplicate keys | RTB-09 | dup-payload: `verify_file` returns `ok=False, reason=non-canonical-line` (pre-fix: verified); `Database.open` refuses. dup-event: `unparseable-line` verdict, no exception. |
| p12 concurrency (held H-7) | — | Covered by the `conc.axmr` fixture: still fails at seq 7 (post-fix reason `unsigned-event` — the first seq-7 copy is unsigned in a signed ledger; see the manifest note). |

### Article VI — the suite fails against the pre-fix build

The adopted corpus suite (`tests/redteam_corpus.rs` + fixtures, copied
unchanged) was run against the pristine pre-fix tree (v0.1.0 behavior).
Result: **FAILED with 43 mismatches, every one on a forgery
expectation; no honest file mismatched.** Pre-fix, the suite shows
`forged-anchor2` verifying (RTB-01), strips passing bare and trusted,
`atk-as-mallory` passing trusted, `snap-forge` / `dead-a2` / `dead-c` /
`dead-d` / `unsigned-append` all verifying, and the byte-variant files
opening for appends. Record: `evidence-003/article-vi-prefix.txt`.

### Differential (FR-VP-12)

Core (`verify_file`) vs CLI (`axiomere verify`) vs bindings
(`_axiomere.verify_file`) on all 38 corpus files, bare and trusted:
**identical verdicts (ok + reason) on 38/38 in both modes** — 76
comparisons, zero drift (`evidence-003/differential.txt`).

### Suite state

`cargo test --workspace` green post-fix: floats 8, integration 7,
policy 7, vectors 5, redteam_corpus 1, redteam_corpus_file 3,
from_spec 1. Bindings tests (`crates/axiomere-py/tests/test_bindings.py`,
3 tests) pass against the maturin-built post-fix module.

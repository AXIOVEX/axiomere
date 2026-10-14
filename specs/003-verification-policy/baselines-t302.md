# Spec 003 — T302 baselines (recorded 2026-10-13, before implementation)

Method: the pre-fix tree (main @ `7f01339`, the v0.1.0 behavior) was cloned
pristine and a public-API-only dump tool (`read_events` + `verify_events` /
`verify_events_trusted` + `Ledger::open`) was run over both corpora on copies.
Originals were never modified. These are the outcomes the post-fix runs
(T312/T315) are diffed against.

## Compatibility corpus (AXM-003-13) — pre-fix outcomes

All 17 files: `ok=true`, `Ledger::open` accepts, **zero signed events in every
file** (the presence rule therefore cannot fire on them; their post-fix risk
surface is snapshot validation, checkpoint recomputation, and canonical
bytes — all writer-guaranteed).

| File (copy name) | Source | Events | ok |
|---|---|---|---|
| tekmerion-eval-study.axmr | tekmerion repo `specs/002…/evaluation/study/study.axmr` | 45 | true |
| tekmerion-eval-bundle-v1.axmr | tekmerion repo bundle-v1 ledger | 41 | true |
| tekmerion-eval-bundle-v2.axmr | tekmerion repo bundle-v2 ledger | 45 | true |
| tekmerion-e6-audit.axmr | tekmerion repo Spec 005 E6 demonstration bundle audit ledger | 8 | true |
| phase5-study.axmr | workspace `phase5-demo/study.axmr` | 20 | true |
| phase5-bundle-v1.axmr | workspace phase5-demo bundle-v1 | 11 | true |
| phase5-bundle-v2.axmr | workspace phase5-demo bundle-v2 | 14 | true |
| phase5-bundle-v3.axmr | workspace phase5-demo bundle-v3 | 18 | true |
| phase5-bundle-v4.axmr | workspace phase5-demo bundle-v4 | 20 | true |
| math-study.axmr | workspace `math-study/study.axmr` | 95 | true |
| math-bundle-v1.axmr | workspace math-study bundle-v1 | 93 | true |
| math-bundle-v2.axmr | workspace math-study bundle-v2 | 95 | true |
| zenodo-23311479.axmr | Zenodo 10.5281/zenodo.23311479 (Phase 5 v1) ledger | 11 | true |
| zenodo-23317023.axmr | Zenodo 10.5281/zenodo.23317023 (Phase 5 v3) ledger | 18 | true |
| zenodo-23317259.axmr | Zenodo 10.5281/zenodo.23317259 (Spec 003 math) ledger | 93 | true |
| zenodo-23319126.axmr | Zenodo 10.5281/zenodo.23319126 (Phase 7) ledger | 41 | true |
| osf-84dk7-study.axmr | OSF registration 84dk7 (DOI 10.17605/OSF.IO/84DK7) ledger file | 45 | true |

(Deposit copies were the red team's verified downloads under
`~/workspace/redteam-2026-10-13/rt-e/outputs/deposits/`; several are
byte-identical to their repo/workspace counterparts, as expected.)

## Red-team corpus — pre-fix outcomes

Per-file pre-fix verdicts (bare + trusted where applicable) and the post-fix
expectations are recorded in the adopted fixture manifest:
`crates/axiomere/tests/fixtures/redteam-2026-10-13/manifest.json` (each entry
carries `prefix` = measured pre-fix outcome and `postfix` = the expected
post-fix verdict with its governing claim). Headline pre-fix facts the spec
exists to change: `forged-anchor2` ok=true bare (RTB-01 transplant);
`strip-both/sig/signer` ok=true bare **and** trusted (RTB-02);
`unsigned-append` ok=true (RTB-03); `atk-as-mallory` ok=true trusted
(RTB-04); `snap-forge` ok=true (RTB-06); `dead-a2` (resurrection), `dead-c`
(reasonless death), `dead-d` (forged dead) all ok=true (RTB-12); the
`file-*`/`canon-mangled`/`dup-payload` byte variants all ok=true (RTB-08/09);
`surg-truncate` ok=true bare (RTB-11, stays ok bare by design — detection is
the pin). Held pre-fix and required to stay held: all `surg-*` surgery
(seq-gap/hash-mismatch), `atk-as-victim` trusted (signer-key-mismatch),
`conc` (seq-gap), `dup-event`/`file-bom`/`file-magic2` read errors
(fail-closed; post-fix they become returned verdicts, AXM-003-12).

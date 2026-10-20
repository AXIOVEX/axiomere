# AEE Assessment — Spec 003 Verification Policy (after_implement, T317)

**Date:** 2026-10-20 · **Tooling:** AEE engine 1.0.4 (pinned for Tekmerion/Axiomere work), threshold 0.70
**Claims assessed:** 15 (`aee-claims.json` v0.2.0) · **Machine record:** `aee-assessment.json` (this phase); the after_specify record is preserved unchanged at `aee-assessment-after_specify.json`
**Outcome: ITERATE** — "Assessed 15 claim(s); found 51 failure mode(s); 0 claim(s) below the 0.70 confidence threshold."
**Before (after_specify, 2026-10-13):** outcome ITERATE, the same summary line; every claim a draft, scored on evidence that demonstrated the defect, not yet the fix.

## What changed since the after_specify assessment

The build (T304–T316) is merged, the T315 evidence runs are recorded
in `evidence-t312-t315.md`, and the whole red-team battery was re-run
against the remediated code on 2026-10-20 (`battery-evidence-2026-10-20.md`,
with Tekmerion's `battery-compat-report-2026-10-20.md`). Claims moved
to **supported** on that evidence — implementation evidence, as the
after_specify assessment promised; a score is not a demonstration,
and no status here rests on the score. The full core suite is green
locally at the assessed tree (`cargo test -p axiomere`: 32 passed,
0 failed — the same count the battery recorded).

In plain terms, what a verifier can now rely on: the checkpoint root
in a ledger is recomputed from the events themselves, never taken
from the file's own claim, so a transplanted root fails even when
re-signed by its true author. A ledger that carries signatures must
carry one on every event, and verification against a trusted key map
is strict — an actor the map does not name fails by name. Files are
read strictly: one non-canonical line fails the file. A shortened
copy of a ledger is caught whenever the caller pins the expected
head, and the documentation says a bare check cannot catch it.

| Claim | Score | State |
|-------|-------|-------|
| AXM-003-01 verification was presence-only (characterization of v0.1.0) | 1.00 | supported — the pre-fix fixture runs at T315 demonstrate the characterized behavior |
| AXM-003-02 checkpoint roots recomputed, never read from payloads | 1.00 | supported — transplant fails `checkpoint-root-mismatch`, incl. the fully re-signed variant |
| AXM-003-03 signature presence enforced in signed ledgers | 1.00 | supported — strip/unsigned variants fail in trusted mode, named; bare mode reports the unsigned events |
| AXM-003-04 trusted map strict; untrusted actors fail and are reported | 1.00 | supported — attacker appends fail `untrusted-actor`; signing as the victim fails `signer-key-mismatch` |
| AXM-003-05 trusted mode exposed on CLI + bindings | 1.00 | supported — battery used both surfaces throughout; 40/40 cross-surface differential, zero disagreements |
| AXM-003-06 snapshot payloads validated in verify + fold | 1.00 | supported — forged snapshot fails `snapshot-id-mismatch` |
| AXM-003-07 dead semantics enforced in the core | 1.00 | supported — resurrection refused; death is terminal on every path |
| AXM-003-08 bindings reject non-finite / out-of-range values | 1.00 | supported — append refuses and writes nothing; largest exact integer round-trips |
| AXM-003-09 DECISION: canonical bytes enforced on file read | 1.00 | supported — every respelling variant fails `non-canonical-line` |
| AXM-003-10 encoding pinned normatively, with vectors | 1.00 | supported — a clean-room reading of the document verifies the honest corpus and reproduces its worked hash |
| AXM-003-11 expected-head verification (truncation detectable) | 1.00 | supported — the truncated prefix fails pinned checks (`shortfall` / `root-mismatch`); the intact ledger passes |
| AXM-003-12 verify_file: full result shape, no raise | 1.00 | supported — malformed files return ok=false with a named reason |
| AXM-003-13 backward compatibility over the frozen/deposited corpus | 1.00 | supported — 17/17 corpus ledgers identical after all remediation merges, deposited ledgers included |
| AXM-003-14 red-team corpus adopted as regression fixtures | 1.00 | supported — probes re-executed post-fix (T315) and again in the 2026-10-20 battery |
| AXM-003-15 DECISION: companion obligations live in their own repos | 1.00 | **partially supported — see below** |

## AXM-003-15, stated plainly

The engine scores AXM-003-15 at 1.00, because the decision itself and
the evidence for it are complete. The claim's *standing* is recorded
as **partially supported**, and the distinction matters: this claim
says the companion obligations are executed in their own repos and
that this spec's assessment must not claim them done (its falsifier,
F-VP-15, is exactly that act). The measured state of the companions:

- **axiomere-python companion — executed.** The bindings were rebuilt
  against the fixed core and expose the full result and the trusted
  map / expected-head parameters (that repo's PRs #9 and #10; core
  pinned, signed round-trip smoke test in its release workflow).
- **Tekmerion companion — executed at study boundaries; open at the
  platform layer.** Freeze, export, and bundle verification now pin
  against the recomputed root and trusted verification is required at
  signed study boundaries (Tekmerion PR #40). What remains open is
  the platform-layer half of the seedless-append closure: platform
  ledgers hold no enrolled ledger-signing key, so trusted-mode
  verification there awaits a key-custody design decision that
  belongs to Tekmerion's Spec 005. It is recorded in that package
  (`qualification-note-2026-10-20-spec-013-t40.md`) and in the
  battery report's residuals. A forged platform ledger is refused at
  open today; the stronger positive check is the part still missing.

So: both companions landed in their own repos, one obligation inside
the Tekmerion companion is still open by a named decision elsewhere,
and — per this claim's own rule — this assessment records that
exactly as it stands rather than rounding the claim up to supported.

## Reading

Fourteen claims stand supported on implementation evidence; no claim
is below threshold; the compatibility hard constraint (AXM-003-13)
was re-measured after every remediation merge, not only at build
time, and holds at 17 of 17. The outcome word is ITERATE, as it was
at after_specify — the engine's word for a bundle whose failure modes
are all answered by recorded recoveries, not a verdict that anything
failed. The remaining T317 items are release mechanics: the changelog
entry ships with this record, and the 0.2.0 tag on the exact build
commit is the coordinator's release act, prepared for in the
changelog's [Unreleased] section and not performed here.

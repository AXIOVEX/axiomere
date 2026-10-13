# AEE Assessment — Spec 003 Verification Policy (after_specify)

**Date:** 2026-10-13 · **Tooling:** spec-kit 1.0.10, AEE engine 1.0.4 (pinned for Tekmerion/Axiomere work), threshold 0.70
**Claims assessed:** 15 (`aee-claims.json` v0.1.0, all at status draft) · **Machine record:** `aee-assessment.json`
**Outcome: ITERATE** — the characterization claim is fully evidenced by the red team's probes; the fourteen contract claims carry their demonstrated before-states and await implementation evidence. No claim sits below threshold.

| Claim | Score | State |
|-------|-------|-------|
| AXM-003-01 verification is presence-only (characterization) | 1.00 | draft — demonstrated by RT-B probes across all shipped surfaces |
| AXM-003-02 checkpoint roots recomputed, never read from payloads | 1.00 | draft — before-state is RTB-01 (Critical); demonstration is T305/T315 |
| AXM-003-03 signature presence enforced in signed ledgers | 1.00 | draft — before-states RTB-02/RTB-03; demonstration is T306/T315 |
| AXM-003-04 trusted map strict; untrusted actors fail and are reported | 1.00 | draft — before-state RTB-04; demonstration is T306/T315 |
| AXM-003-05 trusted mode exposed on CLI + bindings | 0.90 | draft — single-source evidence (RTB-05 source audit + harness); demonstration is F-VP-5 |
| AXM-003-06 snapshot payloads validated in verify + fold | 1.00 | draft — before-state RTB-06; demonstration is T307/T315 |
| AXM-003-07 dead semantics enforced in the core | 1.00 | draft — before-state RTB-12 battery; demonstration is T307/T308 |
| AXM-003-08 bindings reject non-finite / out-of-range values | 1.00 | draft — before-state RTB-07; demonstration is T314 (F-VP-8) |
| AXM-003-09 DECISION: canonical bytes enforced on file read | 1.00 | draft — decision recorded with its rejected alternative (spec §6) |
| AXM-003-10 encoding pinned normatively, with vectors | 1.00 | draft — before-state RTB-10; the pinned values were measured from the reference implementation before drafting (plan.md, decisions) |
| AXM-003-11 expected-head verification (truncation detectable) | 0.90 | draft — single-source evidence (RTB-11); demonstration is F-VP-11 |
| AXM-003-12 verify_file: full result shape, no raise | 0.90 | draft — single-source evidence (RTB-13); demonstration is F-VP-12 |
| AXM-003-13 backward compatibility over the frozen/deposited corpus | 1.00 | draft — design basis asserted; the demonstration (baselines T302, corpus run T312) is the build's hard gate |
| AXM-003-14 red-team corpus adopted as regression fixtures | 1.00 | draft — corpus + verdict manifest (T302/T303); Article VI defective-build runs at T315 |
| AXM-003-15 DECISION: companion obligations live in their own repos | 1.00 | draft — decision recorded; this spec's close-out must not claim the companions done |

## Reading

Claims preceded code, per Article VII — the sequence this repo adopted after Spec 001's recorded deviation. Every requirement claim in this bundle answers a named red-team finding whose before-state was demonstrated by a runnable probe, and every falsification test is the corresponding probe re-run (or a fixture built from its corpus file) against the post-fix build: the implementation is falsifiable against this bundle, not retro-fitted to it. All fifteen claims remain at **draft** — the three at 0.90 carry single-source evidence by construction (each is a surface/contract gap the red team proved from one audit), and the rest score on evidence that demonstrates the *defect*, not yet the *fix*. Statuses move only on implementation evidence at the after_implement assessment (T317); a score is not a demonstration.

Two process notes, recorded not hidden. (1) The encoding values pinned in spec §5 were **generated from the reference implementation** during drafting, and the measurement corrected the drafter: positive exponents are signed in the reference rendering (`1e+21`), where the first drafting assumption had them unsigned — an error that, unmeasured, would have made the pinned document wrong in exactly the way RTB-10 documents. (2) This package was authored in a dedicated clone (`~/workspace/remediation-2026-10-13/axiomere-spec`); the shared checkout was not touched, and no implementation code is in this branch — the diff is the spec package only.

# Spec 003 — Red-team battery evidence (Axiomere side)

Date: 2026-10-20. Companion to the Tekmerion Spec 013 battery and
compatibility report (Tekmerion PR #44), which carries the full
program-level account. This file records the Axiomere-side results:
the RT-B ledger-core battery re-run against this repository at
`cc452998c7bdbdd3fae50964733fef5f9bed0d3e`, the cross-surface
differential, and the backward-compatibility corpus measurement.

Suites: `cargo test -p axiomere` — **32 passed, 0 failed** (10 test
binaries); `cargo check -p axiomere-py` clean; the axiomere-python
suite (separate repository, core pinned to cc45299) — **17 passed**.

## Battery results (RT-B)

Every demonstrated RT-B attack was re-run from the red team's own
probes, against a regenerated victim ledger (the victim generator
uses random seeds per run; the dependent probes were re-run against
the regenerated victim so trusted-mode comparisons stay valid). One
probe adaptation: p01's result unpacking was updated to the current
`Verification` fields.

- **RTB-01 (Critical) — checkpoint-root transplant: fails.** The
  transplanted ledger fails `verify_file` at sequence 5 with
  `checkpoint-root-mismatch` (roots are recomputed), `Database.open`
  refuses, and the fully re-signed transplant (red team's resign
  tool rebuilt against this core) is refused at open with the same
  reason.
- **RTB-02 — signature stripping: fails.** All three strip variants
  fail in trusted mode (`unsigned-event`, sequence 1). Bare mode:
  strip-signature and strip-signer fail the same way; strip-both
  passes bare only as an honestly unsigned ledger, with the unsigned
  events (1–6) listed in the result — visible, not silent.
- **RTB-03 — unsigned append: fails.** Verification rejects with
  `unsigned-event` (events 7–8 named).
- **RTB-04 — attacker-signed append: fails.** Trusted mode:
  `untrusted-actor`, the actor named in the result.
- **RTB-05 — trusted mode reachability: closed.** Trusted maps and
  expected-head pins are exposed in the CLI and in the Python
  `verify`/`verify_file`, and were used throughout this battery.
- **RTB-06 — snapshot forgery: fails.** `snapshot-id-mismatch` at
  sequence 7; the current view refuses.
- **RTB-07 — numeric range: fails.** An out-of-range integer crashes
  the append with the exact representable-range error; 2^70, −2^65,
  NaN, and ±infinity are each refused ("non-finite float (NaN or
  infinity) cannot be stored"); the largest exactly representable
  integer round-trips exactly.
- **RTB-08 — canonical bytes: fails.** Blank-line, CRLF,
  extra-field, and space variants all fail `non-canonical-line`; a
  mangled re-serialization fails the same way. (A missing final
  newline still verifies — the canonical form does not require one.)
- **RTB-09 — duplicates: fail.** A duplicated payload field fails
  `non-canonical-line`; a duplicated event fails `unparseable-line`.
- **RTB-10 — format ambiguity: closed by the document.** DESIGN.md
  §4 now pins raw UTF-8 strings, exponents always signed and never
  zero-padded (`1e-7`), and prev_hash as hex text, with a worked
  example. A clean-room implementation of the current text verifies
  both honest corpus ledgers (ASCII and unicode/float), and the
  document's printed example hash
  (`cbaa00f584506ec8d87c8596766651736afb14071cf4e00dc4538c83c4b8bfea`)
  reproduces exactly when the hash input is built the way the text
  specifies (prev_hash prepended to the canonical body). Readings of
  the old text fail the unicode/float ledger, as expected.
- **RTB-11 — truncated prefix: contained by the head pin.** A bare
  truncated prefix still verifies — a prefix is a valid shorter
  ledger, and the API documentation says so and names the pin as
  required. Pinned verification of the same prefix fails
  (`shortfall` against a full pin; `root-mismatch` against a
  root-only pin), and the intact ledger passes the pin.
- **RTB-12 — dead records: fail.** Resurrection is refused in the
  core ("record r1 is dead; death is terminal"); a dead snapshot
  with no reason is refused; a "Dead" status fails verification
  (`invalid-status`); a raw forged dead snapshot appended at write
  time fails verification (`snapshot-id-mismatch`) and the view
  refuses it. Score forcing still stores 0.0.
- **RTB-13 — malformed files: fixed.** `verify_file` returns a full
  result (ok=false, named reason) for malformed files instead of
  raising.

Held defenses, all still holding: the surgery family (delete,
duplicate, edit, replay, splice, swap) is caught with named breaks
(`seq-gap`, `hash-mismatch`); signing as the victim fails trusted
mode (`signer-key-mismatch`); bad magic and unparseable content fail
closed; a missing native extension raises ImportError with build
instructions; the two-writer interleave fails verification and
refuses to open.

Acknowledged items: genesis recomputation (AK-1) remains possible
as self-consistency — the anchored mitigation is RTB-01, measured
working. Erasure (AK-2) remains documented-unimplemented and was
not re-probed. Lossy float parsing (AK-3) remains closed (exact
round-trips).

## Cross-surface differential

The original differential premise (shipped bindings vs a fresh core
build) no longer exists — one core (cc45299) sits under every
surface. The held property was re-measured as agreement among the
three current surfaces over the 40-file probe corpus:
axiomere-python bindings, the bindings bundled in the Tekmerion
environment, and the Rust CLI built from this clone. **40 of 40
files, zero disagreements** — identical verdict, event count, break
position, and reason on every file.

## Backward compatibility

The 17-ledger compat corpus (every frozen and deposited ledger on
record: math study and bundles, the OSF-registered study, Phase 5
study and bundles, the platform audit ledger, the evaluation study
and bundles, and the four Zenodo-deposited ledgers) was re-measured
with the current bindings. **17 of 17 identical** to the recorded
baseline: bare verification ok, the recorded event counts
(93/95/95/45/11/14/18/20/20/8/41/45/45/11/18/93/41), no break, no
reason, and every ledger opens. No migrations, no silent changes.

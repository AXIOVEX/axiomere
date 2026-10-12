# Spec 002 — Float Canonicalization: Correctly-Rounded Parsing

**Date:** 2026-10-12 · **Status:** Specified (claims assessed at after_specify; implementation follows) · **Process:** claims precede code, per Constitution Article VII. This spec exists because of a defect found downstream (Tekmerion E1, 2026-10-12), and because the fix touches the one component whose correctness is the product: verification itself.

## 1. The defect

An Axiomere event whose payload carries certain finite `f64` values **fails hash verification on reopen** — `hash-mismatch` at that event's `seq` — although nothing has tampered with the file. The chain reads as broken when it is not.

Reproduction (2026-10-12, current build, via the Python package):

- Append `{"ts_like": 1791810119.6349843, "plain": 0.5}` → append succeeds; in-session `verify()` already returns false; `Database.open` raises `verification failed at seq 1: hash-mismatch`; `verify_file` agrees. Frozen as fixture `crates/axiomere/tests/fixtures/legacy-floats-defect.axmr`.
- Tekmerion's platform ledger hit the same failure with a high-precision timestamp float (value `1791810119.6349843`) and now restricts payloads to whole-second timestamps as a workaround (documented in Tekmerion `src/tekmerion/platform/ledger.py`). That workaround mitigates one caller; the defect is here.

## 2. Characterization (evidence before design)

The divergence is **reader-side only**. The writer is correct; the reader misreads.

- **Writer (correct).** Appends hash `canonical(body)` where floats are rendered by serde_json's number formatting (Ryū shortest-round-trip): for the reproduction value it emits exactly `1791810119.6349843`, which correctly-rounded parsing (Python) maps back to the appended bits `0x41dab33691e8a395`. The bytes on disk are the correct canonical form of the appended value.
- **Reader (defective).** Verification re-reads each line with serde_json's default float parser, which is *not* correctly rounded: it reads `1791810119.6349843` as bits `0x41dab33691e8a396` — **1 ULP high** — whose canonical rendering is `1791810119.6349845`. The recomputed body differs from the hashed body; verification reports `hash-mismatch`. It is not tampering, and the failure is in the JSON number parser, not in `canonical()` and not in the hash chain.
- **Trigger class.** Any finite float whose canonical decimal needs **≥ 17 significant digits** to pin down (including exponent forms such as `1.0086398161248257e+149`). Measured: 593,459 / 2,000,000 random finite f64 bit patterns (≈ 30 %) fail the format→parse round trip; timestamp-shaped decimals (10 integer digits + fraction) fail at ≈ 5 % with 7 fraction digits (17 significant) and ≈ 12 % beyond; **zero** failures at ≤ 16 significant digits in 200,000 trials per shape. Every observed error is exactly ±1 ULP.
- **Unaffected paths.** Payloads entering via PyO3 carry exact f64 bits (no text), so appends hash the caller's true value; integers, booleans, strings, and short-decimal floats are unaffected. Non-finite values (inf/NaN) never reach the file: both the Rust and Python entry points map them to `null` today; that behavior is unchanged by this spec.

## 3. The hard constraint (governs the design)

**No existing `.axmr` log may change verification outcome.** Every log that verifies today must verify identically after the fix, and the canonical encoding of every event must be byte-identical. Axiomere's records are evidence; a fix that heals new writes by invalidating (or silently re-interpreting) existing history would violate the premise of the ledger — and DN-005's absolute append-only rule, under which the Tekmerion records on Axiomere are kept.

The constraint holds by design for this fix, by the following argument (also claim AXM-002-03's basis): verification recomputes `hash(prev ‖ canonical(body))` from the *parsed* event. The canonical rendering is injective on f64 bit patterns (distinct doubles have distinct shortest representations). So a log that verifies under the old parser had, for every float in it, `parse_old(text) = ` the bits the appender hashed — which are also the *correctly rounded* bits of that text. A correctly-rounded parser therefore returns the identical bits for every float in every currently-verifying log: identical canonical bodies, identical hashes, identical verdicts. The new parser can differ from the old only on texts the old parser misreads — and a log containing such a text fails today *unless* the appender itself ingested the value through the old parser (the CLI path), in which case the hashed bits are the old-parser bits, the stored text is their canonical rendering, and both parsers return those same bits for that rendering. Either way: **outcomes on existing logs are preserved exactly; the only outcome changes anywhere are fail → pass for logs whose sole defect is this one.**

## 4. Requirements

- **FR-FLT-01 (validity preservation).** Every `.axmr` log that verifies before the fix verifies identically after it — same `ok`, same event count, same reason — including the real Tekmerion study ledgers (Phase-5 demo study + bundled copies) and this repo's fixtures. No canonical byte of any event changes.
- **FR-FLT-02 (defect repaired).** An untampered log whose only verification failure is this defect verifies after the fix and is appendable again — including the frozen defect fixture and the reproduction value `1791810119.6349843` through the production append → reopen → verify path, in Rust and via the Python bindings.
- **FR-FLT-03 (format untouched).** The v1 format is unchanged: magic `AXMR1`, one canonical-JSON event per line, same canonical float rendering (shortest round-trip). The fix changes only how decimal text is parsed back to f64 — to IEEE-754 correctly-rounded (round-to-nearest-even), inverting the writer exactly. No new magic version (Constitution Article V is not triggered: the format does not change; its reading is made faithful).
- **FR-FLT-04 (round-trip totality, tested).** For every finite f64, format → parse → format is the identity through the production verify path. Demonstrated by a bounded randomized bit-pattern sweep test plus the characterized shape matrix: ≥ 17-significant-digit decimals, exponent forms, subnormals, ±0.0, integral-valued floats, exact-tie decimals, maximum/minimum finite values — on both the Rust core path and the bindings path.
- **FR-FLT-05 (tamper detection undiminished).** Seeded defects still fail: a digit altered in a float payload fails verification exactly as before, as do the Spec 001 seeded defects (tampered payload, deleted line, forged signature). The fix grants no new acceptance to altered records. (Pre-existing note, out of scope: verification hashes the canonical form, so respelling a number to a different text *of the same value* — `0.50` for `0.5` — was and remains verification-equivalent; this spec neither introduces nor removes that property.)
- **FR-FLT-06 (format contract completed in writing).** `docs/DESIGN.md` §4 gains the number rule the format always implied: canonical float rendering is the shortest decimal that round-trips, and parsing a conforming file **must** be correctly rounded — so an independent implementation written from the document alone verifies what the reference implementation verifies (supports AXM-001-08).

## 5. Design

Enable serde_json's **`float_roundtrip`** feature in `crates/axiomere`
(and, for explicitness, `crates/axiomere-py`, whose build must not depend
on feature unification to be correct). The feature switches serde_json's
float parsing to a correctly-rounded algorithm — the documented purpose
of the feature is exactly round-trip fidelity. Nothing else changes: the
formatter, `canonical()`, the hash construction, the file format, and
every public API are untouched.

**Alternatives considered and rejected:**

- *Change the canonical float encoding* (fixed precision, hex floats,
  floats-as-strings): changes canonical bytes → changes hashes of
  existing events → violates FR-FLT-01 and triggers Article V. Forbidden
  by the constraint, not merely dispreferred.
- *Hand-rolled correctly-rounded parser:* re-implementing correct
  rounding is how a codebase acquires a second, subtler float bug. The
  upstream feature exists for this purpose and is maintained with
  serde_json.
- *Reject ≥ 17-digit floats at append:* would refuse legitimate payloads
  (timestamps with sub-second precision are ordinary data) and would do
  nothing for logs already written.

**Accepted cost:** float parsing becomes slower (serde_json documents
≈ 2× for float-heavy parsing). Verification is a batch audit operation,
not a hot path; correctness of the audit is the product. Noted, accepted.

## 6. Non-goals

- No change to how non-finite floats are handled (they map to `null` at
  the entry points today; changing that is a separate, labeled decision).
- No change to Tekmerion's payload discipline (its whole-second
  timestamps may stay; Axiomere stops requiring them).
- No edits to the axiomere-python repository from this spec — see §7.

## 7. Companion item (axiomere-python — specified here, executed there)

The Python package's extension *is* this repo's `crates/axiomere-py`, so
no API or code change is needed in axiomere-python. Its obligations when
it next builds/releases:

1. Build against an axiomere revision carrying this fix (so
   `float_roundtrip` is enabled in its extension build).
2. Add the regression test to its suite: append
   `{"ts_like": 1791810119.6349843}` → reopen → `verify()` ok — the exact
   downstream reproduction that found this defect.
3. Until then, axiomere-python builds in the wild carry the defect;
   Tekmerion's E1 workaround (whole-second timestamps) remains a
   reasonable caller-side mitigation, now documented as temporary.

## 8. Verification

- Reproduction case (frozen fixtures, generated with the pre-fix build):
  `legacy-floats-safe.axmr` verifies before and after;
  `legacy-floats-defect.axmr` fails before (`hash-mismatch`), verifies
  after — both asserted in integration tests.
- Real records: copies of the Tekmerion Phase-5 demo study ledger and
  its bundled copies (5 files, all verifying pre-fix) verify identically
  post-fix (evidence run recorded in the assessment; originals never
  mutated).
- Randomized sweep: ≥ 1,000,000 finite f64 bit patterns through
  canonical format → parse → compare bits, in the integration suite
  (bounded subset in CI if runtime requires; the full sweep is run and
  recorded pre-merge).
- Seeded-defect suite (Spec 001) unchanged and green; a new seeded
  defect — one digit altered in a ≥ 17-digit float payload — fails
  verification.
- `cargo test` green locally and in GitHub CI on the branch.

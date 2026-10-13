# Spec 003 — Verification Policy: Presence, Roots, and Lineage

**Date:** 2026-10-13 · **Status:** Specified (claims precede code, per Constitution Article VII; claims bundle `aee-claims.json` v0.1.0, all claims at draft) · **Origin:** the owner-ordered red team of the Axiovex Research Engine, 2026-10-13 — Team B (ledger core) demonstrated thirteen probe-verified findings against v0.1.0, one critical and five high. Full evidence: `~/workspace/redteam-2026-10-13/rt-b/FINDINGS.md`, probes and corpus preserved there; synthesis: `~/workspace/your_files/2026-10-13-red-team-axiovex-research-engine.md`. This spec is the remediation contract for the axiomere core. It specifies; it does not implement.

## 1. The defect class

The hash chain is sound. Every naive tamper the red team tried against it — delete, swap, duplicate, splice, edit, replay — is caught at a named seq with a named reason (RT-B held result H-1, re-verified across a 38-file corpus on three surfaces with zero drift, H-6). The breaks are all one layer up, and they share one root cause, stated as claim **AXM-003-01**:

> Verification validates the internal consistency of **what is present**, and never checks **what should be present**.

Concretely, against v0.1.0 as shipped (core `crates/axiomere`, bindings `crates/axiomere-py`, CLI `crates/axiomere/src/bin/axiomere.rs`):

| # | Gap | Red-team finding | Claim |
|---|-----|------------------|-------|
| 1 | Checkpoint roots are read from the checkpoint payload and never recomputed — a forged history carrying the victim's transplanted root passes `verify()` and matches the external anchor | RTB-01 (Critical) | AXM-003-02 |
| 2 | Signatures are checked only when present: stripping `sig`/`signer` from every event passes even trusted-map mode; unsigned events appended to a signed ledger pass every mode and win the current view | RTB-02, RTB-03 (High) | AXM-003-03 |
| 3 | Trusted-map mode falls back to trusting the event's embedded key for any actor absent from the map — an attacker appending as `mallory` passes identity verification | RTB-04 (High) | AXM-003-04 |
| 4 | The trusted-map mode exists only in the Rust core library — no CLI flag, no bindings exposure — so the Python consumer (Tekmerion) cannot perform identity verification at all | RTB-05 (Medium) | AXM-003-05 |
| 5 | Snapshot payloads are never validated on read: an invented `snapshot_id` and parent verify, enter the current view, and are then adopted by the official API as the parent of the next honest snapshot | RTB-06 (High) | AXM-003-06 |
| 6 | Withdrawal semantics are enforced only on the happy path: reason-less death succeeds via `append_snapshot`, a stored dead score is unchecked on read, status `"Dead"` folds as live, and an ordinary snapshot resurrects a dead record | RTB-12 (Medium) | AXM-003-07 |
| 7 | The Python bindings silently corrupt values at write time: NaN/∞ are stored as `null`; integers outside i64 are rounded through f64 (2⁷⁰, and even 2⁶⁴−1, which u64 holds exactly — `py_to_json` never tries u64) | RTB-07 (High) | AXM-003-08 |
| 8 | Verification is over parsed values, not file bytes: re-serialized lines, CRLF, blank lines, padding, and injected unknown fields all verify; a duplicated payload key with a forged first occurrence verifies while a human reading the raw line sees the forged value | RTB-08, RTB-09 (Medium) | AXM-003-09 |
| 9 | The published encoding underdetermines the hash: a verifier written from DESIGN §4 alone rejects honest unicode/float files under every reading tried (exponent form, string escaping, prev_hash representation all unpinned) — Constitution Article V's own standard, unmet | RTB-10 (Medium) | AXM-003-10 |
| 10 | Tail truncation is invisible: a prefix of a valid chain is a valid chain, and no verify surface accepts an expected head, count, or root to pin against | RTB-11 (Medium) | AXM-003-11 |
| 11 | `verify_file` returns a reduced tuple and *raises* on malformed files instead of returning a failed verdict; the CLI conflates parse failure with verification failure | RTB-13 (Low) | AXM-003-12 |

Two red-team items are **not** in this spec's scope, and are named so they are not mistaken for oversights: recompute-from-genesis by a sole-copy operator (AK-1) is the design's admitted residual — layers 1–4 make tampering evident to the operator, anchoring makes it evident to everyone; this spec's job is to make the *anchoring mitigation actually work as consumed* (AXM-003-02 + AXM-003-11 + the AXM-003-15 companions), not to re-litigate the layering. The erasure exception (AK-2) remains specified-not-implemented exactly as DESIGN §6 states; nothing here touches it.

## 2. The hard constraints (govern the design)

**C1 — Backward compatibility (AXM-003-13).** Every ledger frozen or deposited by Tekmerion studies to date verifies under the new policy with the identical outcome (same `ok`, same event count) it has under v0.1.0. The corpus is enumerated in the claim: the `.axmr` ledgers tracked in AXIOVEX/tekmerion, the workspace-held frozen study ledgers (Phase 5 demo + bundles; Spec 003 math study), and the deposited bundles (Zenodo 10.5281/zenodo.23311479, 23317023, 23317259, 23319126; OSF 84dk7). This holds by design for the same reason Spec 002's constraint held: **every check this spec adds verifies a property the reference writer already guarantees.** The writer emits canonical bytes; `checkpoint()` stores the true Merkle root of the events it closes; `append_snapshot` computes real content addresses and forces dead scores to 0.0; Tekmerion appends signed, through the API. The new verifier holds files to the writer's own standard — honest files cannot fail it. Pre-fix baselines are recorded before implementation (tasks T302), the post-fix corpus run is evidence (T312), and any exception is named in the assessment with its migration: no silent exemptions.

**C2 — The format does not change (Constitution Article V not triggered).** Magic `AXMR1` stands; the canonical encoding's *output* is byte-identical (Spec 002's constraint carries over: no f64's rendering changes); hash definitions are unchanged. What changes is the reader: it now enforces the encoding the writer always produced, recomputes the commitments the writer always made, and validates the payload semantics the format always defined. DESIGN §4 is *expanded* to state the encoding completely (AXM-003-10) — completing the contract in writing, as Spec 002's FR-FLT-06 did for the number rule — not amended to a new one.

**C3 — A broken log still cannot be opened for appends.** `Ledger::open`'s refusal (DESIGN §3) is preserved and, via the stricter verification, strengthened. The raising surfaces (`read_events`, `Ledger::open`) keep raising; only the standalone verify surfaces gain the no-raise contract (AXM-003-12).

## 3. Requirements

- **FR-VP-01 (checkpoint recomputation).** Verification recomputes every checkpoint's Merkle root over the hashes of the events with `seq <= upto_seq` and fails on mismatch with the payload's claimed root — named reason, checkpoint's seq. (AXM-003-02; RTB-01.)
- **FR-VP-02 (signature presence).** In a ledger carrying ≥ 1 signed event, every event must carry `signer` + `sig`; failure names the unsigned seqs. The `Verification` result gains `unsigned_events` (all seqs lacking either field) reported in every mode, so an unsigned ledger is a visible state (`signatures_checked = 0`, `unsigned_events = all`), never a silent one. (AXM-003-03; RTB-02/03.)
- **FR-VP-03 (trusted map strictness).** With a trusted map supplied, an event whose actor is absent from the map fails verification (`untrusted-actor`, naming actor + seq); the result gains `untrusted_actors`. No fallback to embedded keys, anywhere, in trusted mode. (AXM-003-04; RTB-04.)
- **FR-VP-04 (trusted mode on all surfaces).** CLI: `verify` accepts a trusted map (`--trusted actor=pubkey_hex`, repeatable, and/or `--trusted-file` JSON). Bindings: `verify(trusted={...})` on the ledger object and `verify_file(path, trusted=...)`, returning the full result shape. `None` (no map) and `{}` (empty map — trusts nobody) are distinct inputs with distinct meanings, documented as such. (AXM-003-05; RTB-05.)
- **FR-VP-05 (snapshot validation).** Verification and the fold validate every `record.snapshot` payload: `snapshot_id` recomputed from `{content, parent}` per the pinned construction; `parent` is null (first snapshot of the record) or the `snapshot_id` of an earlier snapshot of the same record; required fields present and typed. Failure names the seq and the lineage fault. Because the fold validates, `current_view` never projects a forged snapshot and `append_snapshot` never adopts one as parent (a ledger containing one fails `Ledger::open` first). (AXM-003-06; RTB-06.)
- **FR-VP-06 (dead semantics in the core).** (a) Every core write path that can produce a dead snapshot requires a non-empty reason (`append_snapshot` included — today only `mark_dead` and the Python helper do); verification fails a stored dead snapshot whose reason is missing/empty. (b) Verification fails a stored dead snapshot whose score is not exactly 0.0. (c) `status` is validated against the vocabulary {live, dead} on verify and on fold — anything else (including case variants) fails verification instead of folding as live; write paths already reject other values and keep doing so. (d) Death is terminal: `append_snapshot` refuses a live snapshot for a record whose current state is dead, and verification fails a log containing a live snapshot after a dead one for the same record. (AXM-003-07; RTB-12.)
- **FR-VP-07 (bindings value fidelity).** `py_to_json` conversion order becomes i64 → u64 → finite-f64; non-finite floats and integers outside [i64::MIN, u64::MAX] raise `ValueError` and append nothing (the ledger file is byte-identical after a rejected call). This is the labeled decision Spec 002 §6 deferred: non-finite values no longer map to `null` at the bindings boundary. (AXM-003-08; RTB-07.)
- **FR-VP-08 (canonical bytes on file read).** The file-reading path used for verification compares each event line byte-for-byte with `canonical(event)`; any line that differs — respelled numbers, reordered keys, `\u` escapes the writer would not emit, duplicate keys at any level, unknown event fields, whitespace inside the line, CRLF, blank lines inside the event region — fails verification with the line number and reason `non-canonical-line`. The magic line discipline is unchanged (BOM and mid-file magic already fail closed; they keep doing so, now reported per AXM-003-12). (AXM-003-09 — decision; RTB-08/09.)
- **FR-VP-09 (encoding pinned, with vectors).** DESIGN §4 gains the complete encoding rules of §5 below, and frozen test vectors (generated from the reference implementation, force-added as fixtures) pin: canonical bodies incl. unicode/control/float payloads, event hashes, Merkle roots for 0/1/2/3-element sets, snapshot ids with and without parent. A verifier written from the document + vectors alone must verify the honest corpus. (AXM-003-10; RTB-10.)
- **FR-VP-10 (expected head).** All verify surfaces accept an optional expected head: `{head_hash, min_events, checkpoint_root}` in any combination. Failure reasons are named (`head-mismatch`, `shortfall`, `root-mismatch` — the last compared against the **recomputed** root of the latest checkpoint). Bare verification is unchanged; the docs state plainly that a truncated prefix passes bare verification and that the pin (or an anchored root checked under FR-VP-01) is the detection. (AXM-003-11; RTB-11.)
- **FR-VP-11 (verify surfaces report, never conflate).** `verify_file` returns the full `Verification` shape on every surface; malformed files return `ok=false` with `break_seq`/line and reason instead of raising. CLI `verify` prints the full result and distinguishes exit/output for unreadable-file vs failed-verification. (AXM-003-12; RTB-13.)
- **FR-VP-12 (corpus + regression adoption).** The Tekmerion compatibility corpus (C1) and the 38-file red-team corpus are adopted as fixtures with recorded expected verdicts (manifest: per-file pre-fix verdict, post-fix verdict, governing claim). The differential property — core, CLI, and bindings return identical verdicts on every fixture — is asserted in-suite. (AXM-003-13, AXM-003-14; Constitution Article VI.)

## 4. Design

**One policy object, every surface.** Verification gains a single options/policy value threaded through core → bindings → CLI (working shape; names may settle in implementation without changing the contract):

```
VerifyPolicy {
    trusted: Option<Map<actor, pubkey_hex>>,   // None = self-consistency; Some = strict (FR-VP-03)
    expected_head: Option<ExpectedHead>,        // FR-VP-10
}
ExpectedHead { head_hash: Option<String>, min_events: Option<usize>, checkpoint_root: Option<String> }
```

The structural checks (FR-VP-01, 02, 05, 06, 08) are **not** policy switches — they hold in every verification, because they restate the writer's own guarantees (C1). What remains a caller choice is exactly what was always a caller choice: whose keys to trust, and what head to pin. `verify_events` keeps its signature as the default-policy entry point; `Ledger::verify`, bindings `verify`, and CLI `verify` accept the policy. The `Verification` result gains `unsigned_events: Vec<u64>` and `untrusted_actors: Vec<String>`; existing fields are unchanged in meaning (`signatures_checked` counts events carrying both fields; a half-stripped event is unsigned, not checked).

**Checkpoint recomputation.** During the verify pass, maintain the running list of event hashes (already computed for the chain check). At a `checkpoint` event, parse `merkle_root`/`upto_seq` from the payload (malformed ⇒ fail at that seq), recompute `merkle_root` over hashes of events `1..=upto_seq` — the events preceding the checkpoint, exactly the set `Ledger::checkpoint` committed — and compare. This reuses the existing `merkle_root`, so writer and verifier share one construction by code, and AXM-003-10's vectors pin it for outsiders.

**Strict file reading.** `read_events` gains a strict sibling used by verification paths: read raw lines, require line 1 == `AXMR1`, then per line: parse the event, recompute `canonical(event)`, require byte equality with the raw line. Byte equality subsumes the duplicate-key and unknown-field cases: the writer's canonical form contains each key exactly once and only the format's fields, so any line carrying more (or spelled differently) differs from its own canonical form and fails — no separate key-scanning pass is needed, and the reason can name the first differing line. Parsing stays serde-based; canonical comparison is over the parsed event exactly as `Ledger::append` produced it (`canonical(&to_value(event))`), so the check is *line == what the writer would have written*, by construction. `Ledger::open` uses the strict reader + full verification, preserving C3.

**Snapshot/death validation in the fold.** One validating fold used by both `verify` and `current_view`: iterate events in seq order, maintain per-record lineage (latest snapshot_id, dead flag). A `record.snapshot` event is validated as it is folded (FR-VP-05/06); the first fault stops verification with seq + reason. `current_view` surfaces the same validation as an error rather than projecting an invalid lineage — a view of a forged ledger is refused, not laundered.

**Bindings.** `py_to_json` per FR-VP-07 (extraction order i64 → u64 → f64-with-finiteness-check; final fallthrough is an error, not `Value::Null`). `PyLedger::verify` and `verify_file` accept `trusted` and `expected_head` keyword arguments and return the full result dict (adding `unsigned_events`, `untrusted_actors`); `verify_file` catches read/parse failures into the result (FR-VP-11) — while `PyLedger::open` keeps raising, per C3.

**CLI.** `verify` grows `--trusted`, `--trusted-file`, `--expect-head`, `--expect-events`, `--expect-root` flags and prints the full result (including `break_seq` and `reason` when failed, `unsigned_events`/`untrusted_actors` when non-empty). Exit codes: 0 = verified; 1 = verification failed; 2 = file unreadable — parse and verification failures are never again the same output (FR-VP-11).

## 5. The pinned encoding (FR-VP-09 — normative text for DESIGN §4)

Measured from the reference implementation at `836d88c` (2026-10-13); the frozen vectors (T310) are these values as fixtures.

- **Objects:** keys sorted lexicographically (byte order), recursively; compact separators — no whitespace anywhere.
- **Strings:** emitted as raw UTF-8. Escape only `"` (as `\"`), `\` (as `\\`), and C0 controls (U+0000–U+001F): short escapes `\b \t \n \f \r` where JSON defines them, otherwise `\u00XX` with lowercase hex. Nothing else is escaped — non-ASCII (é, ☕, €) appears literally. Example: the string `café ☕ "q" \ <U+0001><U+001F><LF><TAB> €` encodes as `"café ☕ \"q\" \\ \u0001\u001f\n\t €"`.
- **Numbers:** integers exact, plain decimal. Non-integers: the shortest decimal that round-trips (Spec 002), in two forms — plain (`0.30000000000000004`, `123456789.12345679`, `1.0`, `-0.0`) or exponent form where the reference chooses it, with the exponent **always signed and never zero-padded**: `1e-7`, `5e-324`, `1.0086398161248257e+149`, `1e+21`. Parsing must be correctly rounded (round-to-nearest-even, Spec 002).
- **Event hash:** `SHA-256(prev_hash ‖ canonical(body))` where `prev_hash` enters as its **64-character lowercase hex text** (not raw bytes), the body is the canonical encoding of `{seq, type, ts, actor, payload, prev_hash}`, and the genesis `prev_hash` is 64 zeros. Example body (payload `{"text":"café ☕","x":1e-7}`): `{"actor":"vect","payload":{"text":"café ☕","x":1e-7},"prev_hash":"0000000000000000000000000000000000000000000000000000000000000000","seq":1,"ts":1792000000,"type":"note"}`.
- **Merkle root:** leaves are event-hash hex strings; a parent is `SHA-256(left_hex ‖ right_hex)` over the hex **text**; an odd element is paired with itself; a single element is its own root; the empty set's root is `SHA-256("")` = `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Vectors: root(`aa…aa`) = `aa…aa`; root(`aa…aa`,`bb…bb`) = `fa0dafbf43f1f551e536353e9d1a942a8e86e41a0b58dfeaf264ef217f6b862a`; root(`aa…aa`,`bb…bb`,`cc…cc`) = `f372961e0178fea099eb05057b8b6a363a21f7ee2456e6e17a8f92990d01d1f9`.
- **Snapshot id:** `SHA-256(canonical({"content": <content>, "parent": <parent-or-null>}))`. Vectors: content `{"k":"v"}`, parent `"dddd…"` (64 d's) → `1a270a907febde697935f2d5529521005d4a731b4455a4e2a04408ea2afb5520`; parent null → `ed346a259f7cc6f798ea9bc7727c38800cc5b81164691c37df6d4ec12bfda3ce`.

## 6. Alternatives considered and rejected

- **Docs-only correction for RTB-08/09** (keep value-level verification; strike "byte-for-byte" from the docs): rejected, and the rejection is claim AXM-003-09's substance. The failure is not a docs overstatement; it is that the file a human auditor inspects can carry content (a forged first key occurrence, an injected field) that the verifier demonstrably did not commit to. The format's reason to exist (DECISION-2026-10-13: the file *is* the canonical form; inspectability by strangers with generic tools) requires the verified object and the inspected object to be the same bytes. Value-level verification makes them different objects.
- **Reject duplicate keys only, keep value-level otherwise:** leaves respelled/reordered files verifying — the RTB-08 residue — and still lets unknown fields ride inside "verified" evidence. Partial measures leave the two findings' shared root (parsed ≠ written) intact.
- **Signatures required in all ledgers by default:** overreach — honestly unsigned ledgers are a legitimate v0.1 state (Article III: appends *may* be signed). The downgrade attack needs a signed ledger to strip; FR-VP-02 conditions presence on the ledger carrying signatures, and FR-VP-03's trusted mode is where identity requirements belong.
- **Signature inside the hashed body:** would stop stripping from being hash-invisible — by changing every existing event's hash. Forbidden by C1/C2, and unnecessary: presence is cheaper to check than the body is to change.
- **Live-after-dead as a warning, not a failure:** rejected — a warning every consumer may ignore recreates RTB-12's silent resurrection one level up. Death terminal (FR-VP-06d) with honest sequences protected by C1's named-exception rule is the falsifiable choice.
- **A new magic version (`AXMR2`) for strict reading:** rejected — the files do not change (C2); a new magic would fragment the corpus and imply the old files are a different format rather than the same format read carelessly.

**Accepted costs:** file verification now also serializes every event (canonical comparison) — verification is a batch audit operation (the Spec 002 precedent accepted ~2× float-parse cost on the same reasoning); strict reading refuses cosmetically damaged files that v0.1.0 accepted (a CRLF-mangled copy now fails loudly with its line named instead of passing silently — that is the RTB-08 fix working, and repair is a deliberate human act outside verification); bindings callers passing NaN statistics get an error where they silently got `null` — intended, and announced in release notes.

## 7. Non-goals

- No format change, no new magic, no hash-definition change (C2).
- No anchor transport (DESIGN §3 layer 5's T111 residual): the crate still produces roots and receipt data; transport remains the embedding system's.
- No erasure/encryption work (DESIGN §6 unchanged).
- No normalization/repair tool for non-canonical files (deliberately; see accepted costs).
- No changes to Tekmerion or axiomere-python from this repo — §8.
- Append-path performance and query/index features (DESIGN §7 non-goals stand).

## 8. Companion items (specified here, executed in their own repos)

Per AXM-003-15, mirroring Spec 002 §7:

1. **axiomere-python:** rebuild against the fixed core; expose `trusted` / `expected_head` idiomatically on `Database.verify` / `verify_file`; update `docs/verification.md` (full result shape, presence policy, strict trusted mode, expected-head pinning) and `docs/format.md` (the §5 pinned encoding, verbatim where shared); regression tests: F-VP-5 from Python, the F-VP-8 rejection battery, `verify_file` no-raise corpus.
2. **Tekmerion:** in freeze/export/anchor flows, compare the **recomputed** checkpoint root (from this spec's verification result), never the payload's claim (`freeze.py` and `export.py` read payload roots today); at study boundaries, require signed mode and supply the trusted map of study actors — the consumer-side act that closes the seedless-open exposure Team C demonstrated at the platform layer. Until these land, this spec's assessment and release notes must say the consumption-side exposure remains open (Constitution Article IV).

## 9. Verification of this spec

- **Baselines before code (T302):** v0.1.0 outcomes recorded for both corpora (compatibility corpus per file: `ok`, event count; red-team corpus per file: verdict + reason), the red-team expected-verdict manifest drafted from `rt-b/outputs`.
- **Probes re-run (T311):** every red-team Team B probe re-executed against the post-fix build; each falsification test F-VP-1…F-VP-12 is a probe or fixture test, not a reading.
- **Article VI defective-build runs:** the adopted fixture suite is run against the pre-fix tree and each forgery test shown to fail there before it counts.
- **Differential:** core vs CLI vs bindings verdicts asserted identical on all fixtures (the H-6 property, preserved by construction and by test).
- **Release discipline:** the fix ships as the next minor release (0.2.0 — behavior change, format unchanged) with CHANGELOG entry and tag on the exact build commit, per the owner's standing semver rule; the release notes carry the two announced behavior changes (strict file reading; bindings non-finite rejection).
- `cargo test` green locally and in GitHub CI on the branch, by read-back.

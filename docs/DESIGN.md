# Axiomere — Design

**Status:** v0.1 design, 2026-10-11 · **License:** MIT · **Provenance:** implements, in Rust, the absolute-append-only decision recorded as DN-005 in the Tekmerion framework repository (design there: Constitution v1.1, Articles IV + XIII).

## 1. Purpose

A record store designed with **truthfulness, provenance, and change management** as its primary requirements — in that order, ahead of query speed. Conventional databases optimize for current state; Axiomere optimizes for *defensible history*: what was asserted, by whom, when, what it replaced, whether it still stands, and proof that none of it was quietly altered.

## 2. The one rule

**Nothing is ever deleted, and nothing is ever updated in place.** There is no delete or update operation in the API — by construction, not by policy. All change is append:

- A **record** is a lineage of **snapshots**.
- A **snapshot** is content-addressed (named by the hash of its contents): `snapshot_id = SHA-256(content + parent_snapshot_id)`. The first snapshot's parent is null.
- **Revision** = a new snapshot. The old one is *superseded* and stays readable.
- **Withdrawal** = a new snapshot with `status: "dead"`, a mandatory `reason`, and `score: 0.0`. Status serves correct consumers; 0.0 protects score-only consumers from continuing to use a dead record.
- **Score** (epistemic weight) and **status** (lifecycle) are separate fields and are never conflated.
- The **current view** is a fold — the events applied in order to produce it (latest snapshot per record, dead excluded by default). It is a projection — producing it never alters the log.

## 3. Integrity in five layers

1. **Hash chain.** Every event carries `seq`, `prev_hash`, and `hash = SHA-256(prev_hash ‖ canonical(event body))`. Alteration, insertion, deletion, or reordering breaks verification at the first affected `seq`, with a named reason.
2. **Content addressing.** Snapshot lineage is hash-linked (layer 2 makes a snapshot's ancestry tamper-evident independently of the event chain).
3. **Signatures.** Appends may be signed with the actor's Ed25519 key; the event carries the signer public key and signature over the event hash. Verification checks signatures against embedded keys (self-consistency) or a caller-supplied trusted-key map (identity). Two rules keep this layer honest (Spec 003): in a ledger that carries any signature, **every** event must be signed — an unsigned event in a signed ledger fails verification and is named; and trusted-map mode is **strict** — an actor absent from the map fails verification (`untrusted-actor`), with no fallback to the key embedded in the event. A ledger with no signatures at all still verifies in bare mode, but the result reports every event in `unsigned_events`, so the unsigned state is visible rather than silent.
4. **Merkle checkpoints.** A `checkpoint` event closes a range with the Merkle root over its event hashes — a compact commitment to a whole history. Verification **recomputes** the root over the hashes of the events the checkpoint closes and compares it with the payload's claim; the claimed root is never taken on trust (a mismatch fails at the checkpoint's `seq`, reason `checkpoint-root-mismatch`).
5. **External anchoring.** Checkpoint roots are meant to be deposited where the operator does not control them (a DOI-minting repository, a timestamp authority under the RFC 3161 timestamping standard, a transparency log). Layers 1–4 make tampering evident to the operator; layer 5 makes it evident to *anyone*, because a sole-copy operator could otherwise recompute a chain from the very first event (from genesis). The crate produces roots and receipt data; anchor transport belongs to the embedding system, and a deployment that skips anchoring must say so.

A log that fails verification **cannot be opened for appends** (`Ledger::open` refuses): a broken record is evidence, not a writing surface.

## 4. File format v1 (`.axmr`)

```
AXMR1\n
{"actor":…,"hash":…,"payload":…,"prev_hash":…,"seq":1,"sig":…,"signer":…,"ts":…,"type":…}\n
…
```

- Line 1: magic `AXMR1`. Verification rejects anything else.
- One event per line, **canonical JSON** (object keys sorted recursively, compact separators) — hashing is defined over this encoding, so independent implementations can verify each other's files byte-for-byte.
- **Numbers.** Integers are exact. A non-integer number is an IEEE-754 double rendered in its **shortest round-trip decimal form** (the shortest decimal string that parses back to the same double). Parsing a conforming file **must be correctly rounded** (round-to-nearest-even): a parser that approximates — landing even 1 ULP from the written value — recomputes a different canonical body and reports a sound file as broken. This requirement is part of the format contract; it was learned from a real defect (Spec 002, 2026-10-12), in which the reference implementation's own JSON parser misread roughly a third of high-precision float payloads on verification.
- Appends are fsync'd. The format is deliberately inspectable with generic tools; a future binary framing must preserve the same event model and hash definitions.

### 4.1 The pinned encoding (normative)

The rules below state the canonical encoding completely — an independent verifier written from this section alone must reach the same bytes and the same hashes as the reference implementation. (Spec 003, FR-VP-09; the values are measured from the reference implementation, and frozen test vectors pinning each rule live in `crates/axiomere/tests/fixtures/vectors-003.json`.)

- **Objects:** keys sorted lexicographically (byte order), recursively; compact separators — no whitespace anywhere.
- **Strings:** emitted as raw UTF-8. Escape only `"` (as `\"`), `\` (as `\\`), and C0 controls (U+0000–U+001F): the short escapes `\b \t \n \f \r` where JSON defines them, otherwise `\u00XX` with **lowercase** hex. Nothing else is escaped — non-ASCII (é, ☕, €) appears literally. Example: the string `café ☕ "q" \ <U+0001><U+001F><LF><TAB> €` encodes as `"café ☕ \"q\" \\ \u0001\u001f\n\t €"`.
- **Numbers:** integers exact, plain decimal. Non-integers: the shortest decimal that round-trips, in two forms — plain (`0.30000000000000004`, `123456789.12345679`, `1.0`, `-0.0`) or exponent form where the reference chooses it, with the exponent **always signed and never zero-padded**: `1e-7`, `5e-324`, `1.0086398161248257e+149`, `1e+21`. Parsing must be correctly rounded (round-to-nearest-even).
- **Event hash:** `SHA-256(prev_hash ‖ canonical(body))`, where `prev_hash` enters as its **64-character lowercase hex text** (not raw bytes), the body is the canonical encoding of `{seq, type, ts, actor, payload, prev_hash}`, and the genesis `prev_hash` is 64 zeros. Example body (payload `{"text":"café ☕","x":1e-7}`): `{"actor":"vect","payload":{"text":"café ☕","x":1e-7},"prev_hash":"0000000000000000000000000000000000000000000000000000000000000000","seq":1,"ts":1792000000,"type":"note"}` hashes to `cbaa00f584506ec8d87c8596766651736afb14071cf4e00dc4538c83c4b8bfea`.
- **Merkle root:** leaves are event-hash hex strings; a parent is `SHA-256(left_hex ‖ right_hex)` over the hex **text**; an odd element is paired with itself; a single element is its own root; the empty set's root is `SHA-256("")` = `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Vectors: root(`aa…aa`) = `aa…aa`; root(`aa…aa`, `bb…bb`) = `fa0dafbf43f1f551e536353e9d1a942a8e86e41a0b58dfeaf264ef217f6b862a`; root(`aa…aa`, `bb…bb`, `cc…cc`) = `f372961e0178fea099eb05057b8b6a363a21f7ee2456e6e17a8f92990d01d1f9`.
- **Snapshot id:** `SHA-256(canonical({"content": <content>, "parent": <parent-or-null>}))`. Vectors: content `{"k":"v"}` with parent `"dddd…"` (64 d's) → `1a270a907febde697935f2d5529521005d4a731b4455a4e2a04408ea2afb5520`; with parent null → `ed346a259f7cc6f798ea9bc7727c38800cc5b81164691c37df6d4ec12bfda3ce`.

### 4.2 Verification policy

Verification checks the **file bytes**, not just the parsed values (Spec 003, FR-VP-08): every event line must equal `canonical(event)` byte-for-byte. A line that differs — respelled numbers, reordered keys, escapes the writer would not emit, duplicate keys at any level, unknown event fields, whitespace inside the line, CRLF, blank lines inside the event region — fails with reason `non-canonical-line` at the first differing line. (One tolerance: a missing final newline — the last line still equals its canonical form.)

On top of the chain check (§3 layer 1), every verification:

- **recomputes checkpoint roots** (layer 4 above) — never reads them from payloads;
- **validates snapshot lineage**: each `record.snapshot` payload's `snapshot_id` is recomputed from `{content, parent}`; `parent` must be null for a record's first snapshot, otherwise one of that record's earlier snapshot ids;
- **enforces the withdrawal rules on read**: a dead snapshot must carry a non-empty `reason` and a `score` of exactly 0.0; `status` must be `live` or `dead`; and death is terminal — a live snapshot after a dead one for the same record fails verification (and the write path refuses to append one);
- **enforces signature presence** in ledgers that carry signatures, and **strict identity** when a trusted map is supplied (layer 3 above).

Two caller choices — and only these two — are policy:

- **Trusted map** (`trusted`): `None` means self-consistency (signatures, where present, are checked against the keys embedded in the events). A map — including an **empty** map, which trusts nobody — means strict identity: every actor must appear in the map, every event must be signed, and each signature must match the map's key for its actor. `None` and `{}` are different inputs with different meanings.
- **Expected head** (`expected_head`): an optional pin — `head_hash`, `min_events`, `checkpoint_root` (compared against the **recomputed** root of the latest checkpoint), in any combination. Failures are named (`head-mismatch`, `shortfall`, `root-mismatch`). Plainly stated: **a truncated prefix of a valid chain is itself a valid chain** — bare verification cannot see truncation; the pin (or an externally anchored root checked under the recomputation rule) is the detection.

The verification **result** reports: `ok`, `events` (count), `break_seq` and `reason` (first fault, when failed), `signatures_checked`, `signatures_failed`, `unsigned_events` (every seq lacking `signer` or `sig`, reported in every mode), and `untrusted_actors` (actors absent from a supplied trusted map). The standalone file-verification surfaces (`verify_file` in the core and the bindings, `axiomere verify` in the CLI) return this result for every file — a malformed or unreadable file is `ok=false` with a named reason, never an exception. `Ledger::open` keeps the opposite contract: it raises, and refuses appends, on any file that does not verify (§3).

## 5. Signing keys

Keys are Ed25519. `generate_keypair()` returns (seed_hex, public_key_hex); seeds are held by actors, never written into the ledger. The ledger stores public keys (embedded per event) and signatures only. Key rotation is an application-level act: new appends use the new key, old signatures verify against the keys embedded in their events — history is not re-signed, ever.

## 6. The erasure exception (specified, narrow)

Payloads are plaintext in v0.1. The design reserves exactly one exception to append-only, for documented legal compulsion: per-record payload encryption with an owner-signed **erasure certificate** appended to the chain and destruction of that record's key (the certificate, hashes, and signatures remain; the chain still verifies). It is not implemented in v0.1, has no API, and no deployment may imply it exists until it does. Ingest discipline is the primary defense: secrets and personal data should be referenced by hash, not embedded as payloads.

## 7. Non-goals (v0.1)

No query language, no secondary indexes, no networking/server mode, no multi-writer concurrency (a ledger has one sequencing writer; concurrent appends are serialized by the owning process), no compression. These are later layers built *around* the append-only core, never inside it.

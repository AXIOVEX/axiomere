# Axiomere — Design

**Status:** v0.1 design, 2026-10-11 · **License:** MIT · **Provenance:** implements, in Rust, the absolute-append-only decision recorded as DN-005 in the Tekmerion framework repository (design there: Constitution v1.1, Articles IV + XIII).

## 1. Purpose

A record store designed with **truthfulness, provenance, and change management** as its primary requirements — in that order, ahead of query speed. Conventional databases optimize for current state; Axiomere optimizes for *defensible history*: what was asserted, by whom, when, what it replaced, whether it still stands, and proof that none of it was quietly altered.

## 2. The one rule

**Nothing is ever deleted, and nothing is ever updated in place.** There is no delete or update operation in the API — by construction, not by policy. All change is append:

- A **record** is a lineage of **snapshots**.
- A **snapshot** is content-addressed: `snapshot_id = SHA-256(content + parent_snapshot_id)`. The first snapshot's parent is null.
- **Revision** = a new snapshot. The old one is *superseded* and stays readable.
- **Withdrawal** = a new snapshot with `status: "dead"`, a mandatory `reason`, and `score: 0.0`. Status serves correct consumers; 0.0 protects score-only consumers from continuing to use a dead record.
- **Score** (epistemic weight) and **status** (lifecycle) are separate fields and are never conflated.
- The **current view** is a fold (latest snapshot per record, dead excluded by default). It is a projection — producing it never alters the log.

## 3. Integrity in five layers

1. **Hash chain.** Every event carries `seq`, `prev_hash`, and `hash = SHA-256(prev_hash ‖ canonical(event body))`. Alteration, insertion, deletion, or reordering breaks verification at the first affected `seq`, with a named reason.
2. **Content addressing.** Snapshot lineage is hash-linked (layer 2 makes a snapshot's ancestry tamper-evident independently of the event chain).
3. **Signatures.** Appends may be signed with the actor's Ed25519 key; the event carries the signer public key and signature over the event hash. Verification checks signatures against embedded keys (self-consistency) or a caller-supplied trusted-key map (identity).
4. **Merkle checkpoints.** A `checkpoint` event closes a range with the Merkle root over its event hashes — a compact commitment to a whole history.
5. **External anchoring.** Checkpoint roots are meant to be deposited where the operator does not control them (a DOI-minting repository, an RFC 3161 timestamp authority, a transparency log). Layers 1–4 make tampering evident to the operator; layer 5 makes it evident to *anyone*, because a sole-copy operator could otherwise recompute a chain from genesis. The crate produces roots and receipt data; anchor transport belongs to the embedding system, and a deployment that skips anchoring must say so.

A log that fails verification **cannot be opened for appends** (`Ledger::open` refuses): a broken record is evidence, not a writing surface.

## 4. File format v1 (`.axmr`)

```
AXMR1\n
{"actor":…,"hash":…,"payload":…,"prev_hash":…,"seq":1,"sig":…,"signer":…,"ts":…,"type":…}\n
…
```

- Line 1: magic `AXMR1`. Verification rejects anything else.
- One event per line, **canonical JSON** (object keys sorted recursively, compact separators) — hashing is defined over this encoding, so independent implementations can verify each other's files byte-for-byte.
- Appends are fsync'd. The format is deliberately inspectable with generic tools; a future binary framing must preserve the same event model and hash definitions.

## 5. Signing keys

Keys are Ed25519. `generate_keypair()` returns (seed_hex, public_key_hex); seeds are held by actors, never written into the ledger. The ledger stores public keys (embedded per event) and signatures only. Key rotation is an application-level act: new appends use the new key, old signatures verify against the keys embedded in their events — history is not re-signed, ever.

## 6. The erasure exception (specified, narrow)

Payloads are plaintext in v0.1. The design reserves exactly one exception to append-only, for documented legal compulsion: per-record payload encryption with an owner-signed **erasure certificate** appended to the chain and destruction of that record's key (the certificate, hashes, and signatures remain; the chain still verifies). It is not implemented in v0.1, has no API, and no deployment may imply it exists until it does. Ingest discipline is the primary defense: secrets and personal data should be referenced by hash, not embedded as payloads.

## 7. Non-goals (v0.1)

No query language, no secondary indexes, no networking/server mode, no multi-writer concurrency (a ledger has one sequencing writer; concurrent appends are serialized by the owning process), no compression. These are later layers built *around* the append-only core, never inside it.

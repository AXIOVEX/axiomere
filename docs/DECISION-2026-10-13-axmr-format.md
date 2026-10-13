# Decision — `.axmr` stays canonical JSON lines (no compressed binary form)

**Date:** 2026-10-13 · **Decided by:** Tristen Pierson (owner) · **Status:** decided

## The question

Should the `.axmr` file format change from its current form to a compressed binary form? As it stands (DESIGN.md §4), a ledger is a single file: the first line is the magic `AXMR1`, each following line is one event written as canonical JSON — object keys sorted recursively, compact separators — and the hashes that protect the ledger are computed over exactly this encoding. Appends are flushed to disk (fsync'd) line by line, and payloads are stored as readable text.

## The decision

Keep the format as it stands. No compressed binary form.

## Why

1. **The file is the thing the hashes are computed over.** Because the stored bytes are the canonical form itself, two independent implementations can verify each other's files byte for byte, and anyone can open a ledger in a text editor and inspect it with generic tools. That inspectability by strangers is the product property — a reader does not have to trust our software to check our records. A compressed binary form would put our decoder between every reader and every record: each inspection, and each dispute about whether a file verifies, would first have to argue about the decoding step.
2. **Compression fights the append-only design.** Events are appended one line at a time and each append is fsync'd, so a write torn by a crash or power loss costs at most the last line, and the hash chain shows exactly where the damage is. A compressed stream is one interdependent whole: damage can make everything after the tear unreadable.
3. **Size is not a real pressure at ledger scale.** A ledger records claims, snapshots, and checkpoints; the ledgers this system actually produces — study ledgers and institutional records — are small next to the artifacts they refer to. Large artifacts do not belong inside event payloads at all: the ingest discipline in DESIGN.md §6 already says secrets, personal data, and bulk content are referenced by hash, not embedded.
4. **Bulk copies can still be compressed — outside the format.** When a sealed ledger needs to move or sit in an archive, it may be compressed as an outer wrapper around a copy, the way any document can be zipped for transport. Verification then runs on the decompressed canonical bytes. The format itself does not change.

## The costs, named honestly

Files are larger than a compressed equivalent would be, and payloads are readable by anyone who holds the file. The second cost is real, but compression would not fix it — compression is not confidentiality. Keeping payload contents private is the separate encryption and erasure work already recorded in DESIGN.md §6 (the erasure exception); it is not a format question, and this decision does not change it.

## What could reopen this

Two things, neither of which is a current use: recording at very high volume — orders of magnitude beyond study-ledger scale — or a deployment shape that embeds large artifacts inside events instead of referencing them by hash. If either arrives, the standing constraint from DESIGN.md §4 applies: any future binary framing must preserve the same event model and the same hash definitions. It would be an additional framing alongside this one, not a replacement for it.

## Relation to the existing record

DESIGN.md §4 and §7 already list "no compression" among the v0.1 non-goals. This note records the decision and its reasoning, so the non-goal is a settled choice with stated grounds — not re-litigated without new facts of the kind named above.

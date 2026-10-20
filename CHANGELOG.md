# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-10-20

These entries record the verification-policy work (Spec 003), driven by
an adversarial review of the first release. The review showed that
verification checked the consistency of whatever a ledger file
contained, but never checked that what should be there was there.
Each entry names what was wrong in plain terms and what changed.

### Added

- Checkpoint roots are recomputed, never trusted. When verification reaches a checkpoint, it rebuilds the root from the hashes of the events the checkpoint covers and compares. A checkpoint root copied from another ledger — even one re-signed by its true author — now fails with the reason `checkpoint-root-mismatch`. Before, the root was read from the file's own payload and never recomputed, so a transplanted root passed.
- Signature presence is enforced. In a ledger that carries signatures, every event must carry one; verification fails and names the unsigned events. Verification results now also report how many signatures were checked and failed, which events are unsigned, and which actors a trusted key map does not cover.
- Trusted verification is strict, and reachable from every surface. Given a map of actor to public key, every actor in the ledger must be named in the map and every event must be signed with that actor's key — there is no fallback to keys found inside the file. The core, the command-line tool (`--trusted`, `--expect-head` and siblings), and the Python bindings crate all expose the same policy and return the same full result.
- Expected-head pinning. Any verify call can pin the end of the ledger — the last event's hash, a minimum event count, the latest checkpoint root — and fails with a named reason (`head-mismatch`, `shortfall`, `root-mismatch`) when the file does not reach it. This is the only way a shortened copy of a ledger is caught: a shorter copy is still a valid chain on its own, and the documentation now says so plainly instead of leaving the property silent.
- Snapshot and withdrawal rules are enforced wherever a ledger is read: snapshot identities are recomputed and their lineage checked, and a withdrawn ("dead") record stays dead — no later snapshot can bring it back.

### Changed

- **Behavior change — files are read strictly.** Verification of a ledger file now requires every line to be the exact canonical writing of its event: fixed key order, no extra fields, no respelled values, no stray blank lines or line-ending changes. A file that differs by even one byte from the canonical writing fails with the reason `non-canonical-line`, naming the first differing line. Files written by any released version are already canonical, and every frozen or deposited ledger was re-verified unchanged under the strict reading (17 of 17 identical); hand-edited or re-serialized copies will now fail instead of passing.
- **Behavior change — the Python bindings refuse values they cannot store exactly.** Appending a payload that carries NaN, positive or negative infinity, or an integer outside the exactly representable range now raises `ValueError` and appends nothing. Before, the bindings silently stored NaN as null and rounded very large integers, so the file on disk was not what the caller submitted. `verify_file` also changed shape: it returns the full verification result, and a malformed or unreadable file comes back as a failed result with a named reason instead of raising or answering in part.

### Fixed

- Verification no longer stops at what is present. The four gaps the review demonstrated — checkpoint roots taken on faith, signatures allowed to be absent, snapshot lineage asserted rather than validated, and trusted mode reachable only from the Rust library — are all closed above, and the crafted files from the review are kept in this repository as regression fixtures, so each forgery is re-tested on every change.

### Notes for the release

- Still open, by a decision recorded in Spec 003 (AXM-003-15): the companion work in the other repositories. The Python package (axiomere-python) has been rebuilt against this core and exposes the same policy. Tekmerion now pins against the recomputed root in its freeze, export, and bundle-verification paths and requires trusted verification at signed study boundaries. One part remains open: platform ledgers in Tekmerion hold no enrolled ledger-signing key, so trusted-mode verification at that layer waits on a key-custody design decision in Tekmerion's Spec 005 (how such a key is generated, stored across restarts, and rotated). A forged platform ledger is refused when opened today; the stronger positive check is the part still missing, and it is named here so the release does not claim it.

## [0.1.0] - 2026-10-13

The `v0.1.0` tag marks the exact core commit (`40d6933`) the published Python files were built against; released 2026-10-13.

### Added

- An append-only ledger in the `.axmr` format: a single file holding one event per line, each line written as canonical JSON (keys in a fixed order, no extra spaces), with every append flushed to disk. The current state is worked out by reading the events in order.
- A SHA-256 hash chain over events, so changing any earlier event breaks the chain and is detected on verification.
- Signed appends using Ed25519, so each append can be checked against the key that signed it.
- Content-addressed snapshots and Merkle checkpoints, so a record can be verified on its own without trusting the machine that holds it.
- No update or delete operations, by design: corrections are new events appended to the record, never edits to what is already there.
- The Python bindings crate (`axiomere-py`), which makes the ledger available from Python.

### Fixed

- Numbers with 17 or more significant digits could be read back one smallest-possible step too high, because the JSON parser was not correctly rounded. Fixed by enabling correctly rounded parsing. The 0.1.0 core includes this fix, and older study ledgers were re-verified unchanged after it.

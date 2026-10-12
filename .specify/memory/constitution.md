# Axiomere — Project Constitution

**Version:** 1.0.0 · **Ratified:** 2026-10-12 · **Owner:** Tristen Pierson, Axiovex Systems, LLC

This constitution governs all work in this repository. It exists because Axiomere's guarantee — *nothing is ever deleted* — is only credible if the project's own process cannot quietly weaken it. Development follows the spec-kit + AEE flow: claims before code, assessments on record, deviations recorded rather than hidden.

## Article I — Nothing is deleted

There is no delete or update-in-place operation in Axiomere — not in the API, not in a CLI flag, not as an admin backdoor, not behind a feature flag. A pull request that introduces one is rejected on this article alone, regardless of its other merits. Change happens only by appending new snapshots; withdrawal is a dead snapshot (Article II).

## Article II — Revision is a snapshot; withdrawal is death, in the open

Every revision is a new, content-addressed snapshot naming its parent. Withdrawal appends a snapshot with `status: dead`, a mandatory reason, and `score: 0.0`. Score (epistemic weight) and status (lifecycle) are distinct fields and are never conflated. History — including deaths and their reasons — remains fully readable forever.

## Article III — Every append is chained; checkpoints are signed; anchors are external

Events are SHA-256 hash-chained over a canonical encoding. Appends may be Ed25519-signed by their actor; Merkle checkpoints close ranges under a signed root. A deployment that skips external anchoring of checkpoint roots must say so in its own record — layers 1–4 make tampering evident to the operator; only anchoring makes it evident to everyone.

## Article IV — Honest labeling

What is not implemented is labeled as not implemented, wherever it is mentioned — in the README, the design docs, and release notes alike. Specified-but-unbuilt mechanisms (external anchor transport, payload encryption and the erasure-certificate protocol) are never described in the present tense as capabilities.

## Article V — The format is a contract

The `.axmr` v1 format (magic `AXMR1`, one canonical-JSON event per line, hash definitions in DESIGN §4) is frozen. Independent implementations must be able to verify an Axiomere file byte-for-byte from the published spec alone. Format changes require a new magic version and a design note.

## Article VI — A test that cannot fail is deleted

Verification features ship with seeded-defect tests: tampered payloads, deleted lines, forged signatures, broken logs. If a defect test cannot be made to fail against a deliberately defective build, the test is decorative and is fixed or removed before merge.

## Article VII — Claims carry falsifiers

Every spec in this repo ships an AEE claims bundle (`aee-claims.json`, engine format) in which each claim names its falsifier, and an assessment record produced by actually running the assessor. Building ahead of the claims bundle is a process deviation: permitted only when recorded as one in the assessment that catches up.

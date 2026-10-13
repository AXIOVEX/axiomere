# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

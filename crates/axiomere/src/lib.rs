//! Axiomere — a truth ledger.
//!
//! An append-only, signed, hash-chained record store designed for
//! truthfulness, provenance, and change management.
//!
//! # The one rule
//!
//! **There is no delete.** This API deliberately contains no update,
//! delete, remove, or redact function — not public, not private, not
//! behind a feature flag. Records change only by appending new
//! *snapshots* of older records (see [`Ledger::append_snapshot`]):
//! every snapshot is content-addressed, names its parent snapshot,
//! and stays readable forever. Withdrawal is a new snapshot with
//! `status: "dead"`, a mandatory reason, and `score: 0.0`
//! (see [`Ledger::mark_dead`]).
//!
//! # Integrity layers (design: docs/DESIGN.md)
//!
//! 1. SHA-256 hash chain over canonical event encodings.
//! 2. Content-addressed snapshots with parent links (git-like lineage).
//! 3. Ed25519 signatures on appends (actor keys).
//! 4. Signed Merkle checkpoints over event ranges.
//! 5. External anchoring of checkpoint roots (transport is the
//!    caller's; this crate produces the root and the receipt data).
//!
//! # File format
//!
//! A ledger is a single file, extension `.axmr`, whose first line is
//! the magic `AXMR1`, followed by one canonical-JSON event per line.

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAGIC: &str = "AXMR1";
pub const EXTENSION: &str = "axmr";
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

// ---------------------------------------------------------------- errors

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    BadMagic,
    Invalid(String),
    Verification { seq: u64, reason: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Json(e) => write!(f, "json: {e}"),
            Error::BadMagic => write!(f, "not an Axiomere file (missing {MAGIC} header)"),
            Error::Invalid(m) => write!(f, "invalid: {m}"),
            Error::Verification { seq, reason } => {
                write!(f, "verification failed at seq {seq}: {reason}")
            }
        }
    }
}

impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// ---------------------------------------------------------------- events

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub seq: u64,
    #[serde(rename = "type")]
    pub event_type: String,
    pub ts: u64,
    pub actor: String,
    pub payload: Value,
    pub prev_hash: String,
    pub hash: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub signer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sig: Option<String>,
}

/// Canonical JSON: object keys sorted recursively, compact separators.
/// Hashing and signing are defined over this encoding, so two writers
/// can never disagree about the bytes of the same logical event.
pub fn canonical(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .iter()
                .map(|k| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap(),
                        canonical(&map[*k])
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", parts.join(","))
        }
        other => serde_json::to_string(other).unwrap(),
    }
}

fn sha256_hex(data: &str) -> String {
    let mut h = Sha256::new();
    h.update(data.as_bytes());
    hex::encode(h.finalize())
}

fn event_body(ev: &Event) -> Value {
    let mut m = Map::new();
    m.insert("seq".into(), json!(ev.seq));
    m.insert("type".into(), json!(ev.event_type));
    m.insert("ts".into(), json!(ev.ts));
    m.insert("actor".into(), json!(ev.actor));
    m.insert("payload".into(), ev.payload.clone());
    m.insert("prev_hash".into(), json!(ev.prev_hash));
    Value::Object(m)
}

fn event_hash(prev_hash: &str, body: &Value) -> String {
    sha256_hex(&format!("{}{}", prev_hash, canonical(body)))
}

// ---------------------------------------------------------------- ledger

pub struct Ledger {
    path: PathBuf,
    seq: u64,
    head: String,
    signer: Option<SigningKey>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Ledger {
    /// Create a new ledger file (fails if a non-empty file exists).
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if path.exists() && std::fs::metadata(&path)?.len() > 0 {
            return Err(Error::Invalid(format!(
                "{} already exists and is not empty",
                path.display()
            )));
        }
        let mut f = File::create(&path)?;
        writeln!(f, "{MAGIC}")?;
        f.sync_all()?;
        Ok(Ledger {
            path,
            seq: 0,
            head: GENESIS_HASH.to_string(),
            signer: None,
        })
    }

    /// Open an existing ledger, verifying the chain head state as we go.
    /// A log that fails verification cannot be opened for appends — a
    /// broken record is read-only evidence, never a writing surface.
    /// Since Spec 003 the read is strict (FR-VP-08) and the
    /// verification is the full default policy (C3, strengthened).
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let events = read_events_strict(&path).map_err(|fault| match fault {
            FileFault::Io(e) => Error::Io(e),
            FileFault::BadMagic => Error::BadMagic,
            other => Error::Verification {
                seq: other.break_seq().unwrap_or(0),
                reason: other.reason().to_string(),
            },
        })?;
        let result = verify_events(&events);
        if !result.ok {
            return Err(Error::Verification {
                seq: result.break_seq.unwrap_or(0),
                reason: result.reason.unwrap_or_default(),
            });
        }
        let (seq, head) = events
            .last()
            .map(|e| (e.seq, e.hash.clone()))
            .unwrap_or((0, GENESIS_HASH.to_string()));
        Ok(Ledger {
            path,
            seq,
            head,
            signer: None,
        })
    }

    /// Attach a signing key: subsequent appends are Ed25519-signed.
    pub fn with_signer(mut self, seed: &[u8; 32]) -> Self {
        self.signer = Some(SigningKey::from_bytes(seed));
        self
    }

    pub fn public_key_hex(&self) -> Option<String> {
        self.signer
            .as_ref()
            .map(|k| hex::encode(k.verifying_key().to_bytes()))
    }

    /// Append one event. This — and only this — is how a ledger changes.
    pub fn append(&mut self, event_type: &str, payload: Value, actor: &str) -> Result<Event> {
        let mut ev = Event {
            seq: self.seq + 1,
            event_type: event_type.to_string(),
            ts: now_secs(),
            actor: actor.to_string(),
            payload,
            prev_hash: self.head.clone(),
            hash: String::new(),
            signer: None,
            sig: None,
        };
        ev.hash = event_hash(&ev.prev_hash, &event_body(&ev));
        if let Some(key) = &self.signer {
            let sig = key.sign(ev.hash.as_bytes());
            ev.signer = Some(hex::encode(key.verifying_key().to_bytes()));
            ev.sig = Some(hex::encode(sig.to_bytes()));
        }
        let mut f = OpenOptions::new().append(true).open(&self.path)?;
        writeln!(f, "{}", canonical(&serde_json::to_value(&ev)?))?;
        f.sync_all()?;
        self.seq = ev.seq;
        self.head = ev.hash.clone();
        Ok(ev)
    }

    pub fn events(&self) -> Result<Vec<Event>> {
        read_events(&self.path)
    }

    /// Verify this ledger's file under the default policy. The file
    /// is read strictly (FR-VP-08): the verified object is the bytes
    /// on disk, not a lenient re-parse of them.
    pub fn verify(&self) -> Result<Verification> {
        Ok(verify_file(&self.path, &VerifyPolicy::default()))
    }

    /// Verify this ledger's file under an explicit policy (trusted
    /// map and/or expected-head pin).
    pub fn verify_with_policy(&self, policy: &VerifyPolicy) -> Result<Verification> {
        Ok(verify_file(&self.path, policy))
    }

    // ------------------------------------------------------------ snapshots

    /// Append a new snapshot of a record. `parent` is resolved from the
    /// record's current latest snapshot; the snapshot's content address
    /// binds content + parent, so lineage itself is tamper-evident.
    pub fn append_snapshot(
        &mut self,
        record_id: &str,
        content: Value,
        status: &str,
        score: Option<f64>,
        reason: Option<&str>,
        actor: &str,
    ) -> Result<Event> {
        if status != "live" && status != "dead" {
            return Err(Error::Invalid(format!(
                "status must be \"live\" or \"dead\", got {status:?}"
            )));
        }
        // FR-VP-06 (write side): death in the core requires a reason
        // on every path, and death is terminal — no live snapshot may
        // follow a dead one for the same record.
        if status == "dead" {
            match reason {
                Some(r) if !r.is_empty() => {}
                _ => {
                    return Err(Error::Invalid(
                        "a dead snapshot requires a non-empty reason".into(),
                    ))
                }
            }
        }
        if status == "live" {
            let view = self.current_view(true)?;
            if let Some(cur) = view.get(record_id) {
                if cur.get("status").and_then(|v| v.as_str()) == Some("dead") {
                    return Err(Error::Invalid(format!(
                        "record {record_id} is dead; death is terminal — a live snapshot cannot follow a dead one"
                    )));
                }
            }
        }
        let parent = self
            .current_view(true)?
            .get(record_id)
            .and_then(|s| s.get("snapshot_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let snapshot_id = snapshot_id(&content, parent.as_deref());
        let score = if status == "dead" { Some(0.0) } else { score };
        let payload = json!({
            "record_id": record_id,
            "snapshot_id": snapshot_id,
            "parent": parent,
            "status": status,
            "score": score,
            "reason": reason,
            "content": content,
        });
        self.append("record.snapshot", payload, actor)
    }

    /// Withdraw a record the only way Axiomere allows: a new snapshot,
    /// marked dead, reason on the record, score forced to 0.0. The
    /// record's history — including this death — remains fully readable.
    pub fn mark_dead(&mut self, record_id: &str, reason: &str, actor: &str) -> Result<Event> {
        let view = self.current_view(true)?;
        let current = view
            .get(record_id)
            .ok_or_else(|| Error::Invalid(format!("unknown record {record_id}")))?;
        let content = current
            .get("content")
            .cloned()
            .unwrap_or(Value::Null);
        self.append_snapshot(record_id, content, "dead", Some(0.0), Some(reason), actor)
    }

    /// The current view: latest snapshot per record, folded from the log.
    /// Dead records are excluded unless `include_dead` is set. The fold
    /// is a projection — the log itself is never altered to produce it.
    pub fn current_view(&self, include_dead: bool) -> Result<BTreeMap<String, Value>> {
        let events = self.events()?;
        // The fold validates lineage as it goes (FR-VP-05/06): a view
        // of an invalid lineage is refused, never laundered into a
        // projection. (Ledgers reached through `Ledger::open` are
        // already fully verified; this keeps the fold honest for any
        // other construction path.)
        let mut fold_state: BTreeMap<String, RecordState> = BTreeMap::new();
        for ev in &events {
            if let Err(reason) = fold_validate(&mut fold_state, ev) {
                return Err(Error::Verification {
                    seq: ev.seq,
                    reason,
                });
            }
        }
        let mut view: BTreeMap<String, Value> = BTreeMap::new();
        for ev in events {
            if ev.event_type == "record.snapshot" {
                let p = &ev.payload;
                if let Some(rid) = p.get("record_id").and_then(|v| v.as_str()) {
                    view.insert(rid.to_string(), p.clone());
                }
            }
        }
        if !include_dead {
            view.retain(|_, s| s.get("status").and_then(|v| v.as_str()) != Some("dead"));
        }
        Ok(view)
    }

    // ------------------------------------------------------------ checkpoints

    /// Close the current event range under a Merkle checkpoint: appends
    /// a `checkpoint` event carrying the Merkle root over all event
    /// hashes so far. Anchor that root externally (docs/DESIGN.md §5)
    /// to make history rewriting detectable by anyone.
    pub fn checkpoint(&mut self, actor: &str) -> Result<Event> {
        let events = self.events()?;
        let hashes: Vec<&str> = events.iter().map(|e| e.hash.as_str()).collect();
        let root = merkle_root(&hashes);
        self.append(
            "checkpoint",
            json!({"merkle_root": root, "upto_seq": self.seq, "event_count": events.len()}),
            actor,
        )
    }
}

/// Content address of a snapshot: hash of content + parent address.
pub fn snapshot_id(content: &Value, parent: Option<&str>) -> String {
    let body = json!({"content": content, "parent": parent});
    sha256_hex(&canonical(&body))
}

/// Merkle root over hex hash strings (empty set = hash of empty string).
pub fn merkle_root(hashes: &[&str]) -> String {
    if hashes.is_empty() {
        return sha256_hex("");
    }
    let mut level: Vec<String> = hashes.iter().map(|s| s.to_string()).collect();
    while level.len() > 1 {
        let mut next = Vec::new();
        for pair in level.chunks(2) {
            let right = if pair.len() == 2 { pair[1].as_str() } else { pair[0].as_str() };
            next.push(sha256_hex(&format!("{}{}", pair[0], right)));
        }
        level = next;
    }
    level.into_iter().next().unwrap()
}

// ---------------------------------------------------------------- reading

pub fn read_events(path: impl AsRef<Path>) -> Result<Vec<Event>> {
    let f = File::open(path.as_ref())?;
    let mut lines = BufReader::new(f).lines();
    let first = lines.next().transpose()?;
    match first.as_deref() {
        Some(MAGIC) => {}
        _ => return Err(Error::BadMagic),
    }
    let mut out = Vec::new();
    for line in lines {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str::<Event>(&line)?);
    }
    Ok(out)
}

/// Why a strict file read failed, with the 1-based would-be seq of the
/// offending event line where one exists.
#[derive(Debug)]
pub enum FileFault {
    Io(std::io::Error),
    BadMagic,
    /// The line does not parse as an event at all.
    Unparseable { seq: u64 },
    /// The line parses, but its bytes are not the canonical encoding
    /// of the parsed event (FR-VP-08).
    NonCanonical { seq: u64 },
}

impl FileFault {
    fn reason(&self) -> &'static str {
        match self {
            FileFault::Io(_) => "unreadable-file",
            FileFault::BadMagic => "bad-magic",
            FileFault::Unparseable { .. } => "unparseable-line",
            FileFault::NonCanonical { .. } => "non-canonical-line",
        }
    }
    fn break_seq(&self) -> Option<u64> {
        match self {
            // seq 0 marks a fault on the magic line itself (a CRLF
            // magic): no event was reached, so there is no break seq.
            FileFault::Unparseable { seq } | FileFault::NonCanonical { seq } if *seq > 0 => {
                Some(*seq)
            }
            _ => None,
        }
    }
}

/// Strict file reader (FR-VP-08): the verification paths read a ledger
/// file byte-for-byte. Line 1 must be exactly the magic; every event
/// line must equal `canonical(event)` for the event it parses to. Byte
/// equality subsumes the duplicate-key, unknown-field, respelling,
/// whitespace, CRLF, and blank-line cases: the writer's canonical form
/// contains each key exactly once and nothing else, so any line
/// carrying more — or spelled differently — differs from its own
/// canonical form. A missing final newline is the one tolerated
/// deviation: the last line still equals its canonical form.
pub fn read_events_strict(path: impl AsRef<Path>) -> std::result::Result<Vec<Event>, FileFault> {
    let bytes = std::fs::read(path.as_ref()).map_err(FileFault::Io)?;
    let text = match String::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => {
            // Locate the offending line for a named failure.
            let valid_up_to = e.utf8_error().valid_up_to();
            let seq = e.as_bytes()[..valid_up_to]
                .iter()
                .filter(|&&b| b == b'\n')
                .count() as u64;
            return Err(FileFault::Unparseable { seq: seq.max(1) });
        }
    };
    let mut lines: Vec<&str> = text.split('\n').collect();
    // A trailing newline yields one final empty segment — the file's
    // terminator, not a blank line in the event region.
    if lines.last().map(|l| l.is_empty()) == Some(true) {
        lines.pop();
    }
    match lines.first() {
        Some(&MAGIC) => {}
        // A magic line carrying a carriage return is a CRLF file:
        // FR-VP-08 names CRLF a non-canonical-line fault, not a bad
        // magic. Genuinely wrong magic stays BadMagic.
        Some(line) if line.strip_suffix('\r') == Some(MAGIC) => {
            return Err(FileFault::NonCanonical { seq: 0 });
        }
        _ => return Err(FileFault::BadMagic),
    }
    let mut out = Vec::new();
    for (idx, line) in lines.iter().skip(1).enumerate() {
        let seq = (idx + 1) as u64;
        if line.is_empty() {
            return Err(FileFault::NonCanonical { seq });
        }
        let ev: Event = match serde_json::from_str(line) {
            Ok(ev) => ev,
            Err(_) => return Err(FileFault::Unparseable { seq }),
        };
        let canon = canonical(&serde_json::to_value(&ev).unwrap());
        if canon != *line {
            return Err(FileFault::NonCanonical { seq: ev.seq });
        }
        out.push(ev);
    }
    Ok(out)
}

/// Verify a ledger **file** under a policy (FR-VP-08 + FR-VP-11): the
/// strict reader plus the full verification policy. This function
/// always returns a verdict — malformed and unreadable files are
/// `ok=false` results with a named reason, never an exception.
pub fn verify_file(path: impl AsRef<Path>, policy: &VerifyPolicy) -> Verification {
    match read_events_strict(path) {
        Ok(events) => verify_events_with_policy(&events, policy),
        Err(fault) => Verification {
            ok: false,
            events: fault.break_seq().map(|s| (s - 1) as usize).unwrap_or(0),
            break_seq: fault.break_seq(),
            reason: Some(fault.reason().to_string()),
            signatures_checked: 0,
            signatures_failed: 0,
            unsigned_events: Vec::new(),
            untrusted_actors: Vec::new(),
        },
    }
}

#[derive(Debug, Clone)]
pub struct Verification {
    pub ok: bool,
    pub events: usize,
    pub break_seq: Option<u64>,
    pub reason: Option<String>,
    pub signatures_checked: usize,
    pub signatures_failed: usize,
    /// Seqs of events lacking `signer`, `sig`, or both. Reported in
    /// every mode (Spec 003 FR-VP-02): an unsigned or partially signed
    /// state is always visible, never silent. A failure is raised only
    /// where the policy requires signatures (a ledger carrying
    /// signature material, or trusted mode).
    pub unsigned_events: Vec<u64>,
    /// Actors seen in trusted mode that are absent from the trusted
    /// map (Spec 003 FR-VP-03). Empty outside trusted mode.
    pub untrusted_actors: Vec<String>,
}

/// An expected head to pin verification against (Spec 003 FR-VP-10).
/// Any combination of fields may be set. Bare verification (no pin)
/// accepts any valid prefix of a chain — a truncated file passes —
/// and the pin is the detection.
#[derive(Debug, Clone, Default)]
pub struct ExpectedHead {
    /// The hash of the last event the caller expects.
    pub head_hash: Option<String>,
    /// The minimum number of events the caller expects.
    pub min_events: Option<usize>,
    /// The Merkle root of the latest checkpoint, compared against the
    /// root **recomputed** during verification — never the payload's
    /// claim (FR-VP-01).
    pub checkpoint_root: Option<String>,
}

/// The verification policy (Spec 003 §4). The structural checks —
/// checkpoint recomputation, signature presence, snapshot/death
/// validation, chain integrity — are unconditional: they restate the
/// reference writer's own guarantees. What the caller chooses is whose
/// keys to trust and what head to pin.
#[derive(Debug, Clone, Default)]
pub struct VerifyPolicy {
    /// `None` = self-consistency (signatures checked against the keys
    /// embedded in the events). `Some(map)` = identity mode, strict:
    /// every event's actor must be in the map, every event must be
    /// signed, and the signer must equal the mapped key. An empty map
    /// trusts nobody — it is NOT the same input as `None`.
    pub trusted: Option<BTreeMap<String, String>>,
    pub expected_head: Option<ExpectedHead>,
}

/// Verify a parsed event list under the default policy
/// (self-consistency, no head pin).
pub fn verify_events(events: &[Event]) -> Verification {
    verify_events_with_policy(events, &VerifyPolicy::default())
}

/// Verify with an optional trusted-key map (`None` = self-consistency;
/// `Some` = strict identity mode, FR-VP-03).
pub fn verify_events_trusted(
    events: &[Event],
    trusted: Option<&BTreeMap<String, String>>,
) -> Verification {
    verify_events_with_policy(
        events,
        &VerifyPolicy {
            trusted: trusted.cloned(),
            expected_head: None,
        },
    )
}

fn is_signed(ev: &Event) -> bool {
    ev.signer.is_some() && ev.sig.is_some()
}

/// The validating fold's per-record state (FR-VP-05/06).
#[derive(Default)]
struct RecordState {
    snapshot_ids: Vec<String>,
    dead: bool,
}

/// Validate one event against the snapshot lineage state, updating the
/// state on success. Returns the failure reason. Only `record.snapshot`
/// events are constrained; every other event type passes. Shared by
/// verification and `current_view`, so a forged lineage is refused by
/// both — never projected, never adopted as a parent.
fn fold_validate(
    state: &mut BTreeMap<String, RecordState>,
    ev: &Event,
) -> std::result::Result<(), String> {
    if ev.event_type != "record.snapshot" {
        return Ok(());
    }
    let p = ev.payload.as_object().ok_or("invalid-snapshot-payload")?;
    let field = |k: &str| p.get(k).ok_or("invalid-snapshot-payload");
    let record_id = field("record_id")?.as_str().ok_or("invalid-snapshot-payload")?;
    let snap_id = field("snapshot_id")?.as_str().ok_or("invalid-snapshot-payload")?;
    let parent_val = field("parent")?;
    let parent: Option<&str> = match parent_val {
        Value::Null => None,
        Value::String(s) => Some(s.as_str()),
        _ => return Err("invalid-snapshot-payload".into()),
    };
    let status = field("status")?.as_str().ok_or("invalid-snapshot-payload")?;
    if status != "live" && status != "dead" {
        return Err("invalid-status".into());
    }
    let score = field("score")?;
    if !(score.is_null() || score.is_number()) {
        return Err("invalid-snapshot-payload".into());
    }
    let reason = field("reason")?;
    if !(reason.is_null() || reason.is_string()) {
        return Err("invalid-snapshot-payload".into());
    }
    let content = field("content")?;
    if snapshot_id(content, parent) != snap_id {
        return Err("snapshot-id-mismatch".into());
    }
    let rec = state.entry(record_id.to_string()).or_default();
    match parent {
        None => {
            if !rec.snapshot_ids.is_empty() {
                return Err("snapshot-parent-unknown".into());
            }
        }
        Some(par) => {
            if !rec.snapshot_ids.iter().any(|s| s == par) {
                return Err("snapshot-parent-unknown".into());
            }
        }
    }
    if status == "dead" {
        match reason.as_str() {
            Some(r) if !r.is_empty() => {}
            _ => return Err("dead-reason-missing".into()),
        }
        match score.as_f64() {
            Some(s) if s == 0.0 => {}
            _ => return Err("dead-score-nonzero".into()),
        }
    }
    if rec.dead && status == "live" {
        return Err("live-after-dead".into());
    }
    rec.snapshot_ids.push(snap_id.to_string());
    if status == "dead" {
        rec.dead = true;
    }
    Ok(())
}

/// Verify a parsed event list under an explicit policy: sequence,
/// links, hashes, checkpoint recomputation (FR-VP-01), snapshot
/// lineage and death rules (FR-VP-05/06), signature presence
/// (FR-VP-02), strict trusted mode (FR-VP-03), and the expected-head
/// pin (FR-VP-10).
pub fn verify_events_with_policy(events: &[Event], policy: &VerifyPolicy) -> Verification {
    // Presence is a whole-ledger scan, computed up front so it is
    // reported identically on success and on every failure.
    let unsigned_events: Vec<u64> = events
        .iter()
        .filter(|e| !is_signed(e))
        .map(|e| e.seq)
        .collect();
    let carries_signature_material = events
        .iter()
        .any(|e| e.signer.is_some() || e.sig.is_some());
    let mut untrusted_actors: Vec<String> = Vec::new();
    let mut prev = GENESIS_HASH.to_string();
    let mut sig_checked = 0usize;
    let mut sig_failed = 0usize;
    let mut hashes: Vec<String> = Vec::with_capacity(events.len());
    let mut fold: BTreeMap<String, RecordState> = BTreeMap::new();
    let mut last_checkpoint_root: Option<String> = None;
    let fail = |reason: &str,
                seq: u64,
                checked: usize,
                failed: usize,
                unsigned: Vec<u64>,
                untrusted: Vec<String>| Verification {
        ok: false,
        events: events.len(),
        break_seq: Some(seq),
        reason: Some(reason.to_string()),
        signatures_checked: checked,
        signatures_failed: failed,
        unsigned_events: unsigned,
        untrusted_actors: untrusted,
    };
    for (i, ev) in events.iter().enumerate() {
        let expect_seq = (i + 1) as u64;
        if ev.seq != expect_seq {
            return fail("seq-gap", ev.seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
        }
        if ev.prev_hash != prev {
            return fail("prev-link-mismatch", ev.seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
        }
        if ev.hash != event_hash(&ev.prev_hash, &event_body(ev)) {
            return fail("hash-mismatch", ev.seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
        }
        hashes.push(ev.hash.clone());
        // FR-VP-01: a checkpoint's claimed root is never trusted. The
        // writer commits the events preceding the checkpoint; recompute
        // exactly that set and compare.
        if ev.event_type == "checkpoint" {
            let claimed = ev.payload.get("merkle_root").and_then(|v| v.as_str());
            let upto = ev.payload.get("upto_seq").and_then(|v| v.as_u64());
            match (claimed, upto) {
                (Some(root), Some(upto)) if (upto as usize) <= i => {
                    let refs: Vec<&str> =
                        hashes[..upto as usize].iter().map(|s| s.as_str()).collect();
                    let recomputed = merkle_root(&refs);
                    if recomputed != root {
                        return fail("checkpoint-root-mismatch", ev.seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
                    }
                    last_checkpoint_root = Some(recomputed);
                }
                (Some(_), Some(_)) => {
                    return fail("checkpoint-root-mismatch", ev.seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
                }
                _ => {
                    return fail("malformed-checkpoint", ev.seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
                }
            }
        }
        // FR-VP-05/06: snapshot lineage + death rules, on read.
        if let Err(reason) = fold_validate(&mut fold, ev) {
            return fail(&reason, ev.seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
        }
        // FR-VP-02/03: presence and identity.
        let signed = is_signed(ev);
        if let Some(map) = &policy.trusted {
            if !map.contains_key(&ev.actor) {
                if !untrusted_actors.contains(&ev.actor) {
                    untrusted_actors.push(ev.actor.clone());
                }
                return fail("untrusted-actor", ev.seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
            }
            if !signed {
                return fail("unsigned-event", ev.seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
            }
        } else if !signed && carries_signature_material {
            return fail("unsigned-event", ev.seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
        }
        if signed {
            sig_checked += 1;
            let signer_hex = ev.signer.as_ref().unwrap();
            let sig_hex = ev.sig.as_ref().unwrap();
            if let Some(map) = &policy.trusted {
                if map.get(&ev.actor).map(|s| s.as_str()) != Some(signer_hex.as_str()) {
                    sig_failed += 1;
                    return fail("signer-key-mismatch", ev.seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
                }
            }
            let ok = (|| -> Option<bool> {
                let pk_bytes = hex::decode(signer_hex).ok()?;
                let pk: [u8; 32] = pk_bytes.try_into().ok()?;
                let vk = VerifyingKey::from_bytes(&pk).ok()?;
                let sig_bytes = hex::decode(sig_hex).ok()?;
                let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes.try_into().ok()?);
                Some(vk.verify(ev.hash.as_bytes(), &sig).is_ok())
            })()
            .unwrap_or(false);
            if !ok {
                sig_failed += 1;
                return fail("signature-invalid", ev.seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
            }
        }
        prev = ev.hash.clone();
    }
    // FR-VP-10: the expected-head pin — the only truncation detection.
    if let Some(head) = &policy.expected_head {
        let last_seq = events.last().map(|e| e.seq).unwrap_or(0);
        if let Some(min) = head.min_events {
            if events.len() < min {
                return fail("shortfall", last_seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
            }
        }
        if let Some(h) = &head.head_hash {
            if &prev != h {
                return fail("head-mismatch", last_seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
            }
        }
        if let Some(r) = &head.checkpoint_root {
            if last_checkpoint_root.as_deref() != Some(r.as_str()) {
                return fail("root-mismatch", last_seq, sig_checked, sig_failed, unsigned_events, untrusted_actors);
            }
        }
    }
    Verification {
        ok: true,
        events: events.len(),
        break_seq: None,
        reason: None,
        signatures_checked: sig_checked,
        signatures_failed: sig_failed,
        unsigned_events,
        untrusted_actors,
    }
}

/// Generate a new signing seed (32 bytes) and its public key, hex-encoded.
pub fn generate_keypair() -> (String, String) {
    let key = SigningKey::generate(&mut rand::rngs::OsRng);
    (
        hex::encode(key.to_bytes()),
        hex::encode(key.verifying_key().to_bytes()),
    )
}

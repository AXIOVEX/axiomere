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
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let events = read_events(&path)?;
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

    pub fn verify(&self) -> Result<Verification> {
        Ok(verify_events(&self.events()?))
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
        let mut view: BTreeMap<String, Value> = BTreeMap::new();
        for ev in self.events()? {
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

#[derive(Debug, Clone)]
pub struct Verification {
    pub ok: bool,
    pub events: usize,
    pub break_seq: Option<u64>,
    pub reason: Option<String>,
    pub signatures_checked: usize,
    pub signatures_failed: usize,
}

/// Verify a parsed event list: sequence, links, hashes, and — where a
/// trusted-keys map is supplied — signatures. Without a map, embedded
/// signer keys are checked for self-consistency (the signature must
/// verify against the key embedded in the event); trust in that key
/// comes from comparing it to a key you hold out-of-band.
pub fn verify_events(events: &[Event]) -> Verification {
    verify_events_trusted(events, None)
}

pub fn verify_events_trusted(
    events: &[Event],
    trusted: Option<&BTreeMap<String, String>>,
) -> Verification {
    let mut prev = GENESIS_HASH.to_string();
    let mut sig_checked = 0usize;
    let mut sig_failed = 0usize;
    for (i, ev) in events.iter().enumerate() {
        let expect_seq = (i + 1) as u64;
        let fail = |reason: &str, checked, failed| Verification {
            ok: false,
            events: events.len(),
            break_seq: Some(ev.seq),
            reason: Some(reason.to_string()),
            signatures_checked: checked,
            signatures_failed: failed,
        };
        if ev.seq != expect_seq {
            return fail("seq-gap", sig_checked, sig_failed);
        }
        if ev.prev_hash != prev {
            return fail("prev-link-mismatch", sig_checked, sig_failed);
        }
        if ev.hash != event_hash(&ev.prev_hash, &event_body(ev)) {
            return fail("hash-mismatch", sig_checked, sig_failed);
        }
        if let (Some(sig_hex), Some(signer_hex)) = (&ev.sig, &ev.signer) {
            sig_checked += 1;
            let expected_key = trusted
                .and_then(|m| m.get(&ev.actor))
                .map(|s| s.as_str())
                .unwrap_or(signer_hex.as_str());
            if expected_key != signer_hex {
                sig_failed += 1;
                return fail("signer-key-mismatch", sig_checked, sig_failed);
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
                return fail("signature-invalid", sig_checked, sig_failed);
            }
        }
        prev = ev.hash.clone();
    }
    Verification {
        ok: true,
        events: events.len(),
        break_seq: None,
        reason: None,
        signatures_checked: sig_checked,
        signatures_failed: sig_failed,
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

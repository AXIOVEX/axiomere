#!/usr/bin/env python3
"""From-spec verifier (Spec 003, T311 / FR-VP-09 / AXM-003-10).

An independent `.axmr` reader written ONLY from docs/DESIGN.md section 4
(as expanded by Spec 003) and the frozen vectors in
tests/fixtures/vectors-003.json. It shares no code with the Rust core:
canonical JSON, chain hashes, Merkle roots, snapshot ids, and Ed25519
signature checks are all reimplemented here from the document, in pure
Python stdlib.

Checks per file (bare mode, mirroring DESIGN section 4.2):
  * line 1 is exactly the magic `AXMR1`;
  * every event line equals canonical(event) byte-for-byte;
  * seq runs 1..n, prev_hash links, event hashes recompute;
  * checkpoint Merkle roots are RECOMPUTED over the hashes of the
    events the checkpoint closes and compared with the payload claim;
  * record.snapshot payloads validate (snapshot_id recomputed from
    {content, parent}; parent is null for a record's first snapshot or
    one of its earlier snapshot ids; status in {live, dead}; a dead
    snapshot carries a non-empty reason and score exactly 0.0; death
    is terminal);
  * signature presence: in a ledger carrying any signature, every
    event must carry signer+sig; present signatures verify against
    the embedded key over the event hash's ASCII bytes (Ed25519).

Usage: from_spec_verify.py [ledger.axmr ...]
With no arguments it verifies the honest in-repo fixtures
(victim.axmr, canon.axmr). The frozen vectors are always checked.
Exit status 0 iff every vector reproduces and every file verifies.
"""

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FIXTURES = HERE / "fixtures" / "redteam-2026-10-13"
VECTORS = HERE / "fixtures" / "vectors-003.json"
GENESIS = "0" * 64

# ---------------------------------------------------------------------
# Canonical encoding (DESIGN 4.1)

_SHORT_ESCAPES = {
    "\b": "\\b",
    "\t": "\\t",
    "\n": "\\n",
    "\f": "\\f",
    "\r": "\\r",
}


def escape_string(s: str) -> str:
    out = ['"']
    for ch in s:
        if ch == '"':
            out.append('\\"')
        elif ch == "\\":
            out.append("\\\\")
        elif ch in _SHORT_ESCAPES:
            out.append(_SHORT_ESCAPES[ch])
        elif ord(ch) < 0x20:
            out.append("\\u%04x" % ord(ch))
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


def render_float(x: float) -> str:
    """Shortest round-trip decimal; exponent form always signed,
    never zero-padded (DESIGN 4.1). Python's repr is shortest
    round-trip but zero-pads exponents (1e-07) — normalize."""
    r = repr(x)
    if "e" in r or "E" in r:
        mant, _, exp = r.replace("E", "e").partition("e")
        return "%se%+d" % (mant, int(exp))
    return r


def canonical(v) -> str:
    if v is None:
        return "null"
    if v is True:
        return "true"
    if v is False:
        return "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, float):
        return render_float(v)
    if isinstance(v, str):
        return escape_string(v)
    if isinstance(v, list):
        return "[" + ",".join(canonical(x) for x in v) + "]"
    if isinstance(v, dict):
        parts = [
            escape_string(k) + ":" + canonical(v[k])
            for k in sorted(v.keys(), key=lambda k: k.encode("utf-8"))
        ]
        return "{" + ",".join(parts) + "}"
    raise TypeError(f"unencodable value {v!r}")


def sha256_hex(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def event_hash(prev_hash: str, body: dict) -> str:
    # prev_hash enters as its 64-char lowercase hex TEXT (DESIGN 4.1).
    return sha256_hex(prev_hash + canonical(body))


def merkle_root(hashes) -> str:
    if not hashes:
        return sha256_hex("")
    level = list(hashes)
    while len(level) > 1:
        nxt = []
        for i in range(0, len(level), 2):
            left = level[i]
            right = level[i + 1] if i + 1 < len(level) else left
            nxt.append(sha256_hex(left + right))
        level = nxt
    return level[0]


def snapshot_id(content, parent) -> str:
    return sha256_hex(canonical({"content": content, "parent": parent}))


# ---------------------------------------------------------------------
# Ed25519 verification (RFC 8032), pure Python — self-consistency only.

_Q = 2**255 - 19
_L = 2**252 + 27742317777372353535851937790883648493
_D = (-121665 * pow(121666, _Q - 2, _Q)) % _Q
_I = pow(2, (_Q - 1) // 4, _Q)
_IDENTITY = (0, 1, 1, 0)  # extended coordinates (X, Y, Z, T)


def _point_add(P, Q):
    X1, Y1, Z1, T1 = P
    X2, Y2, Z2, T2 = Q
    A = (Y1 - X1) * (Y2 - X2) % _Q
    B = (Y1 + X1) * (Y2 + X2) % _Q
    C = 2 * _D * T1 * T2 % _Q
    Dv = 2 * Z1 * Z2 % _Q
    E = (B - A) % _Q
    F = (Dv - C) % _Q
    G = (Dv + C) % _Q
    H = (B + A) % _Q
    return (E * F % _Q, G * H % _Q, F * G % _Q, E * H % _Q)


def _scalarmult(P, e: int):
    R = _IDENTITY
    while e > 0:
        if e & 1:
            R = _point_add(R, P)
        P = _point_add(P, P)
        e >>= 1
    return R


def _decompress(bytes32: bytes):
    if len(bytes32) != 32:
        return None
    y = int.from_bytes(bytes32, "little") & (2**255 - 1)
    sign = (bytes32[31] >> 7) & 1
    if y >= _Q:
        return None
    # x = sqrt((y^2 - 1) / (d y^2 + 1))
    xx = (y * y - 1) * pow(_D * y * y + 1, _Q - 2, _Q) % _Q
    x = pow(xx, (_Q + 3) // 8, _Q)
    if (x * x - xx) % _Q != 0:
        x = x * _I % _Q
    if (x * x - xx) % _Q != 0:
        return None
    if x == 0 and sign == 1:
        return None
    if (x & 1) != sign:
        x = _Q - x
    return (x, y, 1, x * y % _Q)


_B = _decompress(bytes.fromhex(
    "5866666666666666666666666666666666666666666666666666666666666666"))


def ed25519_verify(pubkey_hex: str, sig_hex: str, msg: bytes) -> bool:
    try:
        pk = bytes.fromhex(pubkey_hex)
        sig = bytes.fromhex(sig_hex)
    except ValueError:
        return False
    if len(pk) != 32 or len(sig) != 64:
        return False
    A = _decompress(pk)
    R = _decompress(sig[:32])
    if A is None or R is None:
        return False
    s = int.from_bytes(sig[32:], "little")
    if s >= _L:
        return False
    h = int.from_bytes(hashlib.sha512(sig[:32] + pk + msg).digest(),
                       "little") % _L
    # Check [s]B == R + [H]A in extended coordinates (cross-multiplied).
    left = _scalarmult(_B, s)
    right = _point_add(R, _scalarmult(A, h))
    return (left[0] * right[2] - right[0] * left[2]) % _Q == 0 and \
           (left[1] * right[2] - right[1] * left[2]) % _Q == 0


# ---------------------------------------------------------------------
# Ledger verification


class Fail(Exception):
    def __init__(self, reason, seq=None):
        super().__init__(reason)
        self.reason = reason
        self.seq = seq


def verify_ledger(path: Path):
    raw = path.read_bytes()
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError:
        raise Fail("unparseable-line")
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()  # final terminator, not a blank event line
    if not lines or lines[0] != "AXMR1":
        raise Fail("bad-magic")
    events = []
    for idx, line in enumerate(lines[1:]):
        seq = idx + 1
        if line == "":
            raise Fail("non-canonical-line", seq)
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            raise Fail("unparseable-line", seq)
        if not isinstance(ev, dict) or canonical(ev) != line:
            raise Fail("non-canonical-line", seq)
        events.append(ev)

    carries_sig_material = any("signer" in e or "sig" in e for e in events)
    prev = GENESIS
    hashes = []
    records = {}  # record_id -> {"ids": [...], "dead": bool}
    sig_checked = 0
    for idx, ev in enumerate(events):
        seq = idx + 1
        if ev.get("seq") != seq:
            raise Fail("seq-gap", ev.get("seq"))
        if ev.get("prev_hash") != prev:
            raise Fail("prev-link-mismatch", seq)
        body = {k: ev[k] for k in
                ("seq", "type", "ts", "actor", "payload", "prev_hash")}
        if ev.get("hash") != event_hash(ev["prev_hash"], body):
            raise Fail("hash-mismatch", seq)
        hashes.append(ev["hash"])

        if ev["type"] == "checkpoint":
            p = ev["payload"]
            root, upto = p.get("merkle_root"), p.get("upto_seq")
            if not isinstance(root, str) or not isinstance(upto, int) \
                    or upto > idx:
                raise Fail("malformed-checkpoint", seq)
            if merkle_root(hashes[:upto]) != root:
                raise Fail("checkpoint-root-mismatch", seq)

        if ev["type"] == "record.snapshot":
            p = ev["payload"]
            try:
                record_id = p["record_id"]
                snap_id = p["snapshot_id"]
                parent = p["parent"]
                status = p["status"]
                score = p["score"]
                reason = p["reason"]
                content = p["content"]
            except (KeyError, TypeError):
                raise Fail("invalid-snapshot-payload", seq)
            if status not in ("live", "dead"):
                raise Fail("invalid-status", seq)
            if snapshot_id(content, parent) != snap_id:
                raise Fail("snapshot-id-mismatch", seq)
            rec = records.setdefault(record_id, {"ids": [], "dead": False})
            if parent is None:
                if rec["ids"]:
                    raise Fail("snapshot-parent-unknown", seq)
            elif parent not in rec["ids"]:
                raise Fail("snapshot-parent-unknown", seq)
            if status == "dead":
                if not (isinstance(reason, str) and reason):
                    raise Fail("dead-reason-missing", seq)
                if not (isinstance(score, (int, float))
                        and not isinstance(score, bool) and score == 0.0):
                    raise Fail("dead-score-nonzero", seq)
            if rec["dead"] and status == "live":
                raise Fail("live-after-dead", seq)
            rec["ids"].append(snap_id)
            if status == "dead":
                rec["dead"] = True

        signed = "signer" in ev and "sig" in ev
        if carries_sig_material and not signed:
            raise Fail("unsigned-event", seq)
        if signed:
            sig_checked += 1
            if not ed25519_verify(ev["signer"], ev["sig"],
                                  ev["hash"].encode("ascii")):
                raise Fail("signature-invalid", seq)
        prev = ev["hash"]
    return len(events), sig_checked


# ---------------------------------------------------------------------
# Frozen vectors (DESIGN 4.1 / spec section 5)


def check_vectors() -> list:
    v = json.loads(VECTORS.read_text(encoding="utf-8"))
    errs = []

    se = v["string_escape"]
    if canonical(se["input"]) != se["canonical"]:
        errs.append("string escaping vector mismatch")

    eb = v["event_body"]
    body = {"seq": eb["seq"], "type": eb["type"], "ts": eb["ts"],
            "actor": eb["actor"], "payload": eb["payload"],
            "prev_hash": eb["prev_hash"]}
    if canonical(body) != eb["canonical"]:
        errs.append("event body canonical vector mismatch")
    if event_hash(eb["prev_hash"], body) != eb["hash"]:
        errs.append("event hash vector mismatch")

    for raw, want in v["float_renderings"]:
        got = render_float(float(raw))
        if got != want:
            errs.append(f"float rendering {raw}: got {got}, want {want}")

    mr = v["merkle_roots"]
    aa, bb, cc = "a" * 64, "b" * 64, "c" * 64
    for name, got, want in [
        ("empty", merkle_root([]), mr["empty"]),
        ("one", merkle_root([aa]), mr["one_aa"]),
        ("two", merkle_root([aa, bb]), mr["two_aa_bb"]),
        ("three", merkle_root([aa, bb, cc]), mr["three_aa_bb_cc"]),
    ]:
        if got != want:
            errs.append(f"merkle {name}: got {got}, want {want}")

    si = v["snapshot_ids"]
    if snapshot_id(si["content"], "d" * 64) != si["with_parent_dddd"]:
        errs.append("snapshot id (with parent) vector mismatch")
    if snapshot_id(si["content"], None) != si["null_parent"]:
        errs.append("snapshot id (null parent) vector mismatch")
    return errs


def main(argv) -> int:
    failures = 0
    errs = check_vectors()
    if errs:
        failures += len(errs)
        for e in errs:
            print(f"VECTOR FAIL: {e}")
    else:
        print("vectors: all reproduced (strings, event body+hash, "
              "floats, merkle 0/1/2/3, snapshot ids)")

    paths = [Path(a) for a in argv] or [
        FIXTURES / "victim.axmr", FIXTURES / "canon.axmr"]
    for p in paths:
        try:
            n, checked = verify_ledger(p)
            print(f"OK   {p.name}: events={n} signatures_checked={checked}")
        except Fail as f:
            failures += 1
            print(f"FAIL {p.name}: reason={f.reason} seq={f.seq}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

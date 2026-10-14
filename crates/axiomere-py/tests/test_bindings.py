"""Bindings tests for Spec 003 (T314): FR-VP-04/07/11.

Run against a maturin-built `_axiomere` module, e.g.:

    python -m venv .venv && .venv/bin/pip install maturin
    cd crates/axiomere-py && VIRTUAL_ENV=$OLDPWD/.venv maturin develop
    .venv/bin/python -m pytest crates/axiomere-py/tests/test_bindings.py
    # or without pytest: .venv/bin/python crates/axiomere-py/tests/test_bindings.py

Covers: py_to_json value fidelity (non-finite floats and out-of-range
integers raise ValueError and append NOTHING), the full Verification
dict shape on verify()/verify_file(), trusted-map strictness and
expected-head pins through the bindings, and verify_file's no-raise
contract on malformed/unreadable files (while PyLedger.open raises).
"""

import math
import shutil
import sys
import tempfile
from pathlib import Path

import _axiomere

FIXTURES = (Path(__file__).resolve().parents[2]
            / "axiomere" / "tests" / "fixtures" / "redteam-2026-10-13")
VICTIM_PUB = "2d0fa96b6ed7c3d233dc3a12f455cf451eeeb6afab24fe5349394d55264764f1"
VICTIM_HEAD = "28170e747f1552d095436b2c2bf53b0f9df29535dfa861f574e0a8adf535f343"
VICTIM_ROOT = "e1858626f1fdf463d1ab80585679445acaefe30c46aec26e5367282abb998e6b"
FULL_SHAPE = {"ok", "events", "break_seq", "reason", "signatures_checked",
              "signatures_failed", "unsigned_events", "untrusted_actors"}


def _raises_valueerror(fn) -> bool:
    try:
        fn()
    except ValueError:
        return True
    return False


def test_nonfinite_and_out_of_range_rejected_atomically():
    with tempfile.TemporaryDirectory() as d:
        path = str(Path(d) / "t.axmr")
        db = _axiomere.PyLedger.create(path)
        db.append("note", {"x": 1}, "tester")
        before = Path(path).read_bytes()
        for bad in (math.nan, math.inf, -math.inf, 2**64, 2**70, -(2**63) - 1):
            assert _raises_valueerror(
                lambda bad=bad: db.append("note", {"x": bad}, "tester")), bad
            assert Path(path).read_bytes() == before, \
                f"rejected append of {bad!r} changed the ledger file"
        # The exact edges of the representable range DO append.
        db.append("note", {"x": 2**64 - 1}, "tester")   # u64::MAX
        db.append("note", {"x": -(2**63)}, "tester")   # i64::MIN
        db.append("note", {"x": 0.30000000000000004}, "tester")
        v = db.verify()
        assert v["ok"] and v["events"] == 4, v


def test_verify_file_full_shape_and_never_raises():
    v = _axiomere.verify_file(str(FIXTURES / "victim.axmr"))
    assert set(v.keys()) == FULL_SHAPE, v.keys()
    assert v["ok"] and v["events"] == 6 and v["unsigned_events"] == [], v

    v = _axiomere.verify_file(str(FIXTURES / "strip-sig.axmr"))
    assert not v["ok"] and v["reason"] == "unsigned-event", v

    v = _axiomere.verify_file(str(FIXTURES / "file-bom.axmr"))
    assert not v["ok"] and v["reason"] == "bad-magic", v

    v = _axiomere.verify_file("/nonexistent/definitely-missing.axmr")
    assert not v["ok"] and v["reason"] == "unreadable-file", v

    # ...while opening the same malformed file for appends still raises.
    try:
        _axiomere.PyLedger.open(str(FIXTURES / "file-bom.axmr"))
        raise AssertionError("PyLedger.open should refuse a malformed file")
    except Exception:
        pass


def test_trusted_and_expected_head_through_bindings():
    with tempfile.TemporaryDirectory() as d:
        path = str(Path(d) / "victim.axmr")
        shutil.copy(FIXTURES / "victim.axmr", path)
        db = _axiomere.PyLedger.open(path)

        v = db.verify()
        assert set(v.keys()) == FULL_SHAPE and v["ok"], v

        good_map = {"researcher": VICTIM_PUB, "adjudicator": VICTIM_PUB}
        v = db.verify(trusted=good_map)
        assert v["ok"], v

        v = db.verify(trusted={})
        assert not v["ok"] and v["reason"] == "untrusted-actor", v
        assert set(v["untrusted_actors"]) == {"researcher"}, v

        mallory = str(FIXTURES / "atk-as-mallory.axmr")
        v = _axiomere.verify_file(mallory, trusted=good_map)
        assert not v["ok"] and v["reason"] == "untrusted-actor", v
        assert "mallory" in v["untrusted_actors"], v

        v = db.verify(expected_head={"head_hash": VICTIM_HEAD,
                                     "min_events": 6,
                                     "checkpoint_root": VICTIM_ROOT})
        assert v["ok"], v
        v = db.verify(expected_head={"head_hash": "0" * 64})
        assert not v["ok"] and v["reason"] == "head-mismatch", v
        v = db.verify(expected_head={"min_events": 7})
        assert not v["ok"] and v["reason"] == "shortfall", v
        v = db.verify(expected_head={"checkpoint_root": "0" * 64})
        assert not v["ok"] and v["reason"] == "root-mismatch", v

        # Truncation: bare verify passes, the pin catches it (FR-VP-10).
        trunc = str(FIXTURES / "surg-truncate.axmr")
        v = _axiomere.verify_file(trunc)
        assert v["ok"], v
        v = _axiomere.verify_file(trunc, expected_head={
            "head_hash": VICTIM_HEAD, "min_events": 6})
        assert not v["ok"] and v["reason"] == "shortfall", v


if __name__ == "__main__":
    fns = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for fn in fns:
        fn()
        print(f"PASS {fn.__name__}")
    print(f"{len(fns)} bindings tests passed")
    sys.exit(0)

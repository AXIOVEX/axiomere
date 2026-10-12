//! PyO3 bindings for Axiomere. The Python-facing module is `_axiomere`;
//! the idiomatic Python API lives in the separate `axiomere-python`
//! repository, which wraps this extension.

use axiomere::Ledger;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde_json::Value;
use std::collections::BTreeMap;

fn to_py_err(e: axiomere::Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

fn json_to_py(py: Python<'_>, v: &Value) -> PyObject {
    match v {
        Value::Null => py.None(),
        Value::Bool(b) => b.into_py(py),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_py(py)
            } else if let Some(u) = n.as_u64() {
                u.into_py(py)
            } else {
                n.as_f64().unwrap_or(0.0).into_py(py)
            }
        }
        Value::String(s) => s.clone().into_py(py),
        Value::Array(items) => {
            let list = pyo3::types::PyList::empty_bound(py);
            for item in items {
                list.append(json_to_py(py, item)).unwrap();
            }
            list.into_py(py)
        }
        Value::Object(map) => {
            let dict = pyo3::types::PyDict::new_bound(py);
            for (k, val) in map {
                dict.set_item(k, json_to_py(py, val)).unwrap();
            }
            dict.into_py(py)
        }
    }
}

fn py_to_json(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(Value::Bool(b));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(Value::Number(i.into()));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(serde_json::Number::from_f64(f).map(Value::Number).unwrap_or(Value::Null));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(Value::String(s));
    }
    if let Ok(list) = obj.extract::<Vec<Bound<'_, PyAny>>>() {
        let mut out = Vec::new();
        for item in &list {
            out.push(py_to_json(item)?);
        }
        return Ok(Value::Array(out));
    }
    if let Ok(dict) = obj.extract::<BTreeMap<String, Bound<'_, PyAny>>>() {
        let mut map = serde_json::Map::new();
        for (k, v) in &dict {
            map.insert(k.clone(), py_to_json(v)?);
        }
        return Ok(Value::Object(map));
    }
    Err(PyValueError::new_err("unsupported value type for Axiomere payload"))
}

#[pyclass]
struct PyLedger {
    inner: Ledger,
}

#[pymethods]
impl PyLedger {
    #[staticmethod]
    fn create(path: &str) -> PyResult<Self> {
        Ok(PyLedger {
            inner: Ledger::create(path).map_err(to_py_err)?,
        })
    }

    #[staticmethod]
    fn open(path: &str) -> PyResult<Self> {
        Ok(PyLedger {
            inner: Ledger::open(path).map_err(to_py_err)?,
        })
    }

    fn append(&mut self, event_type: &str, payload: &Bound<'_, PyAny>, actor: &str) -> PyResult<String> {
        let ev = self
            .inner
            .append(event_type, py_to_json(payload)?, actor)
            .map_err(to_py_err)?;
        Ok(serde_json::to_string(&ev).unwrap())
    }

    #[pyo3(signature = (record_id, content, actor, status="live", score=None, reason=None))]
    fn snapshot(
        &mut self,
        record_id: &str,
        content: &Bound<'_, PyAny>,
        actor: &str,
        status: &str,
        score: Option<f64>,
        reason: Option<&str>,
    ) -> PyResult<String> {
        let ev = self
            .inner
            .append_snapshot(record_id, py_to_json(content)?, status, score, reason, actor)
            .map_err(to_py_err)?;
        Ok(serde_json::to_string(&ev).unwrap())
    }

    fn mark_dead(&mut self, record_id: &str, reason: &str, actor: &str) -> PyResult<String> {
        let ev = self.inner.mark_dead(record_id, reason, actor).map_err(to_py_err)?;
        Ok(serde_json::to_string(&ev).unwrap())
    }

    fn checkpoint(&mut self, actor: &str) -> PyResult<String> {
        let ev = self.inner.checkpoint(actor).map_err(to_py_err)?;
        Ok(serde_json::to_string(&ev).unwrap())
    }

    fn verify(&self, py: Python<'_>) -> PyResult<PyObject> {
        let v = self.inner.verify().map_err(to_py_err)?;
        let dict = pyo3::types::PyDict::new_bound(py);
        dict.set_item("ok", v.ok)?;
        dict.set_item("events", v.events)?;
        dict.set_item("break_seq", v.break_seq)?;
        dict.set_item("reason", v.reason)?;
        dict.set_item("signatures_checked", v.signatures_checked)?;
        dict.set_item("signatures_failed", v.signatures_failed)?;
        Ok(dict.into_py(py))
    }

    #[pyo3(signature = (include_dead=false))]
    fn current_view(&self, py: Python<'_>, include_dead: bool) -> PyResult<PyObject> {
        let view = self.inner.current_view(include_dead).map_err(to_py_err)?;
        let dict = pyo3::types::PyDict::new_bound(py);
        for (k, v) in &view {
            dict.set_item(k, json_to_py(py, v))?;
        }
        Ok(dict.into_py(py))
    }

    fn events(&self, py: Python<'_>) -> PyResult<PyObject> {
        let events = self.inner.events().map_err(to_py_err)?;
        let list = pyo3::types::PyList::empty_bound(py);
        for ev in &events {
            list.append(json_to_py(py, &serde_json::to_value(ev).unwrap()))?;
        }
        Ok(list.into_py(py))
    }
}

fn hex_decode(s: &str) -> PyResult<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| PyValueError::new_err("bad hex")))
        .collect()
}

#[pyfunction]
fn open_with_seed(path: &str, seed_hex: &str) -> PyResult<PyLedger> {
    let bytes = hex_decode(seed_hex)?;
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| PyValueError::new_err("seed must be 32 bytes (64 hex chars)"))?;
    Ok(PyLedger {
        inner: Ledger::open(path).map_err(to_py_err)?.with_signer(&seed),
    })
}

#[pyfunction]
fn create_with_seed(path: &str, seed_hex: &str) -> PyResult<PyLedger> {
    let bytes = hex_decode(seed_hex)?;
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| PyValueError::new_err("seed must be 32 bytes (64 hex chars)"))?;
    Ok(PyLedger {
        inner: Ledger::create(path).map_err(to_py_err)?.with_signer(&seed),
    })
}

#[pyfunction]
fn generate_keypair() -> (String, String) {
    axiomere::generate_keypair()
}

#[pyfunction]
fn verify_file(path: &str) -> PyResult<(bool, usize, Option<String>)> {
    let events = axiomere::read_events(path).map_err(to_py_err)?;
    let v = axiomere::verify_events(&events);
    Ok((v.ok, v.events, v.reason))
}

#[pymodule]
fn _axiomere(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLedger>()?;
    m.add_function(wrap_pyfunction!(open_with_seed, m)?)?;
    m.add_function(wrap_pyfunction!(create_with_seed, m)?)?;
    m.add_function(wrap_pyfunction!(generate_keypair, m)?)?;
    m.add_function(wrap_pyfunction!(verify_file, m)?)?;
    Ok(())
}

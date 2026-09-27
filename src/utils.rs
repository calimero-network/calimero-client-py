//! Utility functions for JSON to Python conversion

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// Convert serde_json::Value to Python object
pub fn json_to_python(py: Python, value: &serde_json::Value) -> PyObject {
    match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => b.into_py(py),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_py(py)
            } else if let Some(f) = n.as_f64() {
                f.into_py(py)
            } else {
                n.to_string().into_py(py)
            }
        }
        serde_json::Value::String(s) => s.into_py(py),
        serde_json::Value::Array(arr) => {
            let list = PyList::new_bound(py, Vec::<PyObject>::new());
            for item in arr {
                list.append(json_to_python(py, item)).unwrap();
            }
            list.into_py(py)
        }
        serde_json::Value::Object(obj) => {
            let dict = PyDict::new_bound(py);
            for (k, v) in obj {
                dict.set_item(k, json_to_python(py, v)).unwrap();
            }
            dict.into_py(py)
        }
    }
}

/// Parse intent arguments into the one form both the warrant and the request
/// carry: every object's keys sorted, at every depth.
///
/// The warrant commits to `H(method ‖ args)` and the node recomputes it from
/// the `argsJson` it receives, so the bytes this crate hashes and the bytes it
/// sends must be the same, whatever key order the caller wrote. They used to
/// be sorted by `serde_json` itself, but only while no crate in the build turns
/// on its `preserve_order` feature: `dcap-qvl` (quote verification) does, and
/// features unify, so the order would have become whatever the caller typed.
/// Sorting here keeps it independent of what else is linked in.
pub fn canonical_args(args: &str) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(args).map(sort_keys)
}

fn sort_keys(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => {
            let mut entries: Vec<_> = object.into_iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.cmp(b));
            serde_json::Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, sort_keys(value)))
                    .collect(),
            )
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(sort_keys).collect())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::canonical_args;

    #[test]
    fn keys_are_sorted_at_every_depth() {
        let args = canonical_args(r#"{"b":{"y":1,"x":[{"d":0,"c":0}]},"a":2}"#).unwrap();
        assert_eq!(
            serde_json::to_string(&args).unwrap(),
            r#"{"a":2,"b":{"x":[{"c":0,"d":0}],"y":1}}"#
        );
    }
}

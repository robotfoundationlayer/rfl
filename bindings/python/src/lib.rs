// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Minimal Python binding for the RFL retarget engine.
//!
//! Exposes one function, [`retarget`], wrapping the stable rfl-core pipeline
//! (`Skill::parse_yaml` -> `Embodiment::parse_yaml` -> `translation::retarget` ->
//! `canonical::to_jsonl`) — identical to `rfl-conformance::retarget_example_to_jsonl`
//! and `rfl-cli`'s `retarget` subcommand, but string-in / string-out. Because it
//! exchanges only strings it does not churn as primitive coverage or the canonical
//! JSON format evolve.

use pyo3::prelude::*;

use rfl_core::canonical;
use rfl_core::embodiment::Embodiment;
use rfl_core::skill_isa::Skill;
use rfl_core::translation;

pyo3::create_exception!(rfl, RetargetError, pyo3::exceptions::PyException);

/// Map an rfl-core error to the Python `rfl.RetargetError`.
fn to_py_err(e: rfl_core::Error) -> PyErr {
    RetargetError::new_err(e.to_string())
}

/// Retarget a Skill ISA composition onto an embodiment descriptor.
///
/// Both inputs are YAML text; the return value is the canonical-action JSONL
/// stream (one `execute` message per line). Parse or retarget failures raise
/// `rfl.RetargetError`.
#[pyfunction]
fn retarget(skill_yaml: &str, descriptor_yaml: &str) -> PyResult<String> {
    let skill = Skill::parse_yaml(skill_yaml).map_err(to_py_err)?;
    let emb = Embodiment::parse_yaml(descriptor_yaml).map_err(to_py_err)?;
    let out = translation::retarget(&skill, &emb).map_err(to_py_err)?;
    Ok(canonical::to_jsonl(
        &skill.skill,
        &emb.id,
        &out.actions,
        &out.suffixes,
    ))
}

/// The `rfl` Python module.
#[pymodule]
fn rfl(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(retarget, m)?)?;
    m.add("RetargetError", m.py().get_type::<RetargetError>())?;
    Ok(())
}

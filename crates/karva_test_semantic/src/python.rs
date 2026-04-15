//! Registration hub for the `PyO3` surface exposed to Python as `karva._karva`.
//!
//! Each extension family owns its own Python-facing file:
//! - [`crate::extensions::tags::python`] — the `@karva.tags.*` decorator API
//! - [`crate::extensions::fixtures::python`] — the `@karva.fixture` decorator and companion types
//! - [`crate::extensions::functions::python`] — top-level `karva.{skip, fail, param}` plus their exceptions
//! - [`crate::extensions::functions::raises`] and [`crate::extensions::functions::snapshot`]
//!   — cohesive `PyO3` + Rust modules that stay together because their classes
//!   are tightly coupled to private state in the same file
//!
//! [`init_module`] below is the single place every `#[pyclass]`,
//! `#[pyfunction]`, and `create_exception!` gets handed to the interpreter —
//! grep here first when adding or removing a binding.

use pyo3::prelude::*;
use pyo3::wrap_pymodule;

use crate::extensions::fixtures::python::{
    FixtureFunctionDefinition, FixtureFunctionMarker, InvalidFixtureError, fixture_decorator,
};
use crate::extensions::functions::raises::raises;
use crate::extensions::functions::snapshot::{
    assert_cmd_snapshot, assert_json_snapshot, assert_snapshot, snapshot_settings,
};
use crate::extensions::functions::{
    Command, ExceptionInfo, FailError, RaisesContext, SkipError, SnapshotMismatchError,
    SnapshotSettings, fail, param, skip,
};
use crate::extensions::tags::python::{PyTags, PyTestFunction, tags};

pub fn init_module(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fixture_decorator, m)?)?;
    m.add_function(wrap_pyfunction!(skip, m)?)?;
    m.add_function(wrap_pyfunction!(fail, m)?)?;
    m.add_function(wrap_pyfunction!(param, m)?)?;
    m.add_function(wrap_pyfunction!(raises, m)?)?;
    m.add_function(wrap_pyfunction!(assert_snapshot, m)?)?;
    m.add_function(wrap_pyfunction!(assert_json_snapshot, m)?)?;
    m.add_function(wrap_pyfunction!(assert_cmd_snapshot, m)?)?;
    m.add_function(wrap_pyfunction!(snapshot_settings, m)?)?;

    m.add_class::<FixtureFunctionMarker>()?;
    m.add_class::<FixtureFunctionDefinition>()?;
    m.add_class::<PyTags>()?;
    m.add_class::<PyTestFunction>()?;
    m.add_class::<ExceptionInfo>()?;
    m.add_class::<RaisesContext>()?;
    m.add_class::<SnapshotSettings>()?;
    m.add_class::<Command>()?;

    m.add_wrapped(wrap_pymodule!(tags))?;

    m.add("SkipError", py.get_type::<SkipError>())?;
    m.add("FailError", py.get_type::<FailError>())?;
    m.add("InvalidFixtureError", py.get_type::<InvalidFixtureError>())?;
    m.add(
        "SnapshotMismatchError",
        py.get_type::<SnapshotMismatchError>(),
    )?;
    Ok(())
}

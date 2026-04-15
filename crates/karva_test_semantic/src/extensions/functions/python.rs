//! `PyO3` bindings for the top-level `karva.*` test API — runtime test-control
//! functions (`skip`, `fail`, `param`), the value types they produce, and the
//! exception classes the runner understands.
//!
//! The more involved `PyO3` types for this module live next to their Rust
//! helpers in [`super::raises`] and [`super::snapshot`], where splitting them
//! would only add `pub(super)` noise.

use std::sync::Arc;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

use crate::extensions::tags::parametrize::Parametrization;
use crate::extensions::tags::{Tag, Tags};

// SkipError exception that can be raised to skip tests at runtime with an optional reason
pyo3::create_exception!(karva, SkipError, pyo3::exceptions::PyException);

// FailError exception that can be raised to fail tests at runtime with an optional reason
pyo3::create_exception!(karva, FailError, pyo3::exceptions::PyException);

#[derive(Debug, Clone)]
#[pyclass(from_py_object)]
pub struct Param {
    /// The values of the arguments
    pub(crate) values: Vec<Arc<Py<PyAny>>>,

    /// Tags associated with this parametrization
    pub(crate) tags: Tags,
}

impl Param {
    pub(crate) fn new(py: Python, values: Vec<Py<PyAny>>, tags: Vec<Py<PyAny>>) -> PyResult<Self> {
        let mut new_tags = Vec::new();

        for tag in tags {
            new_tags.push(
                Tag::try_from_py_any(py, &tag)
                    .ok_or_else(|| PyTypeError::new_err("Invalid tag"))?,
            );
        }

        Ok(Self {
            values: values.into_iter().map(Arc::new).collect(),
            tags: Tags::new(new_tags),
        })
    }

    pub(crate) fn from_parametrization(Parametrization { values, tags }: Parametrization) -> Self {
        Self { values, tags }
    }
}

/// Skip the current test at runtime with an optional reason.
///
/// This function raises a `SkipError` exception which will be caught by the test runner
/// and mark the test as skipped.
#[pyfunction]
#[pyo3(signature = (reason = None))]
pub fn skip(_py: Python<'_>, reason: Option<String>) -> PyResult<()> {
    let message = reason.unwrap_or_default();
    Err(SkipError::new_err(message))
}

/// Fail the current test at runtime with an optional reason.
///
/// This function raises a `FailError` exception which will be caught by the test runner
/// and mark the test as failed with the given reason.
#[pyfunction]
#[pyo3(signature = (reason = None))]
pub fn fail(_py: Python<'_>, reason: Option<String>) -> PyResult<()> {
    Err(FailError::new_err(reason))
}

#[pyfunction]
#[pyo3(signature = (*values, tags = None))]
pub fn param(
    py: Python<'_>,
    values: Vec<Py<PyAny>>,
    tags: Option<Vec<Py<PyAny>>>,
) -> PyResult<Param> {
    Param::new(py, values, tags.unwrap_or_default())
}

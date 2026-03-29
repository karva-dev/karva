use pyo3::exceptions::{PyAssertionError, PyRuntimeError};
use pyo3::prelude::*;
use pyo3::types::PyList;

pub fn is_recwarn_fixture_name(fixture_name: &str) -> bool {
    fixture_name == "recwarn"
}

pub fn create_recwarn_fixture(py: Python<'_>) -> Option<(Py<PyAny>, Py<PyAny>)> {
    let recorder = Py::new(py, WarningsChecker::new(py).ok()?).ok()?;
    let finalizer = recorder.getattr(py, "_teardown").ok()?;
    Some((recorder.into_any(), finalizer))
}

/// Fixture that records Python warnings raised during a test.
///
/// Installs `warnings.catch_warnings(record=True)` and `warnings.simplefilter("always")`
/// for the lifetime of the test, then restores the original filter state on teardown.
#[pyclass]
pub struct WarningsChecker {
    /// The list returned by `catch_warnings(record=True).__enter__()`.
    warnings_list: Py<PyList>,
    /// The active `catch_warnings` context manager instance.
    catch_warnings_ctx: Py<PyAny>,
}

impl WarningsChecker {
    fn new(py: Python<'_>) -> PyResult<Self> {
        let warnings = py.import("warnings")?;

        let kwargs = pyo3::types::PyDict::new(py);
        kwargs.set_item("record", true)?;
        let ctx = warnings
            .getattr("catch_warnings")?
            .call((), Some(&kwargs))?;

        let warnings_list: Py<PyList> = ctx.call_method0("__enter__")?.extract()?;

        warnings.call_method1("simplefilter", ("always",))?;

        Ok(Self {
            warnings_list,
            catch_warnings_ctx: ctx.unbind(),
        })
    }
}

#[pymethods]
impl WarningsChecker {
    /// Return a string representation of the `WarningsChecker` object.
    #[expect(clippy::unused_self)]
    fn __repr__(&self) -> &'static str {
        "<WarningsChecker object>"
    }

    /// Return the list of captured `warnings.WarningMessage` objects.
    #[getter]
    fn list(&self, py: Python<'_>) -> Py<PyList> {
        self.warnings_list.clone_ref(py)
    }

    /// Return the number of captured warnings.
    fn __len__(&self, py: Python<'_>) -> usize {
        self.warnings_list.bind(py).len()
    }

    /// Return the warning at the given index.
    fn __getitem__(&self, py: Python<'_>, index: isize) -> PyResult<Py<PyAny>> {
        Ok(self
            .warnings_list
            .bind(py)
            .call_method1("__getitem__", (index,))?
            .unbind())
    }

    /// Iterate over the captured warnings.
    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(self
            .warnings_list
            .bind(py)
            .call_method0("__iter__")?
            .unbind())
    }

    /// Remove and return the first warning whose category is a subclass of `category`.
    ///
    /// Raises `AssertionError` if no matching warning is found.
    #[pyo3(signature = (category = None))]
    fn pop(&self, py: Python<'_>, category: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        let list = self.warnings_list.bind(py);

        let builtins = py.import("builtins")?;

        let category: Py<PyAny> = if let Some(c) = category {
            c
        } else {
            builtins.getattr("Warning")?.unbind()
        };

        for i in 0..list.len() {
            let entry = list.get_item(i)?;
            let warn_category = entry.getattr("category")?;
            let is_match = builtins
                .call_method1("issubclass", (warn_category, &category))?
                .extract::<bool>()?;

            if is_match {
                list.call_method1("pop", (i,))?;
                return Ok(entry.unbind());
            }
        }

        let category_name = category
            .bind(py)
            .getattr("__name__")
            .map(|n| n.to_string())
            .unwrap_or_else(|_| format!("{category:?}"));

        Err(PyAssertionError::new_err(format!(
            "No warnings of type {category_name} were emitted."
        )))
    }

    /// Clear all captured warnings.
    fn clear(&self, py: Python<'_>) -> PyResult<()> {
        self.warnings_list.bind(py).call_method0("clear")?;
        Ok(())
    }

    /// Exit the `catch_warnings` context, restoring the original warning filters.
    fn _teardown(&self, py: Python<'_>) -> PyResult<()> {
        self.catch_warnings_ctx
            .call_method1(py, "__exit__", (py.None(), py.None(), py.None()))
            .map_err(|e| PyRuntimeError::new_err(format!("recwarn teardown failed: {e}")))?;
        Ok(())
    }
}

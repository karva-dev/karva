use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};

pub fn is_caplog_fixture_name(fixture_name: &str) -> bool {
    matches!(fixture_name, "caplog")
}

pub fn create_caplog_fixture(py: Python<'_>) -> Option<(Py<PyAny>, Py<PyAny>)> {
    let caplog = Py::new(py, CapLog::new(py).ok()?).ok()?;
    let teardown_method = caplog.getattr(py, "_teardown").ok()?;
    Some((caplog.into_any(), teardown_method))
}

/// The Python code defining a `logging.Handler` subclass that appends records to a list.
///
/// The handler receives the shared `records` list from Rust and stores emitted
/// `LogRecord` objects in it.
const HANDLER_CODE: &str = r"
import logging

class _CapLogHandler(logging.Handler):
    def __init__(self, records):
        super().__init__(0)
        self._records = records

    def emit(self, record):
        record.message = record.getMessage()
        self._records.append(record)
";

/// Built-in `caplog` fixture for capturing Python log records during a test.
///
/// Provides access to log records emitted during the test via `.records`, `.text`,
/// and `.messages`. Use `.at_level()` as a context manager or `.set_level()` to
/// control which records are captured.
#[pyclass]
pub struct CapLog {
    /// The Python `_CapLogHandler` instance installed on the root logger.
    handler: Py<PyAny>,
    /// Shared Python list of captured `LogRecord` objects.
    records: Py<PyList>,
    /// The `logging.disable` level saved at construction time, restored at teardown.
    saved_disable_level: Option<i32>,
    /// The logger level saved by `set_level()`, restored at teardown.
    saved_level: Option<i32>,
    /// The logger name passed to `set_level()`, used to restore its level at teardown.
    saved_level_logger: Option<String>,
}

impl CapLog {
    /// Create the Python handler and install it on the root logger.
    fn new(py: Python<'_>) -> PyResult<Self> {
        let locals = PyDict::new(py);
        py.run(
            &std::ffi::CString::new(HANDLER_CODE)
                .map_err(|e| PyRuntimeError::new_err(format!("CString error: {e}")))?,
            None,
            Some(&locals),
        )?;

        let handler_class = locals
            .get_item("_CapLogHandler")?
            .ok_or_else(|| PyRuntimeError::new_err("_CapLogHandler not found"))?;

        let records = PyList::empty(py);
        let handler = handler_class.call1((records.clone(),))?;

        let logging = py.import("logging")?;
        let root_logger = logging.call_method0("getLogger")?;
        root_logger.call_method1("addHandler", (&handler,))?;

        // Save the current global disable level and re-enable logging so the
        // handler receives records during the test.
        let saved_disable_level = logging
            .getattr("root")?
            .getattr("manager")?
            .getattr("disable")
            .and_then(|v| v.extract::<i32>())
            .ok();

        let notset = logging.getattr("NOTSET")?;
        logging.call_method1("disable", (notset,))?;

        Ok(Self {
            handler: handler.unbind(),
            records: records.unbind(),
            saved_disable_level,
            saved_level: None,
            saved_level_logger: None,
        })
    }
}

#[pymethods]
impl CapLog {
    #[getter]
    fn records<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        self.records.bind(py).clone()
    }

    #[getter]
    fn handler<'py>(&self, py: Python<'py>) -> Bound<'py, PyAny> {
        self.handler.bind(py).clone()
    }

    #[getter]
    fn record_tuples<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let records = self.records.bind(py);
        let tuples: Vec<_> = records
            .iter()
            .map(|r| {
                let name = r.getattr("name")?;
                let levelno = r.getattr("levelno")?;
                let message = r.call_method0("getMessage")?;
                PyTuple::new(py, [name, levelno, message])
            })
            .collect::<PyResult<_>>()?;
        PyList::new(py, tuples)
    }

    #[getter]
    fn messages<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let records = self.records.bind(py);
        let messages: Vec<_> = records
            .iter()
            .map(|r| r.call_method0("getMessage"))
            .collect::<PyResult<_>>()?;
        PyList::new(py, messages)
    }

    #[getter]
    fn text(&self, py: Python<'_>) -> PyResult<String> {
        let logging = py.import("logging")?;
        let formatter = logging.getattr("Formatter")?.call0()?;

        let records = self.records.bind(py);
        let mut parts = Vec::with_capacity(records.len());
        for record in records.iter() {
            let formatted = formatter.call_method1("format", (&record,))?;
            parts.push(formatted.extract::<String>()?);
        }
        Ok(parts.join("\n"))
    }

    /// Set the capture level for the remainder of the test (no context manager).
    ///
    /// The original level is saved on first call and restored by `_teardown`.
    #[pyo3(signature = (level, logger = None))]
    fn set_level(
        mut slf: PyRefMut<'_, Self>,
        py: Python<'_>,
        level: i32,
        logger: Option<&str>,
    ) -> PyResult<()> {
        let logging = py.import("logging")?;
        let target = if let Some(name) = logger {
            logging.call_method1("getLogger", (name,))?
        } else {
            logging.call_method0("getLogger")?
        };

        // Save the original level on first call so `_teardown` can restore it.
        if slf.saved_level.is_none() {
            slf.saved_level = Some(target.getattr("level")?.extract::<i32>()?);
            slf.saved_level_logger = logger.map(str::to_owned);
        }

        target.call_method1("setLevel", (level,))?;
        slf.handler.bind(py).call_method1("setLevel", (level,))?;
        Ok(())
    }

    /// Context manager that temporarily sets the capture level.
    #[pyo3(signature = (level, logger = None))]
    fn at_level(&self, py: Python<'_>, level: i32, logger: Option<String>) -> PyResult<Py<PyAny>> {
        let prev_handler_level = self.handler.bind(py).getattr("level")?.extract::<i32>()?;

        let context = CapLogLevelContext {
            handler: self.handler.clone_ref(py),
            level,
            logger_name: logger,
            prev_handler_level,
            prev_logger_level: None,
        };

        Ok(Py::new(py, context)?.into_any())
    }

    /// Clear all captured records.
    fn clear(&self, py: Python<'_>) {
        self.records.bind(py).call_method0("clear").ok();
    }

    /// Remove the handler from the root logger and restore the saved disable level and any
    /// logger level changed by `set_level()`.
    fn _teardown(&self, py: Python<'_>) -> PyResult<()> {
        let logging = py.import("logging")?;
        let root_logger = logging.call_method0("getLogger")?;
        root_logger.call_method1("removeHandler", (self.handler.bind(py),))?;

        let restore_level = self.saved_disable_level.unwrap_or(0);
        logging.call_method1("disable", (restore_level,))?;

        if let Some(prev) = self.saved_level {
            let target = if let Some(ref name) = self.saved_level_logger {
                logging.call_method1("getLogger", (name.as_str(),))?
            } else {
                logging.call_method0("getLogger")?
            };
            target.call_method1("setLevel", (prev,))?;
        }

        Ok(())
    }

    #[expect(clippy::unused_self)]
    fn __repr__(&self) -> String {
        "<CapLog object>".to_string()
    }
}

/// Context manager returned by `caplog.at_level(level, logger=...)`.
///
/// On entry, sets the handler and optionally a named logger to `level`.
/// On exit, restores the previous levels.
#[pyclass]
struct CapLogLevelContext {
    handler: Py<PyAny>,
    level: i32,
    logger_name: Option<String>,
    prev_handler_level: i32,
    prev_logger_level: Option<i32>,
}

#[pymethods]
impl CapLogLevelContext {
    fn __enter__(mut slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        let py = slf.py();
        let logging = py.import("logging")?;
        let target = if let Some(ref name) = slf.logger_name {
            logging.call_method1("getLogger", (name.as_str(),))?
        } else {
            logging.call_method0("getLogger")?
        };

        slf.prev_logger_level = Some(target.getattr("level")?.extract::<i32>()?);

        target.call_method1("setLevel", (slf.level,))?;
        slf.handler
            .bind(py)
            .call_method1("setLevel", (slf.level,))?;

        Ok(slf)
    }

    fn __exit__(
        &mut self,
        py: Python<'_>,
        _exc_type: Py<PyAny>,
        _exc_val: Py<PyAny>,
        _exc_tb: Py<PyAny>,
    ) -> PyResult<bool> {
        let logging = py.import("logging")?;
        let target = if let Some(ref name) = self.logger_name {
            logging.call_method1("getLogger", (name.as_str(),))?
        } else {
            logging.call_method0("getLogger")?
        };

        if let Some(prev) = self.prev_logger_level {
            target.call_method1("setLevel", (prev,))?;
        }
        self.handler
            .bind(py)
            .call_method1("setLevel", (self.prev_handler_level,))?;

        Ok(false)
    }
}

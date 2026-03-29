use pyo3::prelude::*;

pub fn is_capsys_fixture_name(fixture_name: &str) -> bool {
    matches!(fixture_name, "capsys")
}

/// `capfd` captures output at the file-descriptor level. This implementation
/// uses stream-level capture (same as `capsys`), which is sufficient for Python
/// code writing to `sys.stdout`/`sys.stderr`. True fd-level capture via
/// `os.dup2` is not implemented.
pub fn is_capfd_fixture_name(fixture_name: &str) -> bool {
    matches!(fixture_name, "capfd")
}

pub fn create_capsys_fixture(py: Python<'_>) -> Option<(Py<PyAny>, Py<PyAny>)> {
    let capsys = Py::new(py, CapsysFixture::new(py).ok()?).ok()?;
    let restore_method = capsys.getattr(py, "_restore").ok()?;
    Some((capsys.into_any(), restore_method))
}

pub fn create_capfd_fixture(py: Python<'_>) -> Option<(Py<PyAny>, Py<PyAny>)> {
    create_capsys_fixture(py)
}

/// Captures writes to `sys.stdout` and `sys.stderr` during a test.
///
/// Provides `readouterr()` to retrieve and reset the captured output, and
/// `disabled()` as a context manager to temporarily restore real I/O.
#[pyclass]
pub struct CapsysFixture {
    /// The real `sys.stdout` saved at fixture creation time.
    real_stdout: Py<PyAny>,
    /// The real `sys.stderr` saved at fixture creation time.
    real_stderr: Py<PyAny>,
    /// The `io.StringIO` buffer currently installed as `sys.stdout`.
    capture_stdout: Py<PyAny>,
    /// The `io.StringIO` buffer currently installed as `sys.stderr`.
    capture_stderr: Py<PyAny>,
    /// The `CaptureResult` namedtuple class, created once and reused across `readouterr()` calls
    /// so that consecutive instances satisfy `isinstance` checks against each other.
    capture_result_class: Py<PyAny>,
}

impl CapsysFixture {
    fn new(py: Python<'_>) -> PyResult<Self> {
        let sys = py.import("sys")?;
        let io = py.import("io")?;

        let real_stdout = sys.getattr("stdout")?.unbind();
        let real_stderr = sys.getattr("stderr")?.unbind();

        let capture_stdout = io.call_method0("StringIO")?.unbind();
        let capture_stderr = io.call_method0("StringIO")?.unbind();

        sys.setattr("stdout", &capture_stdout)?;
        sys.setattr("stderr", &capture_stderr)?;

        let capture_result_class = py
            .import("collections")?
            .call_method1("namedtuple", ("CaptureResult", ["out", "err"]))?
            .unbind();

        Ok(Self {
            real_stdout,
            real_stderr,
            capture_stdout,
            capture_stderr,
            capture_result_class,
        })
    }

    fn fresh_stringio(py: Python<'_>) -> PyResult<Py<PyAny>> {
        Ok(py.import("io")?.call_method0("StringIO")?.unbind())
    }
}

#[pymethods]
impl CapsysFixture {
    /// Return a string representation of the `CapsysFixture` object.
    #[expect(clippy::unused_self)]
    fn __repr__(&self) -> &'static str {
        "<CapsysFixture object>"
    }

    /// Return a `CaptureResult(out, err)` namedtuple with the captured output and reset
    /// the buffers.
    fn readouterr(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let out = self
            .capture_stdout
            .bind(py)
            .call_method0("getvalue")?
            .extract::<String>()?;
        let err = self
            .capture_stderr
            .bind(py)
            .call_method0("getvalue")?
            .extract::<String>()?;

        // Reset both buffers to fresh StringIO instances.
        let new_stdout = Self::fresh_stringio(py)?;
        let new_stderr = Self::fresh_stringio(py)?;

        let sys = py.import("sys")?;
        sys.setattr("stdout", &new_stdout)?;
        sys.setattr("stderr", &new_stderr)?;

        self.capture_stdout = new_stdout;
        self.capture_stderr = new_stderr;

        Ok(self
            .capture_result_class
            .bind(py)
            .call1((out, err))?
            .unbind())
    }

    /// Return a context manager that temporarily disables capture (restores real stdout/stderr).
    fn disabled(&self, py: Python<'_>) -> CapsysDisabled {
        CapsysDisabled {
            real_stdout: self.real_stdout.clone_ref(py),
            real_stderr: self.real_stderr.clone_ref(py),
            capture_stdout: self.capture_stdout.clone_ref(py),
            capture_stderr: self.capture_stderr.clone_ref(py),
        }
    }

    /// Restore `sys.stdout` and `sys.stderr` to the real streams. Called as the fixture finalizer.
    fn _restore(&self, py: Python<'_>) -> PyResult<()> {
        let sys = py.import("sys")?;
        sys.setattr("stdout", &self.real_stdout)?;
        sys.setattr("stderr", &self.real_stderr)?;
        Ok(())
    }
}

/// Context manager returned by `capsys.disabled()` that temporarily restores real I/O.
#[pyclass]
struct CapsysDisabled {
    real_stdout: Py<PyAny>,
    real_stderr: Py<PyAny>,
    capture_stdout: Py<PyAny>,
    capture_stderr: Py<PyAny>,
}

#[pymethods]
impl CapsysDisabled {
    fn __enter__(slf: PyRef<'_, Self>) -> PyResult<PyRef<'_, Self>> {
        let py = slf.py();
        let sys = py.import("sys")?;
        sys.setattr("stdout", &slf.real_stdout)?;
        sys.setattr("stderr", &slf.real_stderr)?;
        Ok(slf)
    }

    fn __exit__(
        &self,
        py: Python<'_>,
        _exc_type: Py<PyAny>,
        _exc_val: Py<PyAny>,
        _exc_tb: Py<PyAny>,
    ) -> PyResult<bool> {
        let sys = py.import("sys")?;
        sys.setattr("stdout", &self.capture_stdout)?;
        sys.setattr("stderr", &self.capture_stderr)?;
        Ok(false)
    }
}

use pyo3::prelude::*;
use pyo3::types::PyTuple;

/// Represents a per-test timeout limit.
///
/// When a test exceeds the timeout, it is reported as a failure. The test is
/// run inside a Python thread so the main runner can observe a timeout even
/// while the test is still executing.
#[derive(Debug, Clone, Copy)]
pub struct TimeoutTag {
    seconds: f64,
}

impl TimeoutTag {
    pub(crate) fn new(seconds: f64) -> Self {
        Self { seconds }
    }

    pub(crate) fn seconds(self) -> f64 {
        self.seconds
    }

    pub(crate) fn try_from_pytest_mark(py_mark: &Bound<'_, PyAny>) -> Option<Self> {
        let args = py_mark.getattr("args").ok()?;
        let tuple = args.extract::<Bound<'_, PyTuple>>().ok()?;
        let first = tuple.get_item(0).ok()?;
        let seconds = first.extract::<f64>().ok()?;
        Some(Self { seconds })
    }
}

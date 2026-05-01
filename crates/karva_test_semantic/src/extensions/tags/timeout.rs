use pyo3::prelude::*;
use pyo3::types::PyTuple;

/// Represents a per-test timeout limit, in seconds.
///
/// Enforcement is performed by `run_test_with_timeout` in `utils.rs`:
/// sync tests run in a `ThreadPoolExecutor` worker, async tests are wrapped
/// in `asyncio.wait_for`.
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

    /// Parse `@pytest.mark.timeout(seconds)`.
    ///
    /// Drops the tag silently if the first positional arg is missing or not a
    /// finite, positive number — keeps behavior consistent with the
    /// Python-side `karva.tags.timeout` validator and avoids passing
    /// nonsensical values into `future.result()` / `asyncio.wait_for`.
    pub(crate) fn try_from_pytest_mark(py_mark: &Bound<'_, PyAny>) -> Option<Self> {
        let args = py_mark.getattr("args").ok()?;
        let tuple = args.extract::<Bound<'_, PyTuple>>().ok()?;
        let first = tuple.get_item(0).ok()?;
        let seconds = first.extract::<f64>().ok()?;
        if !(seconds.is_finite() && seconds > 0.0) {
            return None;
        }
        Some(Self { seconds })
    }
}

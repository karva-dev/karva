use std::collections::HashMap;
use std::fmt::Write;

use camino::Utf8Path;
use pyo3::prelude::*;
use pyo3::types::PyAnyMethods;
use pyo3::{PyResult, Python};
use ruff_source_file::{SourceFile, SourceFileBuilder};

/// Get the source file for the given utf8 path.
pub(crate) fn source_file(path: &Utf8Path) -> SourceFile {
    SourceFileBuilder::new(
        path.as_str(),
        std::fs::read_to_string(path).expect("Failed to read source file"),
    )
    .finish()
}

/// Runs a Python coroutine to completion using `asyncio.run()`.
pub(crate) fn run_coroutine(py: Python<'_>, coroutine: Py<PyAny>) -> PyResult<Py<PyAny>> {
    let asyncio = py.import("asyncio")?;
    Ok(asyncio.call_method1("run", (coroutine,))?.unbind())
}

/// Patches an async test function wrapped by a sync decorator (e.g. Hypothesis `@given`).
///
/// When `@given` decorates an `async def test_*()`, Hypothesis wraps it in a sync callable
/// and stores the original async function at `function.hypothesis.inner_test`. Without
/// patching, Hypothesis calls the async function directly, gets a coroutine, and raises
/// `InvalidArgument` because it cannot await it.
///
/// This function detects that situation and replaces `inner_test` with a sync wrapper
/// that uses `asyncio.run()`, following the Hypothesis-documented pattern for test runners.
///
/// Returns `true` if the function was patched (caller should NOT apply `asyncio.run()`),
/// or `false` if no patching was needed.
pub(crate) fn patch_async_test_function(py: Python<'_>, function: &Py<PyAny>) -> PyResult<bool> {
    let asyncio = py.import("asyncio")?;
    let is_coroutine_fn = asyncio
        .call_method1("iscoroutinefunction", (function,))?
        .extract::<bool>()?;

    // The callable itself is async — no decorator wrapping, use normal asyncio.run() path.
    if is_coroutine_fn {
        return Ok(false);
    }

    // The callable is sync (wrapped by a decorator). Check for Hypothesis inner_test.
    let Ok(hypothesis_attr) = function.getattr(py, "hypothesis") else {
        return Ok(false);
    };
    let Ok(inner_test) = hypothesis_attr.getattr(py, "inner_test") else {
        return Ok(false);
    };

    let inner_is_async = asyncio
        .call_method1("iscoroutinefunction", (&inner_test,))?
        .extract::<bool>()?;

    if !inner_is_async {
        return Ok(false);
    }

    // Replace inner_test with a sync wrapper that uses asyncio.run().
    // Uses inline Python because PyCFunction closures lack the signature metadata and
    // calling conventions that Hypothesis requires to introspect and invoke inner_test.
    let code = r"
def _make_sync(async_fn):
    import asyncio, functools
    @functools.wraps(async_fn)
    def wrapper(*args, **kwargs):
        return asyncio.run(async_fn(*args, **kwargs))
    return wrapper
";
    let locals = pyo3::types::PyDict::new(py);
    py.run(
        &std::ffi::CString::new(code).expect("valid CString"),
        None,
        Some(&locals),
    )?;
    let make_sync = locals
        .get_item("_make_sync")?
        .expect("_make_sync defined in code");
    let sync_wrapper = make_sync.call1((inner_test,))?;
    hypothesis_attr.setattr(py, "inner_test", sync_wrapper)?;

    Ok(true)
}

/// Adds a directory path to Python's sys.path at the specified index.
pub(crate) fn add_to_sys_path(py: Python<'_>, path: &Utf8Path, index: isize) -> PyResult<()> {
    let sys_module = py.import("sys")?;
    let sys_path = sys_module.getattr("path")?;
    sys_path.call_method1("insert", (index, path.to_string()))?;
    Ok(())
}

pub(crate) fn full_test_name(
    py: Python,
    function: String,
    kwargs: &HashMap<String, Py<PyAny>>,
) -> String {
    if kwargs.is_empty() {
        function
    } else {
        let mut args_str = String::new();
        let mut sorted_kwargs: Vec<_> = kwargs.iter().collect();
        sorted_kwargs.sort_by_key(|(key, _)| &**key);

        for (i, (key, value)) in sorted_kwargs.iter().enumerate() {
            if i > 0 {
                args_str.push_str(", ");
            }
            if let Ok(value) = value.cast_bound::<PyAny>(py) {
                let trimmed_value_str = truncate_string(&value.to_string());
                let truncated_key = truncate_string(key);
                let _ = write!(args_str, "{truncated_key}={trimmed_value_str}");
            }
        }
        format!("{function}({args_str})")
    }
}

/// Maximum display length for parameter keys and values in test names.
///
/// Keeps parameterized test names (e.g., `test_foo(key=value)`) readable in
/// CLI output by truncating long values with an ellipsis.
const TRUNCATE_LENGTH: usize = 30;

pub(crate) fn truncate_string(value: &str) -> String {
    if value.chars().count() > TRUNCATE_LENGTH {
        let truncated: String = value.chars().take(TRUNCATE_LENGTH - 3).collect();
        format!("{truncated}...")
    } else {
        value.to_string()
    }
}

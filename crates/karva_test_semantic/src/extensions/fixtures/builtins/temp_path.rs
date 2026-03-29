use std::sync::atomic::{AtomicU32, Ordering};

use pyo3::prelude::*;
use tempfile::TempDir;

pub fn is_temp_path_fixture_name(fixture_name: &str) -> bool {
    matches!(fixture_name, "tmp_path" | "temp_path" | "temp_dir")
}

pub fn is_tmpdir_fixture_name(fixture_name: &str) -> bool {
    matches!(fixture_name, "tmpdir")
}

pub fn is_tmp_path_factory_fixture_name(fixture_name: &str) -> bool {
    matches!(fixture_name, "tmp_path_factory")
}

pub fn is_tmpdir_factory_fixture_name(fixture_name: &str) -> bool {
    matches!(fixture_name, "tmpdir_factory")
}

/// Create a `pathlib.Path` temporary directory fixture.
///
/// Resolves symlinks so the path matches what `Path.resolve()` would return.
/// On macOS, /var/folders/... is a symlink to /private/var/folders/..., which
/// causes path equality checks to fail when test code calls `Path.resolve()`.
pub fn create_temp_dir_fixture(py: Python<'_>) -> Option<Py<PyAny>> {
    let path_str = make_temp_dir()?;

    let pathlib = py.import("pathlib").ok()?;
    let path_class = pathlib.getattr("Path").ok()?;
    let path_obj = path_class.call1((path_str,)).ok()?;

    Some(path_obj.unbind())
}

/// Create a `py.path.local` temporary directory fixture (`tmpdir`).
///
/// Returns a `py.path.local` object (provided by pytest's bundled `_pytest._py`
/// or the standalone `py` package) for backward-compatibility with older test code.
/// Falls back to a `pathlib.Path` if neither is available.
pub fn create_tmpdir_fixture(py: Python<'_>) -> Option<Py<PyAny>> {
    let path_str = make_temp_dir()?;

    // Try `py.path.local` first (standalone `py` package), then pytest's bundled copy.
    let local_class = py
        .import("py")
        .ok()
        .and_then(|m| m.getattr("path").ok())
        .and_then(|p| p.getattr("local").ok())
        .or_else(|| {
            py.import("_pytest._py.path")
                .ok()
                .and_then(|m| m.getattr("LocalPath").ok())
        });

    if let Some(local_class) = local_class {
        local_class.call1((path_str,)).ok().map(Bound::unbind)
    } else {
        // Fall back to pathlib.Path if py.path.local is not available.
        let pathlib = py.import("pathlib").ok()?;
        let path_class = pathlib.getattr("Path").ok()?;
        path_class.call1((path_str,)).ok().map(Bound::unbind)
    }
}

fn make_temp_dir() -> Option<String> {
    let temp_dir = TempDir::with_prefix("karva-").ok()?;

    // Resolve symlinks so the path matches what `Path.resolve()` would return.
    // On macOS, /var/folders/... is a symlink to /private/var/folders/..., which
    // causes path equality checks to fail when test code calls `Path.resolve()`.
    let resolved = temp_dir.path().canonicalize().ok()?;
    let path_str = resolved.to_str()?.to_string();

    let _ = temp_dir.keep();
    Some(path_str)
}

/// Get the `py.path.local` class, trying `py.path.local` then `_pytest._py.path.LocalPath`.
fn get_local_path_class(py: Python<'_>) -> Option<Bound<'_, PyAny>> {
    py.import("py")
        .ok()
        .and_then(|m| m.getattr("path").ok())
        .and_then(|p| p.getattr("local").ok())
        .or_else(|| {
            py.import("_pytest._py.path")
                .ok()
                .and_then(|m| m.getattr("LocalPath").ok())
        })
}

/// Create a `TempPathFactory` fixture (`tmp_path_factory`).
///
/// Returns a factory object with `mktemp(basename)` and `getbasetemp()` that
/// produce `pathlib.Path` objects. The factory owns a unique base temp directory
/// and creates numbered subdirs via `mktemp()`.
pub fn create_tmp_path_factory_fixture(py: Python<'_>) -> Option<Py<PyAny>> {
    let base = make_temp_dir()?;
    let factory = TmpPathFactory::new(base);
    Py::new(py, factory).ok().map(Py::into_any)
}

/// Create a `TmpDirFactory` fixture (`tmpdir_factory`).
///
/// Like `tmp_path_factory` but `mktemp()` returns `py.path.local` objects.
pub fn create_tmpdir_factory_fixture(py: Python<'_>) -> Option<Py<PyAny>> {
    let base = make_temp_dir()?;
    let factory = TmpDirFactory::new(base);
    Py::new(py, factory).ok().map(Py::into_any)
}

/// Factory for `tmp_path_factory` — produces numbered `pathlib.Path` subdirs.
#[pyclass]
struct TmpPathFactory {
    basetemp: String,
    counter: AtomicU32,
}

impl TmpPathFactory {
    fn new(basetemp: String) -> Self {
        Self {
            basetemp,
            counter: AtomicU32::new(0),
        }
    }
}

#[pymethods]
impl TmpPathFactory {
    #[pyo3(signature = (basename, numbered = true))]
    fn mktemp(&self, py: Python<'_>, basename: &str, numbered: bool) -> PyResult<Py<PyAny>> {
        let dir_name = if numbered {
            let n = self.counter.fetch_add(1, Ordering::SeqCst);
            format!("{basename}{n}")
        } else {
            basename.to_string()
        };

        let path = std::path::Path::new(&self.basetemp).join(&dir_name);
        std::fs::create_dir_all(&path).map_err(|e| {
            pyo3::exceptions::PyOSError::new_err(format!("Failed to create dir: {e}"))
        })?;

        let pathlib = py.import("pathlib")?;
        let path_class = pathlib.getattr("Path")?;
        Ok(path_class
            .call1((path.to_string_lossy().as_ref(),))?
            .unbind())
    }

    fn getbasetemp(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let pathlib = py.import("pathlib")?;
        let path_class = pathlib.getattr("Path")?;
        Ok(path_class.call1((self.basetemp.as_str(),))?.unbind())
    }

    fn __repr__(&self) -> String {
        format!("<TmpPathFactory basetemp={}>", self.basetemp)
    }
}

/// Factory for `tmpdir_factory` — produces numbered `py.path.local` subdirs.
#[pyclass]
struct TmpDirFactory {
    basetemp: String,
    counter: AtomicU32,
}

impl TmpDirFactory {
    fn new(basetemp: String) -> Self {
        Self {
            basetemp,
            counter: AtomicU32::new(0),
        }
    }
}

#[pymethods]
impl TmpDirFactory {
    #[pyo3(signature = (basename, numbered = true))]
    fn mktemp(&self, py: Python<'_>, basename: &str, numbered: bool) -> PyResult<Py<PyAny>> {
        let dir_name = if numbered {
            let n = self.counter.fetch_add(1, Ordering::SeqCst);
            format!("{basename}{n}")
        } else {
            basename.to_string()
        };

        let path = std::path::Path::new(&self.basetemp).join(&dir_name);
        std::fs::create_dir_all(&path).map_err(|e| {
            pyo3::exceptions::PyOSError::new_err(format!("Failed to create dir: {e}"))
        })?;

        let path_str = path.to_string_lossy().into_owned();

        if let Some(local_class) = get_local_path_class(py) {
            local_class.call1((path_str,)).map(Bound::unbind)
        } else {
            let pathlib = py.import("pathlib")?;
            let path_class = pathlib.getattr("Path")?;
            path_class.call1((path_str,)).map(Bound::unbind)
        }
    }

    fn getbasetemp(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        if let Some(local_class) = get_local_path_class(py) {
            local_class
                .call1((self.basetemp.as_str(),))
                .map(Bound::unbind)
        } else {
            let pathlib = py.import("pathlib")?;
            let path_class = pathlib.getattr("Path")?;
            path_class
                .call1((self.basetemp.as_str(),))
                .map(Bound::unbind)
        }
    }

    fn __repr__(&self) -> String {
        format!("<TmpDirFactory basetemp={}>", self.basetemp)
    }
}

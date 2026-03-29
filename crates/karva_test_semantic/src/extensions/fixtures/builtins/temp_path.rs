use std::sync::atomic::{AtomicU32, Ordering};

use pyo3::prelude::*;
use pyo3::types::PyDict;
use tempfile::TempDir;

/// Minimal `py.path.local`-compatible shim class, defined as inline Python.
///
/// Used as a last resort when neither the standalone `py` package nor pytest's
/// bundled `_pytest._py.path.LocalPath` is importable (e.g., when running tests
/// without pytest installed).  The shim wraps `pathlib.Path` and exposes the
/// subset of the `py.path.local` API that real-world test suites rely on:
/// `join`, `mkdir`, `makedirs`, `read`, `write`, `isdir`, `isfile`, `exists`,
/// `strpath`, `basename`, `dirname`, `dirpath`, `listdir`, `remove`, etc.
const LOCAL_PATH_SHIM: &str = r#"
import pathlib
import shutil

class LocalPath:
    def __init__(self, path):
        self._path = pathlib.Path(str(path))

    @property
    def strpath(self):
        return str(self._path)

    def __str__(self):
        return self.strpath

    def __repr__(self):
        return "local({!r})".format(self.strpath)

    def __eq__(self, other):
        if isinstance(other, type(self)):
            return self._path == other._path
        return str(self._path) == str(other)

    def __ne__(self, other):
        return not self.__eq__(other)

    def __hash__(self):
        return hash(self._path)

    def __fspath__(self):
        return self.strpath

    def __truediv__(self, other):
        return self.join(str(other))

    def join(self, *args):
        result = self._path
        for arg in args:
            result = result / str(arg)
        return type(self)(result)

    def mkdir(self, mode=0o777):
        self._path.mkdir(mode=mode)
        return self

    def makedirs(self, mode=0o777):
        self._path.mkdir(parents=True, exist_ok=True, mode=mode)
        return self

    def isdir(self):
        return self._path.is_dir()

    def isfile(self):
        return self._path.is_file()

    def exists(self):
        return self._path.exists()

    def check(self, **kw):
        return self._path.exists()

    def read(self, mode="r"):
        if "b" in mode:
            return self._path.read_bytes()
        return self._path.read_text()

    def read_binary(self):
        return self._path.read_bytes()

    def write(self, content, mode="w"):
        if "b" in mode:
            self._path.write_bytes(content)
        else:
            self._path.write_text(str(content))
        return self

    def write_binary(self, data):
        self._path.write_bytes(data)
        return self

    @property
    def basename(self):
        return self._path.name

    @property
    def dirname(self):
        return str(self._path.parent)

    @property
    def ext(self):
        return self._path.suffix

    def dirpath(self, *args):
        p = type(self)(self._path.parent)
        if args:
            return p.join(*args)
        return p

    def listdir(self, fil=None):
        entries = [type(self)(p) for p in self._path.iterdir()]
        if fil is not None:
            entries = [e for e in entries if fil(e)]
        return entries

    def remove(self, rec=1):
        if self._path.is_dir():
            shutil.rmtree(self._path)
        else:
            self._path.unlink()

    def stat(self):
        return self._path.stat()

    def size(self):
        return self._path.stat().st_size

    def mtime(self):
        return self._path.stat().st_mtime

    def copy(self, target):
        shutil.copy2(str(self._path), str(target))
        return type(self)(target)

    def move(self, target):
        shutil.move(str(self._path), str(target))
        return type(self)(target)
"#;

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
/// Returns a `py.path.local`-compatible object for backward-compatibility with
/// older test code.  Tries the standalone `py` package first, then pytest's
/// bundled `_pytest._py.path.LocalPath`, then falls back to the built-in shim.
pub fn create_tmpdir_fixture(py: Python<'_>) -> Option<Py<PyAny>> {
    let path_str = make_temp_dir()?;
    get_local_path_class(py)
        .and_then(|cls| cls.call1((path_str,)).ok())
        .map(Bound::unbind)
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

/// Get the `py.path.local` class, trying in order:
/// 1. `py.path.local` (standalone `py` package)
/// 2. `_pytest._py.path.LocalPath` (pytest's bundled copy)
/// 3. The built-in [`LOCAL_PATH_SHIM`] (always available, no external dependencies)
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
        .or_else(|| get_local_path_shim_class(py))
}

/// Define and return the [`LOCAL_PATH_SHIM`] `LocalPath` class.
fn get_local_path_shim_class(py: Python<'_>) -> Option<Bound<'_, PyAny>> {
    let globals = PyDict::new(py);
    py.run(
        &std::ffi::CString::new(LOCAL_PATH_SHIM).expect("shim code contains no null bytes"),
        Some(&globals),
        None,
    )
    .ok()?;
    globals.get_item("LocalPath").ok().flatten()
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

        get_local_path_class(py)
            .and_then(|cls| cls.call1((path_str,)).ok())
            .map(Bound::unbind)
            .ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err("Failed to create local path object")
            })
    }

    fn getbasetemp(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        get_local_path_class(py)
            .and_then(|cls| cls.call1((self.basetemp.as_str(),)).ok())
            .map(Bound::unbind)
            .ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err("Failed to create local path object")
            })
    }

    fn __repr__(&self) -> String {
        format!("<TmpDirFactory basetemp={}>", self.basetemp)
    }
}

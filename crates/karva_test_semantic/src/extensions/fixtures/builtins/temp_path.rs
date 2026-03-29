use pyo3::prelude::*;
use tempfile::TempDir;

pub fn is_temp_path_fixture_name(fixture_name: &str) -> bool {
    matches!(fixture_name, "tmp_path" | "temp_path" | "temp_dir")
}

pub fn is_tmpdir_fixture_name(fixture_name: &str) -> bool {
    matches!(fixture_name, "tmpdir")
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
    // causes path equality checks to fail when test code calls Path.resolve().
    let resolved = temp_dir.path().canonicalize().ok()?;
    let path_str = resolved.to_str()?.to_string();

    let _ = temp_dir.keep();
    Some(path_str)
}

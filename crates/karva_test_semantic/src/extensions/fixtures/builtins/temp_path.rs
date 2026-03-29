use pyo3::prelude::*;
use tempfile::TempDir;

pub fn is_temp_path_fixture_name(fixture_name: &str) -> bool {
    matches!(
        fixture_name,
        "tmp_path" | "tmpdir" | "temp_path" | "temp_dir"
    )
}

pub fn create_temp_dir_fixture(py: Python<'_>) -> Option<Py<PyAny>> {
    let temp_dir = TempDir::with_prefix("karva-").ok()?;

    // Resolve symlinks so the path matches what `Path.resolve()` would return.
    // On macOS, /var/folders/... is a symlink to /private/var/folders/..., which
    // causes path equality checks to fail when test code calls Path.resolve().
    let resolved = temp_dir.path().canonicalize().ok()?;
    let path_str = resolved.to_str()?.to_string();

    let _ = temp_dir.keep();

    let pathlib = py.import("pathlib").ok()?;
    let path_class = pathlib.getattr("Path").ok()?;
    let path_obj = path_class.call1((path_str,)).ok()?;

    Some(path_obj.unbind())
}

use anyhow::{Context, Result};
use camino::Utf8PathBuf;

const KARVA_WORKER_BINARY_NAME: &str = "karva-worker";

/// Find the `karva-worker` binary by checking PATH, the project venv, and the active venv.
pub fn find_karva_worker_binary(current_dir: &Utf8PathBuf) -> Result<Utf8PathBuf> {
    which::which(KARVA_WORKER_BINARY_NAME)
        .ok()
        .and_then(|path| Utf8PathBuf::try_from(path).ok())
        .inspect(|path| tracing::debug!(path = %path, "Found binary in PATH"))
        .or_else(|| venv_binary(KARVA_WORKER_BINARY_NAME, current_dir))
        .or_else(|| venv_binary_from_active_env(KARVA_WORKER_BINARY_NAME))
        .context("Could not find karva-worker binary")
}

/// Construct a platform-specific binary path within a virtual environment root directory.
fn construct_binary_path(venv_root: &Utf8PathBuf, binary_name: &str) -> Utf8PathBuf {
    if cfg!(target_os = "windows") {
        venv_root.join("Scripts").join(format!("{binary_name}.exe"))
    } else {
        venv_root.join("bin").join(binary_name)
    }
}

/// Check if a binary exists within a virtual environment root and return its path.
fn venv_binary_at(venv_root: &Utf8PathBuf, binary_name: &str) -> Option<Utf8PathBuf> {
    let binary_path = construct_binary_path(venv_root, binary_name);
    binary_path.exists().then_some(binary_path)
}

fn venv_binary(binary_name: &str, directory: &Utf8PathBuf) -> Option<Utf8PathBuf> {
    venv_binary_at(&directory.join(".venv"), binary_name)
}

fn venv_binary_from_active_env(binary_name: &str) -> Option<Utf8PathBuf> {
    let venv_root = std::env::var_os("VIRTUAL_ENV")?;
    let venv_root = Utf8PathBuf::from_path_buf(venv_root.into()).ok()?;
    venv_binary_at(&venv_root, binary_name)
}

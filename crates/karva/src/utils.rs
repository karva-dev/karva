use anyhow::Context;
use camino::Utf8PathBuf;

/// Get the current working directory as a UTF-8 path.
pub fn cwd() -> anyhow::Result<Utf8PathBuf> {
    let cwd = std::env::current_dir().context("Failed to get the current working directory")?;
    Utf8PathBuf::from_path_buf(cwd).map_err(|path| {
        anyhow::anyhow!(
            "The current working directory `{}` contains non-Unicode characters.",
            path.display()
        )
    })
}

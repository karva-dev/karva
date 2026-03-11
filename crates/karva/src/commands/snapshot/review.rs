use anyhow::Result;
use camino::Utf8Path;

use super::resolve_filter_paths;
use crate::ExitStatus;

pub fn review(cwd: &Utf8Path, filter_paths: &[String]) -> Result<ExitStatus> {
    let resolved = resolve_filter_paths(filter_paths, cwd);
    karva_snapshot::review::run_review(cwd, &resolved)?;
    Ok(ExitStatus::Success)
}

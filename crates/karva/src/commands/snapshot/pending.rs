use std::fmt::Write;

use anyhow::Result;
use camino::Utf8Path;

use super::{matches_filter, resolve_filter_paths};
use crate::ExitStatus;

pub fn pending(
    cwd: &Utf8Path,
    stdout: &mut impl Write,
    filter_paths: &[String],
) -> Result<ExitStatus> {
    let pending = karva_snapshot::storage::find_pending_snapshots(cwd);
    let resolved = resolve_filter_paths(filter_paths, cwd);
    let filtered: Vec<_> = pending
        .iter()
        .filter(|info| matches_filter(&info.pending_path, &resolved))
        .collect();
    if filtered.is_empty() {
        writeln!(stdout, "No pending snapshots.")?;
        return Ok(ExitStatus::Success);
    }
    for info in &filtered {
        writeln!(stdout, "{}", info.pending_path)?;
    }
    writeln!(stdout, "\n{} pending snapshot(s).", filtered.len())?;
    Ok(ExitStatus::Success)
}

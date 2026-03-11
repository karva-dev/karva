use std::fmt::Write;

use anyhow::Result;
use camino::Utf8Path;

use super::{matches_filter, resolve_filter_paths};
use crate::ExitStatus;

pub fn accept(
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
        writeln!(stdout, "No pending snapshots found.")?;
        return Ok(ExitStatus::Success);
    }
    karva_snapshot::storage::accept_pending_batch(&filtered)?;
    for info in &filtered {
        writeln!(stdout, "Accepted: {}", info.pending_path)?;
    }
    writeln!(stdout, "\n{} snapshot(s) accepted.", filtered.len())?;
    Ok(ExitStatus::Success)
}

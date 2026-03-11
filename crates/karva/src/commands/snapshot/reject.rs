use std::fmt::Write;

use anyhow::Result;
use camino::Utf8Path;

use super::{matches_filter, resolve_filter_paths};
use crate::ExitStatus;

pub fn reject(
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
    let mut rejected = 0;
    for info in &filtered {
        karva_snapshot::storage::reject_pending(&info.pending_path)?;
        writeln!(stdout, "Rejected: {}", info.pending_path)?;
        rejected += 1;
    }
    writeln!(stdout, "\n{rejected} snapshot(s) rejected.")?;
    Ok(ExitStatus::Success)
}

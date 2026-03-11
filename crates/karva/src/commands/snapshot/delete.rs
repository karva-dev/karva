use std::fmt::Write;

use anyhow::Result;
use camino::Utf8Path;

use super::{matches_filter, resolve_filter_paths};
use crate::ExitStatus;

pub fn delete(
    cwd: &Utf8Path,
    stdout: &mut impl Write,
    filter_paths: &[String],
    dry_run: bool,
) -> Result<ExitStatus> {
    let all = karva_snapshot::storage::find_all_snapshots(cwd);
    let resolved = resolve_filter_paths(filter_paths, cwd);
    let filtered: Vec<_> = all
        .iter()
        .filter(|info| matches_filter(&info.path, &resolved))
        .collect();
    if filtered.is_empty() {
        writeln!(stdout, "No snapshot files found.")?;
        return Ok(ExitStatus::Success);
    }
    if dry_run {
        for info in &filtered {
            writeln!(stdout, "Would delete: {}", info.path)?;
        }
        writeln!(
            stdout,
            "\n{} snapshot file(s) would be deleted.",
            filtered.len()
        )?;
    } else {
        let mut deleted = 0;
        for info in &filtered {
            karva_snapshot::storage::remove_snapshot(&info.path)?;
            writeln!(stdout, "Deleted: {}", info.path)?;
            deleted += 1;
        }
        writeln!(stdout, "\n{deleted} snapshot file(s) deleted.")?;
    }
    Ok(ExitStatus::Success)
}

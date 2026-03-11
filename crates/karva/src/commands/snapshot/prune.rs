use std::fmt::Write;

use anyhow::Result;
use camino::Utf8Path;
use colored::Colorize;

use super::{matches_filter, resolve_filter_paths};
use crate::ExitStatus;

pub fn prune(
    cwd: &Utf8Path,
    stdout: &mut impl Write,
    filter_paths: &[String],
    dry_run: bool,
) -> Result<ExitStatus> {
    {
        use std::io::Write;
        writeln!(
            std::io::stderr(),
            "{} Prune uses static analysis and may not detect all unreferenced snapshots.",
            "warning:".yellow().bold()
        )?;
    }
    let unreferenced = karva_snapshot::storage::find_unreferenced_snapshots(cwd);
    let resolved = resolve_filter_paths(filter_paths, cwd);
    let filtered: Vec<_> = unreferenced
        .iter()
        .filter(|info| matches_filter(&info.snap_path, &resolved))
        .collect();
    if filtered.is_empty() {
        writeln!(stdout, "No unreferenced snapshots found.")?;
        return Ok(ExitStatus::Success);
    }
    if dry_run {
        for info in &filtered {
            writeln!(stdout, "Would remove: {} ({})", info.snap_path, info.reason)?;
        }
        writeln!(
            stdout,
            "\n{} unreferenced snapshot(s) would be removed.",
            filtered.len()
        )?;
    } else {
        let mut removed = 0;
        for info in &filtered {
            karva_snapshot::storage::remove_snapshot(&info.snap_path)?;
            writeln!(stdout, "Removed: {} ({})", info.snap_path, info.reason)?;
            removed += 1;
        }
        writeln!(stdout, "\n{removed} snapshot(s) pruned.")?;
    }
    Ok(ExitStatus::Success)
}

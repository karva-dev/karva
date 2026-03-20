use std::fmt::Write;

use anyhow::Result;
use karva_logging::Printer;

use super::{matches_filter, resolve_filter_paths};
use crate::ExitStatus;
use crate::utils::cwd;

pub fn delete(filter_paths: &[String], dry_run: bool) -> Result<ExitStatus> {
    let cwd = cwd()?;
    let printer = Printer::default();
    let mut stdout = printer.stream_for_requested_summary().lock();
    let all = karva_snapshot::storage::find_all_snapshots(&cwd);
    let resolved = resolve_filter_paths(filter_paths, &cwd);
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
        for info in &filtered {
            karva_snapshot::storage::remove_snapshot(&info.path)?;
            writeln!(stdout, "Deleted: {}", info.path)?;
        }
        writeln!(stdout, "\n{} snapshot file(s) deleted.", filtered.len())?;
    }
    Ok(ExitStatus::Success)
}

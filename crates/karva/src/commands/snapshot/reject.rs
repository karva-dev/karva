use std::fmt::Write;

use anyhow::Result;
use karva_logging::Printer;

use super::{matches_filter, resolve_filter_paths};
use crate::ExitStatus;
use crate::utils::cwd;

pub fn reject(filter_paths: &[String]) -> Result<ExitStatus> {
    let cwd = cwd()?;
    let printer = Printer::default();
    let mut stdout = printer.stream_for_requested_summary().lock();
    let pending = karva_snapshot::storage::find_pending_snapshots(&cwd);
    let resolved = resolve_filter_paths(filter_paths, &cwd);
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

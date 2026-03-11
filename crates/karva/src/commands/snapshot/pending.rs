use std::fmt::Write;

use anyhow::Result;
use karva_logging::Printer;

use super::{matches_filter, resolve_filter_paths};
use crate::ExitStatus;
use crate::utils::cwd;

pub fn pending(filter_paths: &[String]) -> Result<ExitStatus> {
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
        writeln!(stdout, "No pending snapshots.")?;
        return Ok(ExitStatus::Success);
    }
    for info in &filtered {
        writeln!(stdout, "{}", info.pending_path)?;
    }
    writeln!(stdout, "\n{} pending snapshot(s).", filtered.len())?;
    Ok(ExitStatus::Success)
}

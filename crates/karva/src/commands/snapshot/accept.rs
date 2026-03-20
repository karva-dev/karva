use std::fmt::Write;

use anyhow::Result;

use super::{filter_or_empty, snapshot_setup};
use crate::ExitStatus;

pub fn accept(filter_paths: &[String]) -> Result<ExitStatus> {
    let (mut stdout, cwd, resolved) = snapshot_setup(filter_paths)?;
    let pending = karva_snapshot::storage::find_pending_snapshots(&cwd);
    let Some(filtered) = filter_or_empty(
        &pending,
        &resolved,
        |i| &i.pending_path,
        "No pending snapshots found.",
        &mut stdout,
    )?
    else {
        return Ok(ExitStatus::Success);
    };
    karva_snapshot::storage::accept_pending_batch(&filtered)?;
    for info in &filtered {
        writeln!(stdout, "Accepted: {}", info.pending_path)?;
    }
    writeln!(stdout, "\n{} snapshot(s) accepted.", filtered.len())?;
    Ok(ExitStatus::Success)
}

use std::fmt::Write;

use anyhow::Result;

use super::{filter_or_empty, snapshot_setup};
use crate::ExitStatus;

pub fn reject(filter_paths: &[String]) -> Result<ExitStatus> {
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
    for info in &filtered {
        karva_snapshot::storage::reject_pending(&info.pending_path)?;
        writeln!(stdout, "Rejected: {}", info.pending_path)?;
    }
    writeln!(stdout, "\n{} snapshot(s) rejected.", filtered.len())?;
    Ok(ExitStatus::Success)
}

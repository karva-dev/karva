mod accept;
mod delete;
mod pending;
mod prune;
mod reject;
mod review;

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use karva_cli::{SnapshotAction, SnapshotCommand};
use karva_logging::Printer;
use karva_project::path::absolute;

use crate::ExitStatus;
use crate::utils::cwd;

pub fn snapshot(args: SnapshotCommand) -> Result<ExitStatus> {
    let cwd = cwd()?;

    let printer = Printer::default();
    let mut stdout = printer.stream_for_requested_summary().lock();

    match args.action {
        SnapshotAction::Accept(filter) => accept::accept(&cwd, &mut stdout, &filter.paths),
        SnapshotAction::Reject(filter) => reject::reject(&cwd, &mut stdout, &filter.paths),
        SnapshotAction::Pending(filter) => pending::pending(&cwd, &mut stdout, &filter.paths),
        SnapshotAction::Review(filter) => {
            drop(stdout);
            review::review(&cwd, &filter.paths)
        }
        SnapshotAction::Prune(prune_args) => {
            prune::prune(&cwd, &mut stdout, &prune_args.paths, prune_args.dry_run)
        }
        SnapshotAction::Delete(delete_args) => {
            delete::delete(&cwd, &mut stdout, &delete_args.paths, delete_args.dry_run)
        }
    }
}

/// Resolve user-provided filter strings to absolute paths.
fn resolve_filter_paths(filter_paths: &[String], cwd: &Utf8Path) -> Vec<Utf8PathBuf> {
    filter_paths.iter().map(|f| absolute(f, cwd)).collect()
}

/// Check if a snapshot path matches any resolved filter (absolute path prefix match).
/// Returns true if filters is empty (match all).
fn matches_filter(snapshot_path: &Utf8Path, resolved_filters: &[Utf8PathBuf]) -> bool {
    resolved_filters.is_empty()
        || resolved_filters
            .iter()
            .any(|f| snapshot_path.as_str().starts_with(f.as_str()))
}

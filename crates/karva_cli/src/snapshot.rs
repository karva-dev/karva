use clap::Parser;

#[derive(Debug, Parser)]
pub struct SnapshotCommand {
    #[command(subcommand)]
    pub action: SnapshotAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum SnapshotAction {
    /// Accept all (or filtered) pending snapshots.
    Accept(SnapshotFilterArgs),

    /// Reject all (or filtered) pending snapshots.
    Reject(SnapshotFilterArgs),

    /// List pending snapshots.
    Pending(SnapshotFilterArgs),

    /// Interactively review pending snapshots.
    Review(SnapshotFilterArgs),

    /// Remove snapshot files whose source test no longer exists.
    Prune(SnapshotPruneArgs),

    /// Delete all (or filtered) snapshot files (.snap and .snap.new).
    Delete(SnapshotDeleteArgs),
}

#[derive(Debug, Parser, Default)]
pub struct SnapshotFilterArgs {
    /// Optional paths to filter snapshots by directory or file.
    #[clap(value_name = "PATH")]
    pub paths: Vec<String>,
}

#[derive(Debug, Parser, Default)]
pub struct SnapshotPruneArgs {
    /// Optional paths to filter snapshots by directory or file.
    #[clap(value_name = "PATH")]
    pub paths: Vec<String>,

    /// Show which snapshots would be removed without deleting them.
    #[clap(long)]
    pub dry_run: bool,
}

#[derive(Debug, Parser, Default)]
pub struct SnapshotDeleteArgs {
    /// Optional paths to filter which snapshot files are deleted.
    #[clap(value_name = "PATH")]
    pub paths: Vec<String>,

    /// Show which snapshot files would be deleted without removing them.
    #[clap(long)]
    pub dry_run: bool,
}

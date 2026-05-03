pub(crate) mod artifact;
pub(crate) mod cache;
pub(crate) mod hash;

pub use cache::{
    AggregatedResults, Cache, PruneResult, clean_cache, prune_cache, read_last_failed,
    read_recent_durations, write_last_failed,
};
pub use hash::RunHash;
pub use karva_diagnostic::{DisplayFlakyTests, FlakyTest};

/// The directory name used for the cache, relative to the project root.
pub const CACHE_DIR: &str = ".karva_cache";

/// Returns the conventional sub-directory name for a worker within a run directory.
pub(crate) fn worker_folder(worker_id: usize) -> String {
    format!("worker-{worker_id}")
}

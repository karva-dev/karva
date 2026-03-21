use std::num::NonZeroUsize;

pub struct EnvVars;

impl EnvVars {
    /// This is a standard Rayon environment variable.
    pub const RAYON_NUM_THREADS: &'static str = "RAYON_NUM_THREADS";

    /// This is a standard Karva environment variable.
    pub const KARVA_MAX_PARALLELISM: &'static str = "KARVA_MAX_PARALLELISM";

    /// This is a standard Karva environment variable.
    pub const KARVA_CONFIG_FILE: &'static str = "KARVA_CONFIG_FILE";

    /// When set to "1" or "true", snapshot assertions write directly to `.snap`
    /// instead of creating `.snap.new` pending files.
    pub const KARVA_SNAPSHOT_UPDATE: &'static str = "KARVA_SNAPSHOT_UPDATE";

    /// Override the path to the `karva-worker` binary.
    /// Used for coverage instrumentation where the cargo-built binary
    /// should be used instead of the venv-installed console script.
    pub const KARVA_WORKER_BINARY: &'static str = "KARVA_WORKER_BINARY";

    /// Private env var that activates venv support for the embedded Python
    /// interpreter. Set by `just coverage-full` so the cargo-built worker
    /// can find pytest, karva, etc. from the venv's site-packages.
    /// The double-underscore prefix signals this is an internal implementation detail.
    #[doc(hidden)]
    pub const KARVA_COVERAGE_INTERNAL: &'static str = "__KARVA_COVERAGE";
}

pub fn max_parallelism() -> NonZeroUsize {
    std::env::var(EnvVars::KARVA_MAX_PARALLELISM)
        .or_else(|_| std::env::var(EnvVars::RAYON_NUM_THREADS))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN))
}

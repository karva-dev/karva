use clap::Parser;

#[derive(Debug, Parser)]
pub struct CacheCommand {
    #[command(subcommand)]
    pub action: CacheAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum CacheAction {
    /// Remove all but the most recent test run from the cache.
    Prune,

    /// Remove the entire cache directory.
    Clean,
}

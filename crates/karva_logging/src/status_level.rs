/// Which test result statuses to display during the run.
///
/// Modeled after `cargo-nextest`'s `--status-level`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum StatusLevel {
    /// Don't display any test result lines (or the "Starting" header).
    None,
    /// Only display failed test results.
    Fail,
    /// Display failed and skipped test results.
    Skip,
    /// Display all test results (default).
    #[default]
    Pass,
    /// Display all test results.
    All,
}

impl StatusLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Fail => "fail",
            Self::Skip => "skip",
            Self::Pass => "pass",
            Self::All => "all",
        }
    }
}

/// Which final summary information to display at the end of the run.
///
/// Modeled after `cargo-nextest`'s `--final-status-level`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum FinalStatusLevel {
    /// Don't display the summary line or any diagnostic blocks.
    None,
    /// Only display the summary line and diagnostics on failure.
    Fail,
    /// Always display the summary line; diagnostics shown when failures exist (default).
    #[default]
    Pass,
    /// Always display the summary line and diagnostics.
    All,
}

impl FinalStatusLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Fail => "fail",
            Self::Pass => "pass",
            Self::All => "all",
        }
    }
}

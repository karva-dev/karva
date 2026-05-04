use ruff_db::diagnostic::DiagnosticFormat;

use karva_metadata::{NoTestsMode, RunIgnoredMode};

/// Coverage terminal report type.
#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq, PartialOrd, Ord, Default, clap::ValueEnum)]
pub enum CovReport {
    /// Compact terminal table (default).
    #[default]
    #[value(name = "term")]
    Term,

    /// Terminal table with a `Missing` column listing uncovered line numbers.
    #[value(name = "term-missing")]
    TermMissing,
}

/// The diagnostic output format.
#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq, PartialOrd, Ord, Default, clap::ValueEnum)]
pub enum OutputFormat {
    /// Print diagnostics verbosely, with context and helpful hints (default).
    #[default]
    #[value(name = "full")]
    Full,

    /// Print diagnostics concisely, one per line.
    #[value(name = "concise")]
    Concise,
}

impl From<OutputFormat> for DiagnosticFormat {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Full => Self::Full,
            OutputFormat::Concise => Self::Concise,
        }
    }
}

impl From<OutputFormat> for karva_metadata::OutputFormat {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Full => Self::Full,
            OutputFormat::Concise => Self::Concise,
        }
    }
}

impl From<CovReport> for karva_metadata::CovReport {
    fn from(value: CovReport) -> Self {
        match value {
            CovReport::Term => Self::Term,
            CovReport::TermMissing => Self::TermMissing,
        }
    }
}

/// Whether to run ignored/skipped tests.
#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum RunIgnored {
    /// Run only ignored tests.
    Only,

    /// Run both ignored and non-ignored tests.
    All,
}

impl RunIgnored {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Only => "only",
            Self::All => "all",
        }
    }
}

impl From<RunIgnored> for RunIgnoredMode {
    fn from(value: RunIgnored) -> Self {
        match value {
            RunIgnored::Only => Self::Only,
            RunIgnored::All => Self::All,
        }
    }
}

/// Behavior when no tests match filters.
#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum NoTests {
    /// Automatically determine behavior: fail if no filter expressions were
    /// given, pass silently if filters were given.
    Auto,

    /// Silently exit with code 0.
    Pass,

    /// Produce a warning and exit with code 0.
    Warn,

    /// Produce an error message and exit with a non-zero code.
    Fail,
}

impl From<NoTests> for NoTestsMode {
    fn from(value: NoTests) -> Self {
        match value {
            NoTests::Auto => Self::Auto,
            NoTests::Pass => Self::Pass,
            NoTests::Warn => Self::Warn,
            NoTests::Fail => Self::Fail,
        }
    }
}

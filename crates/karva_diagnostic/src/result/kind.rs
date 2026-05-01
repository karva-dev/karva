/// The outcome of a single test execution as observed by the runner.
///
/// Carries optional context (such as the reason a test was skipped) that
/// is dropped when collapsed into [`TestResultKind`] for stats purposes.
#[derive(Debug, Clone)]
pub enum IndividualTestResultKind {
    Passed,
    Failed,
    Skipped { reason: Option<String> },
}

/// A test result kind suitable for aggregation in [`super::TestResultStats`].
///
/// Unlike [`IndividualTestResultKind`] this is plain, hashable, and copyable
/// — it drops contextual fields (like skip reasons) and gains the synthetic
/// `Flaky` marker for tests that passed only after retries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum TestResultKind {
    Passed,
    Failed,
    Skipped,
    /// A test that passed only after at least one retry. Tracked alongside
    /// (not instead of) `Passed` so the summary can show how many of the
    /// passing tests are flaky.
    Flaky,
}

impl TestResultKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Flaky => "flaky",
        }
    }

    pub(super) fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            "flaky" => Ok(Self::Flaky),
            _ => Err("invalid TestResultKind"),
        }
    }
}

impl From<IndividualTestResultKind> for TestResultKind {
    fn from(val: IndividualTestResultKind) -> Self {
        match val {
            IndividualTestResultKind::Passed => Self::Passed,
            IndividualTestResultKind::Failed => Self::Failed,
            IndividualTestResultKind::Skipped { .. } => Self::Skipped,
        }
    }
}

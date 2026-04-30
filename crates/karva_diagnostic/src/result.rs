use std::fmt::Debug;
use std::time::Instant;
use std::{collections::HashMap, fmt};

use colored::Colorize;
use karva_logging::time::format_duration_bracketed;
use karva_python_semantic::{QualifiedFunctionName, QualifiedTestName};
use ruff_db::diagnostic::Diagnostic;
use serde::de::{self, MapAccess};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Visitor};

use crate::reporter::Reporter;

/// Represents the result of a test run.
///
/// This is held in the test context and updated throughout the test run.
#[derive(Debug, Clone, Default)]
pub struct TestRunResult {
    /// Diagnostics generated during test discovery.
    discovery_diagnostics: Vec<Diagnostic>,

    /// Diagnostics generated during test collection and  execution.
    diagnostics: Vec<Diagnostic>,

    /// Stats generated during test execution.
    stats: TestResultStats,

    durations: HashMap<QualifiedFunctionName, std::time::Duration>,

    /// Names of tests that failed during this run.
    failed_tests: Vec<QualifiedFunctionName>,
}

impl TestRunResult {
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn discovery_diagnostics(&self) -> &[Diagnostic] {
        &self.discovery_diagnostics
    }

    pub fn add_discovery_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.discovery_diagnostics.push(diagnostic);
    }

    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn stats(&self) -> &TestResultStats {
        &self.stats
    }

    pub fn register_test_case_result(
        &mut self,
        test_case_name: &QualifiedTestName,
        result: IndividualTestResultKind,
        duration: std::time::Duration,
        reporter: Option<&dyn Reporter>,
    ) {
        self.stats.add(result.clone().into());

        let function_name = test_case_name.function_name().clone();

        if matches!(result, IndividualTestResultKind::Failed) {
            self.failed_tests.push(function_name.clone());
        }

        if let Some(reporter) = reporter {
            reporter.report_test_case_result(test_case_name, result, duration);
        }

        self.durations
            .entry(function_name)
            .and_modify(|existing_duration| *existing_duration += duration)
            .or_insert(duration);
    }

    /// Record that a test had to be retried. Called once per test that needed
    /// at least one retry, in addition to its final pass/fail registration.
    pub fn mark_retried(&mut self) {
        self.stats.add(TestResultKind::Retried);
    }

    /// Forward a per-attempt failure notification to the reporter without
    /// touching summary stats. Stats are only updated for the final outcome.
    pub fn report_retry_attempt(
        &self,
        test_case_name: &QualifiedTestName,
        attempt: u32,
        duration: std::time::Duration,
        reporter: Option<&dyn Reporter>,
    ) {
        if let Some(reporter) = reporter {
            reporter.report_retry_attempt(test_case_name, attempt, duration);
        }
    }

    #[must_use]
    pub fn into_sorted(mut self) -> Self {
        self.diagnostics.sort_by(Diagnostic::ruff_start_ordering);
        self
    }

    pub fn durations(&self) -> &HashMap<QualifiedFunctionName, std::time::Duration> {
        &self.durations
    }

    pub fn failed_tests(&self) -> &[QualifiedFunctionName] {
        &self.failed_tests
    }
}

#[derive(Debug, Clone)]
pub enum IndividualTestResultKind {
    Passed,
    Failed,
    Skipped { reason: Option<String> },
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum TestResultKind {
    Passed,
    Failed,
    Skipped,
    /// A test that was retried at least once. Tracked alongside (not instead
    /// of) the test's final `Passed`/`Failed` outcome so the summary can
    /// report how many tests needed retries to succeed.
    Retried,
}

impl TestResultKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Retried => "retried",
        }
    }

    fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            "retried" => Ok(Self::Retried),
            _ => Err("invalid TestResultKind"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestResultStats {
    inner: HashMap<TestResultKind, usize>,
}

impl TestResultStats {
    /// Total number of tests run. `Retried` is a marker on a test's final
    /// outcome (Passed/Failed) and is not counted as a separate test.
    pub fn total(&self) -> usize {
        self.passed() + self.failed() + self.skipped()
    }

    pub fn is_success(&self) -> bool {
        self.failed() == 0
    }

    fn get(&self, kind: TestResultKind) -> usize {
        self.inner.get(&kind).copied().unwrap_or(0)
    }

    pub fn merge(&mut self, other: &Self) {
        for (kind, count) in &other.inner {
            self.inner
                .entry(*kind)
                .and_modify(|v| *v += count)
                .or_insert(*count);
        }
    }

    pub fn passed(&self) -> usize {
        self.get(TestResultKind::Passed)
    }

    pub fn failed(&self) -> usize {
        self.get(TestResultKind::Failed)
    }

    pub fn skipped(&self) -> usize {
        self.get(TestResultKind::Skipped)
    }

    pub fn retried(&self) -> usize {
        self.get(TestResultKind::Retried)
    }

    pub fn add(&mut self, kind: TestResultKind) {
        self.inner.entry(kind).and_modify(|v| *v += 1).or_insert(1);
    }

    pub fn display(&self, start_time: Instant) -> DisplayTestResultStats<'_> {
        DisplayTestResultStats::new(self, start_time)
    }
}

impl Serialize for TestResultStats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.inner.len()))?;
        for (kind, count) in &self.inner {
            map.serialize_entry(kind.as_str(), count)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for TestResultStats {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StatsVisitor;

        impl<'de> Visitor<'de> for StatsVisitor {
            type Value = TestResultStats;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map of test result kinds to counts")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut inner = HashMap::new();

                while let Some((key, value)) = access.next_entry::<String, usize>()? {
                    let kind = TestResultKind::from_str(&key).map_err(|_| {
                        de::Error::unknown_field(&key, &["passed", "failed", "skipped", "retried"])
                    })?;
                    inner.insert(kind, value);
                }

                Ok(TestResultStats { inner })
            }
        }

        deserializer.deserialize_map(StatsVisitor)
    }
}

pub struct DisplayTestResultStats<'a> {
    stats: &'a TestResultStats,
    start_time: Instant,
}

impl<'a> DisplayTestResultStats<'a> {
    fn new(stats: &'a TestResultStats, start_time: Instant) -> Self {
        Self { stats, start_time }
    }
}

impl std::fmt::Display for DisplayTestResultStats<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let success = self.stats.is_success();
        let elapsed = self.start_time.elapsed();

        writeln!(f, "{}", "─".repeat(12))?;

        let label = format!("{:>12}", "Summary");
        if success {
            write!(f, "{}", label.green().bold())?;
        } else {
            write!(f, "{}", label.red().bold())?;
        }

        let mut parts = vec![
            format!("{} passed", self.stats.passed())
                .green()
                .bold()
                .to_string(),
        ];
        if self.stats.failed() > 0 {
            parts.push(
                format!("{} failed", self.stats.failed())
                    .red()
                    .bold()
                    .to_string(),
            );
        }
        parts.push(
            format!("{} skipped", self.stats.skipped())
                .yellow()
                .bold()
                .to_string(),
        );
        if self.stats.retried() > 0 {
            parts.push(
                format!("{} retried", self.stats.retried())
                    .yellow()
                    .bold()
                    .to_string(),
            );
        }

        writeln!(
            f,
            " {} {} {} run: {}",
            format_duration_bracketed(elapsed),
            self.stats.total(),
            if self.stats.total() == 1 {
                "test"
            } else {
                "tests"
            },
            parts.join(", "),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_roundtrip() {
        let mut stats = TestResultStats::default();
        stats.add(TestResultKind::Passed);
        stats.add(TestResultKind::Passed);
        stats.add(TestResultKind::Failed);
        stats.add(TestResultKind::Skipped);

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: TestResultStats = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.passed(), 2);
        assert_eq!(deserialized.failed(), 1);
        assert_eq!(deserialized.skipped(), 1);
        assert_eq!(deserialized.total(), 4);
    }

    #[test]
    fn test_deserialize_empty() {
        let stats: TestResultStats = serde_json::from_str("{}").unwrap();
        assert_eq!(stats.passed(), 0);
        assert_eq!(stats.failed(), 0);
        assert_eq!(stats.skipped(), 0);
    }

    #[test]
    fn test_deserialize_partial() {
        let stats: TestResultStats = serde_json::from_str(r#"{"passed": 5}"#).unwrap();
        assert_eq!(stats.passed(), 5);
        assert_eq!(stats.failed(), 0);
        assert_eq!(stats.skipped(), 0);
    }

    #[test]
    fn test_deserialize_unknown_field() {
        let result = serde_json::from_str::<TestResultStats>(r#"{"invalid": 1}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_merge() {
        let mut a = TestResultStats::default();
        a.add(TestResultKind::Passed);

        let mut b = TestResultStats::default();
        b.add(TestResultKind::Passed);
        b.add(TestResultKind::Failed);

        a.merge(&b);
        assert_eq!(a.passed(), 2);
        assert_eq!(a.failed(), 1);
    }

    #[test]
    fn test_is_success() {
        let mut stats = TestResultStats::default();
        assert!(stats.is_success());

        stats.add(TestResultKind::Passed);
        assert!(stats.is_success());

        stats.add(TestResultKind::Failed);
        assert!(!stats.is_success());
    }
}

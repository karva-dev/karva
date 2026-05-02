use std::collections::HashMap;
use std::fmt;
use std::time::Instant;

use colored::Colorize;
use karva_logging::time::format_duration_bracketed;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::kind::TestResultKind;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestResultStats {
    inner: HashMap<TestResultKind, usize>,
}

impl TestResultStats {
    /// Total number of tests run. `Flaky` is a marker on a passing test and
    /// is not counted as a separate test.
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

    pub fn flaky(&self) -> usize {
        self.get(TestResultKind::Flaky)
    }

    pub fn slow(&self) -> usize {
        self.get(TestResultKind::Slow)
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
                        de::Error::unknown_field(
                            &key,
                            &["passed", "failed", "skipped", "flaky", "slow"],
                        )
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

impl fmt::Display for DisplayTestResultStats<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let success = self.stats.is_success();
        let elapsed = self.start_time.elapsed();

        writeln!(f, "{}", "─".repeat(12))?;

        let label = format!("{:>12}", "Summary");
        if success {
            write!(f, "{}", label.green().bold())?;
        } else {
            write!(f, "{}", label.red().bold())?;
        }

        let passed_text = if self.stats.flaky() > 0 {
            format!(
                "{} passed ({} flaky)",
                self.stats.passed(),
                self.stats.flaky()
            )
        } else {
            format!("{} passed", self.stats.passed())
        };
        let mut parts = vec![passed_text.green().bold().to_string()];
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
        if self.stats.slow() > 0 {
            parts.push(
                format!("{} slow", self.stats.slow())
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

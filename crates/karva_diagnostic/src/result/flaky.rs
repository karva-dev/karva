use std::fmt;
use std::time::Duration;

use colored::Colorize;
use karva_logging::time::format_duration_bracketed;
use karva_python_semantic::QualifiedTestName;
use serde::{Deserialize, Serialize};

/// A test that passed only after one or more retries. Held in worker
/// memory; serialized as [`FlakyTestRecord`] when persisted to the cache.
#[derive(Debug, Clone)]
pub struct FlakyTest {
    pub test_name: QualifiedTestName,
    pub passed_on: u32,
    pub total_attempts: u32,
    pub duration: Duration,
}

impl FlakyTest {
    pub fn to_record(&self) -> FlakyTestRecord {
        FlakyTestRecord {
            module_name: self
                .test_name
                .function_name()
                .module_path()
                .module_name()
                .to_string(),
            function_name: self.test_name.function_name().function_name().to_string(),
            params: self.test_name.params().map(str::to_string),
            passed_on: self.passed_on,
            total_attempts: self.total_attempts,
            duration: self.duration,
        }
    }
}

/// Serializable form of [`FlakyTest`] used by the cache and aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlakyTestRecord {
    pub module_name: String,
    pub function_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<String>,
    pub passed_on: u32,
    pub total_attempts: u32,
    pub duration: Duration,
}

impl FlakyTestRecord {
    /// Returns a value that formats this record as a single
    /// `   FLAKY M/T [duration] module::name(params)` line.
    pub fn display(&self) -> DisplayFlakyTestRecord<'_> {
        DisplayFlakyTestRecord(self)
    }
}

/// `Display` wrapper for one [`FlakyTestRecord`].
pub struct DisplayFlakyTestRecord<'a>(&'a FlakyTestRecord);

impl fmt::Display for DisplayFlakyTestRecord<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let record = self.0;
        let label = format!("FLAKY {}/{}", record.passed_on, record.total_attempts);
        let padding = " ".repeat(12usize.saturating_sub(label.len()));
        let colored_label = label.yellow().bold();
        let duration_str = format_duration_bracketed(record.duration);
        let module = record.module_name.cyan();
        let fn_name = record.function_name.blue().bold();
        let params = record
            .params
            .as_deref()
            .map(|p| p.blue().bold().to_string())
            .unwrap_or_default();

        writeln!(
            f,
            "{padding}{colored_label} {duration_str} {module}::{fn_name}{params}"
        )
    }
}

/// `Display` wrapper for a slice of [`FlakyTestRecord`]s. Renders each
/// record on its own line; emits nothing for an empty slice.
pub struct DisplayFlakyTestRecords<'a>(&'a [FlakyTestRecord]);

impl<'a> DisplayFlakyTestRecords<'a> {
    pub fn new(records: &'a [FlakyTestRecord]) -> Self {
        Self(records)
    }
}

impl fmt::Display for DisplayFlakyTestRecords<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for record in self.0 {
            write!(f, "{}", record.display())?;
        }
        Ok(())
    }
}

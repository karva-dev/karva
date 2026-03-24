use std::fmt::Write;
use std::time::Duration;

use colored::Colorize;
use karva_logging::Printer;
use karva_logging::time::format_duration_bracketed;
use karva_python_semantic::QualifiedTestName;

use crate::result::IndividualTestResultKind;

/// A reporter for test execution time logging to the user.
pub trait Reporter: Send + Sync {
    /// Report the completion of a given test.
    fn report_test_case_result(
        &self,
        test_name: &QualifiedTestName,
        result_kind: IndividualTestResultKind,
        duration: Duration,
    );
}

/// A no-op implementation of [`Reporter`].
#[derive(Default)]
pub struct DummyReporter;

impl Reporter for DummyReporter {
    fn report_test_case_result(
        &self,
        _test_name: &QualifiedTestName,
        _result_kind: IndividualTestResultKind,
        _duration: Duration,
    ) {
    }
}

/// A reporter that outputs test results to stdout as they complete.
pub struct TestCaseReporter {
    printer: Printer,
}

impl TestCaseReporter {
    pub fn new(printer: Printer) -> Self {
        Self { printer }
    }
}

impl Reporter for TestCaseReporter {
    fn report_test_case_result(
        &self,
        test_name: &QualifiedTestName,
        result_kind: IndividualTestResultKind,
        duration: Duration,
    ) {
        let mut stdout = self.printer.stream_for_test_result().lock();

        let (label, colored_label) = match &result_kind {
            IndividualTestResultKind::Passed => ("PASS", "PASS".green().bold().to_string()),
            IndividualTestResultKind::Failed => ("FAIL", "FAIL".red().bold().to_string()),
            IndividualTestResultKind::Skipped { .. } => {
                ("SKIP", "SKIP".yellow().bold().to_string())
            }
        };

        let padding = " ".repeat(12usize.saturating_sub(label.len()));
        let duration_str = format_duration_bracketed(duration);

        let module = test_name.function_name().module_path().module_name().cyan();
        let fn_name = test_name.function_name().function_name().blue().bold();
        let params = test_name.params().unwrap_or_default();

        let suffix = match &result_kind {
            IndividualTestResultKind::Skipped {
                reason: Some(reason),
            } => format!(": {reason}"),
            _ => String::new(),
        };

        writeln!(
            stdout,
            "{padding}{colored_label} {duration_str} {module}::{fn_name}{params}{suffix}"
        )
        .ok();
    }
}

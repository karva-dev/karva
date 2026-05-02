use karva_combine::Combine;
use karva_logging::{FinalStatusLevel, StatusLevel};
use serde::{Deserialize, Serialize};

use crate::filter::FiltersetSet;
use crate::max_fail::MaxFail;
use crate::options::{CovReport, OutputFormat};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunIgnoredMode {
    #[default]
    Default,
    Only,
    All,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum NoTestsMode {
    #[default]
    Auto,
    Pass,
    Warn,
    Fail,
}

impl Combine for NoTestsMode {
    #[inline(always)]
    fn combine_with(&mut self, _other: Self) {}

    #[inline]
    fn combine(self, _other: Self) -> Self {
        self
    }
}

#[derive(Default, Debug, Clone)]
pub struct ProjectSettings {
    pub(crate) terminal: TerminalSettings,
    pub(crate) src: SrcSettings,
    pub(crate) test: TestSettings,
    pub(crate) coverage: CoverageSettings,
}

impl ProjectSettings {
    pub fn terminal(&self) -> &TerminalSettings {
        &self.terminal
    }

    pub fn src(&self) -> &SrcSettings {
        &self.src
    }

    pub fn test(&self) -> &TestSettings {
        &self.test
    }

    pub fn coverage(&self) -> &CoverageSettings {
        &self.coverage
    }

    pub fn max_fail(&self) -> MaxFail {
        self.test.max_fail
    }

    pub fn set_filter(&mut self, filter: FiltersetSet) {
        self.test.filter = filter;
    }

    pub fn set_run_ignored(&mut self, mode: RunIgnoredMode) {
        self.test.run_ignored = mode;
    }
}

#[derive(Default, Debug, Clone)]
pub struct TerminalSettings {
    pub output_format: OutputFormat,
    pub show_python_output: bool,
    pub status_level: StatusLevel,
    pub final_status_level: FinalStatusLevel,
}

#[derive(Default, Debug, Clone)]
pub struct SrcSettings {
    pub respect_ignore_files: bool,
    pub include_paths: Vec<String>,
}

#[derive(Default, Debug, Clone)]
pub struct CoverageSettings {
    pub sources: Vec<String>,
    pub report: CovReport,
}

#[derive(Default, Debug, Clone)]
pub struct TestSettings {
    pub test_function_prefix: String,
    pub max_fail: MaxFail,
    pub try_import_fixtures: bool,
    pub retry: u32,
    pub filter: FiltersetSet,
    pub run_ignored: RunIgnoredMode,
    pub no_tests: NoTestsMode,
}

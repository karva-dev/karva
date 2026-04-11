use camino::Utf8PathBuf;
use karva_combine::Combine;
use karva_macros::{Combine, OptionsMetadata};
use ruff_db::diagnostic::DiagnosticFormat;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::filter::FiltersetSet;
use crate::max_fail::MaxFail;
use crate::settings::{
    ProjectSettings, RunIgnoredMode, SrcSettings, TerminalSettings, TestSettings,
};

#[derive(
    Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, OptionsMetadata, Combine,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Options {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option_group]
    pub src: Option<SrcOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option_group]
    pub terminal: Option<TerminalOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option_group]
    pub test: Option<TestOptions>,
}

impl Options {
    pub fn from_toml_str(content: &str) -> Result<Self, KarvaTomlError> {
        let options = toml::from_str(content)?;
        Ok(options)
    }

    pub fn to_settings(&self) -> ProjectSettings {
        ProjectSettings {
            terminal: self.terminal.clone().unwrap_or_default().to_settings(),
            src: self.src.clone().unwrap_or_default().to_settings(),
            test: self.test.clone().unwrap_or_default().to_settings(),
        }
    }

    pub(crate) fn from_karva_configuration_file(
        path: &Utf8PathBuf,
    ) -> Result<Self, KarvaTomlError> {
        let karva_toml_str =
            std::fs::read_to_string(path).map_err(|source| KarvaTomlError::FileReadError {
                source,
                path: path.clone(),
            })?;

        Self::from_toml_str(&karva_toml_str)
    }
}

#[derive(
    Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize, OptionsMetadata, Combine,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SrcOptions {
    /// Whether to automatically exclude files that are ignored by `.ignore`,
    /// `.gitignore`, `.git/info/exclude`, and global `gitignore` files.
    /// Enabled by default.
    #[option(
        default = r#"true"#,
        value_type = r#"bool"#,
        example = r#"
            respect-ignore-files = false
        "#
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub respect_ignore_files: Option<bool>,

    /// A list of files and directories to check.
    /// Including a file or directory will make it so that it (and its contents)
    /// are tested.
    ///
    /// - `tests` matches a directory named `tests`
    /// - `tests/test.py` matches a file named `test.py` in the `tests` directory
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option(
        default = r#"null"#,
        value_type = r#"list[str]"#,
        example = r#"
            include = ["tests"]
        "#
    )]
    pub include: Option<Vec<String>>,
}

impl SrcOptions {
    pub(crate) fn to_settings(&self) -> SrcSettings {
        SrcSettings {
            respect_ignore_files: self.respect_ignore_files.unwrap_or(true),
            include_paths: self.include.clone().unwrap_or_default(),
        }
    }
}

#[derive(
    Debug, Default, Clone, Eq, PartialEq, Combine, Serialize, Deserialize, OptionsMetadata,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TerminalOptions {
    /// The format to use for printing diagnostic messages.
    ///
    /// Defaults to `full`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option(
        default = r#"full"#,
        value_type = "full | concise",
        example = r#"
            output-format = "concise"
        "#
    )]
    pub output_format: Option<OutputFormat>,

    /// Whether to show the python output.
    ///
    /// This is the output the `print` goes to etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option(
        default = r#"true"#,
        value_type = "true | false",
        example = r#"
            show-python-output = false
        "#
    )]
    pub show_python_output: Option<bool>,
}

impl TerminalOptions {
    pub fn to_settings(&self) -> TerminalSettings {
        TerminalSettings {
            output_format: self.output_format.unwrap_or_default(),
            show_python_output: self.show_python_output.unwrap_or_default(),
        }
    }
}

#[derive(
    Debug, Default, Clone, Eq, PartialEq, Combine, Serialize, Deserialize, OptionsMetadata,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TestOptions {
    /// The prefix to use for test functions.
    ///
    /// Defaults to `test`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option(
        default = r#"test"#,
        value_type = "string",
        example = r#"
            test-function-prefix = "test"
        "#
    )]
    pub test_function_prefix: Option<String>,

    /// Whether to stop at the first test failure.
    ///
    /// This is a legacy alias for [`max_fail`](#test_max-fail): `true`
    /// corresponds to `max-fail = 1` and `false` leaves the limit unset.
    /// When both are set, `max-fail` takes precedence.
    ///
    /// Defaults to `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option(
        default = r#"false"#,
        value_type = "true | false",
        example = r#"
            fail-fast = true
        "#
    )]
    pub fail_fast: Option<bool>,

    /// Stop scheduling new tests once this many tests have failed.
    ///
    /// Accepts a positive integer. Omitting the field (the default) lets
    /// every test run regardless of how many fail. Setting `max-fail = 1`
    /// is equivalent to the legacy `fail-fast = true`.
    ///
    /// When both [`fail_fast`](#test_fail-fast) and `max-fail` are set,
    /// `max-fail` takes precedence.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option(
        default = "unlimited",
        value_type = "positive integer",
        example = r#"
            max-fail = 3
        "#
    )]
    pub max_fail: Option<MaxFail>,

    /// When set, we will try to import functions in each test file as well as parsing the ast to find them.
    ///
    /// This is often slower, so it is not recommended for most projects.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option(
        default = r#"false"#,
        value_type = "true | false",
        example = r#"
            try-import-fixtures = true
        "#
    )]
    pub try_import_fixtures: Option<bool>,

    /// When set, we will retry failed tests up to this number of times.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option(
        default = r#"0"#,
        value_type = "u32",
        example = r#"
            retry = 3
        "#
    )]
    pub retry: Option<u32>,
}

impl TestOptions {
    pub fn to_settings(&self) -> TestSettings {
        let max_fail = self
            .max_fail
            .or_else(|| self.fail_fast.map(MaxFail::from_fail_fast))
            .unwrap_or_default();

        TestSettings {
            test_function_prefix: self
                .test_function_prefix
                .clone()
                .unwrap_or_else(|| "test".to_string()),
            max_fail,
            try_import_fixtures: self.try_import_fixtures.unwrap_or_default(),
            retry: self.retry.unwrap_or_default(),
            filter: FiltersetSet::default(),
            run_ignored: RunIgnoredMode::default(),
        }
    }
}

#[derive(Error, Debug)]
pub enum KarvaTomlError {
    #[error(transparent)]
    TomlSyntax(#[from] toml::de::Error),
    #[error("Failed to read `{path}`: {source}")]
    FileReadError {
        #[source]
        source: std::io::Error,
        path: Utf8PathBuf,
    },
}

/// The diagnostic output format.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum OutputFormat {
    #[default]
    Full,

    Concise,
}

impl OutputFormat {
    /// Returns `true` if this format is intended for users to read directly, in contrast to
    /// machine-readable or structured formats.
    ///
    /// This can be used to check whether information beyond the diagnostics, such as a header or
    /// `Found N diagnostics` footer, should be included.
    pub fn is_human_readable(self) -> bool {
        matches!(self, Self::Full | Self::Concise)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Concise => "concise",
        }
    }
}

impl From<OutputFormat> for DiagnosticFormat {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Full => Self::Full,
            OutputFormat::Concise => Self::Concise,
        }
    }
}

impl Combine for OutputFormat {
    #[inline(always)]
    fn combine_with(&mut self, _other: Self) {}

    #[inline]
    fn combine(self, _other: Self) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use insta::{assert_debug_snapshot, assert_snapshot};
    use karva_combine::Combine;

    use super::*;

    #[test]
    fn to_settings_defaults_when_empty() {
        assert_debug_snapshot!(TestOptions::default().to_settings(), @r#"
        TestSettings {
            test_function_prefix: "test",
            max_fail: MaxFail(
                None,
            ),
            try_import_fixtures: false,
            retry: 0,
            filter: FiltersetSet {
                filters: [],
            },
        }
        "#);
    }

    #[test]
    fn to_settings_fail_fast_true_becomes_max_fail_one() {
        let options = TestOptions {
            fail_fast: Some(true),
            ..TestOptions::default()
        };
        assert_debug_snapshot!(options.to_settings().max_fail, @"
        MaxFail(
            Some(
                1,
            ),
        )
        ");
    }

    #[test]
    fn to_settings_fail_fast_false_is_unlimited() {
        let options = TestOptions {
            fail_fast: Some(false),
            ..TestOptions::default()
        };
        assert_debug_snapshot!(options.to_settings().max_fail, @"
        MaxFail(
            None,
        )
        ");
    }

    #[test]
    fn to_settings_max_fail_takes_precedence_over_fail_fast() {
        let options = TestOptions {
            fail_fast: Some(true),
            max_fail: Some(MaxFail::from(NonZeroU32::new(5).expect("non-zero"))),
            ..TestOptions::default()
        };
        assert_debug_snapshot!(options.to_settings().max_fail, @"
        MaxFail(
            Some(
                5,
            ),
        )
        ");
    }

    #[test]
    fn from_toml_str_parses_nested_sections() {
        let toml = r#"
[test]
test-function-prefix = "check"
max-fail = 3
retry = 2

[terminal]
output-format = "concise"
show-python-output = true

[src]
respect-ignore-files = false
include = ["tests", "more"]
"#;
        let options = Options::from_toml_str(toml).expect("parse");
        assert_debug_snapshot!(options.to_settings(), @r#"
        ProjectSettings {
            terminal: TerminalSettings {
                output_format: Concise,
                show_python_output: true,
            },
            src: SrcSettings {
                respect_ignore_files: false,
                include_paths: [
                    "tests",
                    "more",
                ],
            },
            test: TestSettings {
                test_function_prefix: "check",
                max_fail: MaxFail(
                    Some(
                        3,
                    ),
                ),
                try_import_fixtures: false,
                retry: 2,
                filter: FiltersetSet {
                    filters: [],
                },
            },
        }
        "#);
    }

    #[test]
    fn from_toml_str_rejects_unknown_key() {
        let toml = r"
[test]
fail-fast = true
nonsense = 42
";
        assert_snapshot!(
            Options::from_toml_str(toml).expect_err("unknown field"),
            @"
        TOML parse error at line 4, column 1
          |
        4 | nonsense = 42
          | ^^^^^^^^
        unknown field `nonsense`, expected one of `test-function-prefix`, `fail-fast`, `max-fail`, `try-import-fixtures`, `retry`
        "
        );
    }

    #[test]
    fn from_toml_str_rejects_unknown_top_level_section() {
        let toml = r"
[bogus]
foo = 1
";
        assert_snapshot!(
            Options::from_toml_str(toml).expect_err("unknown section"),
            @"
        TOML parse error at line 2, column 2
          |
        2 | [bogus]
          |  ^^^^^
        unknown field `bogus`, expected one of `src`, `terminal`, `test`
        "
        );
    }

    #[test]
    fn from_toml_str_empty_is_default() {
        assert_debug_snapshot!(Options::from_toml_str("").expect("parse"), @"
        Options {
            src: None,
            terminal: None,
            test: None,
        }
        ");
    }

    /// `MaxFail` wraps `NonZeroU32`, so raw `0` must be rejected by the
    /// deserializer rather than silently producing `unlimited`.
    #[test]
    fn from_toml_str_rejects_max_fail_zero() {
        let toml = r"
[test]
max-fail = 0
";
        assert_snapshot!(
            Options::from_toml_str(toml).expect_err("zero rejected"),
            @"
        TOML parse error at line 3, column 12
          |
        3 | max-fail = 0
          |            ^
        invalid value: integer `0`, expected a nonzero u32
        "
        );
    }

    #[test]
    fn combine_prefers_self_for_scalars() {
        let cli = TestOptions {
            test_function_prefix: Some("cli_prefix".to_string()),
            retry: Some(5),
            ..TestOptions::default()
        };
        let file = TestOptions {
            test_function_prefix: Some("file_prefix".to_string()),
            retry: Some(1),
            try_import_fixtures: Some(true),
            ..TestOptions::default()
        };
        assert_debug_snapshot!(cli.combine(file), @r#"
        TestOptions {
            test_function_prefix: Some(
                "cli_prefix",
            ),
            fail_fast: None,
            max_fail: None,
            try_import_fixtures: Some(
                true,
            ),
            retry: Some(
                5,
            ),
        }
        "#);
    }

    #[test]
    fn combine_fills_missing_fields_from_other() {
        let cli = TestOptions::default();
        let file = TestOptions {
            test_function_prefix: Some("from_file".to_string()),
            fail_fast: Some(true),
            retry: Some(3),
            ..TestOptions::default()
        };
        assert_debug_snapshot!(cli.combine(file), @r#"
        TestOptions {
            test_function_prefix: Some(
                "from_file",
            ),
            fail_fast: Some(
                true,
            ),
            max_fail: None,
            try_import_fixtures: None,
            retry: Some(
                3,
            ),
        }
        "#);
    }

    /// `Vec::combine` appends `self` after `other`, so CLI entries take
    /// precedence at the tail.
    #[test]
    fn combine_merges_include_paths_with_cli_taking_precedence() {
        let cli = SrcOptions {
            include: Some(vec!["cli_only".to_string()]),
            ..SrcOptions::default()
        };
        let file = SrcOptions {
            include: Some(vec!["file_only".to_string()]),
            respect_ignore_files: Some(false),
        };
        assert_debug_snapshot!(cli.combine(file), @r#"
        SrcOptions {
            respect_ignore_files: Some(
                false,
            ),
            include: Some(
                [
                    "file_only",
                    "cli_only",
                ],
            ),
        }
        "#);
    }

    #[test]
    fn project_overrides_apply_cli_over_file() {
        let cli_options = Options {
            test: Some(TestOptions {
                test_function_prefix: Some("cli".to_string()),
                ..TestOptions::default()
            }),
            ..Options::default()
        };
        let file_options = Options {
            test: Some(TestOptions {
                test_function_prefix: Some("file".to_string()),
                retry: Some(2),
                ..TestOptions::default()
            }),
            ..Options::default()
        };
        let overrides = ProjectOptionsOverrides::new(None, cli_options);
        assert_debug_snapshot!(overrides.apply_to(file_options).test, @r#"
        Some(
            TestOptions {
                test_function_prefix: Some(
                    "cli",
                ),
                fail_fast: None,
                max_fail: None,
                try_import_fixtures: None,
                retry: Some(
                    2,
                ),
            },
        )
        "#);
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct ProjectOptionsOverrides {
    pub config_file_override: Option<Utf8PathBuf>,
    pub options: Options,
}

impl ProjectOptionsOverrides {
    pub fn new(config_file_override: Option<Utf8PathBuf>, options: Options) -> Self {
        Self {
            config_file_override,
            options,
        }
    }

    pub fn apply_to(&self, options: Options) -> Options {
        self.options.clone().combine(options)
    }
}

use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use karva_combine::Combine;
use karva_logging::{FinalStatusLevel, StatusLevel};
use karva_macros::{Combine, OptionsMetadata};
use ruff_db::diagnostic::DiagnosticFormat;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::filter::FiltersetSet;
use crate::max_fail::MaxFail;
use crate::settings::{
    NoTestsMode, ProjectSettings, RunIgnoredMode, SrcSettings, TerminalSettings, TestSettings,
};

/// The implicit name of the default profile.
pub const DEFAULT_PROFILE: &str = "default";

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

    /// Named configuration profiles, selected with `--profile <name>` or the
    /// `KARVA_PROFILE` environment variable.
    ///
    /// Each profile may override `[src]`, `[terminal]`, and `[test]` settings.
    /// Selecting a non-default profile layers its overrides on top of the
    /// `[profile.default]` overrides (if any), which themselves layer on top
    /// of the top-level options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<BTreeMap<String, ProfileOptions>>,
}

impl Options {
    pub fn from_toml_str(content: &str) -> Result<Self, KarvaTomlError> {
        let options: Self = toml::from_str(content)?;
        if let Some(profiles) = &options.profile {
            validate_profile_names(profiles)?;
        }
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

    /// Returns true if `name` is defined as a profile in this configuration.
    /// The implicit `default` profile always exists.
    pub fn has_profile(&self, name: &str) -> bool {
        if name == DEFAULT_PROFILE {
            return true;
        }
        self.profile
            .as_ref()
            .is_some_and(|profiles| profiles.contains_key(name))
    }

    /// Resolve a profile by collapsing the `profile` map into the top-level
    /// option groups.
    ///
    /// The returned `Options` has its `profile` field cleared. The selected
    /// profile is layered on top of any `[profile.default]` overrides, which
    /// themselves layer on top of the top-level options. CLI options can then
    /// be combined with the result via the usual `Combine` precedence.
    ///
    /// Returns [`UnknownProfile`] when `name` is set to a profile that is
    /// not defined.
    pub fn resolve_profile(mut self, name: Option<&str>) -> Result<Self, UnknownProfile> {
        let requested = name.unwrap_or(DEFAULT_PROFILE);
        let profiles = self.profile.take();

        let Some(mut profiles) = profiles else {
            if requested != DEFAULT_PROFILE {
                return Err(UnknownProfile {
                    name: requested.to_string(),
                    available: vec![DEFAULT_PROFILE.to_string()],
                });
            }
            return Ok(self);
        };

        let default_overrides = profiles.remove(DEFAULT_PROFILE);
        let named_overrides = if requested == DEFAULT_PROFILE {
            None
        } else if let Some(p) = profiles.remove(requested) {
            Some(p)
        } else {
            let mut available: Vec<String> = profiles.into_keys().collect();
            available.push(DEFAULT_PROFILE.to_string());
            available.sort();
            available.dedup();
            return Err(UnknownProfile {
                name: requested.to_string(),
                available,
            });
        };

        if let Some(default_p) = default_overrides {
            self = default_p.into_options().combine(self);
        }
        if let Some(named_p) = named_overrides {
            self = named_p.into_options().combine(self);
        }
        Ok(self)
    }
}

/// The portion of [`Options`] that can appear inside a `[profile.<name>]` block.
///
/// Mirrors the top-level option groups but disallows nested `profile` tables.
#[derive(
    Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, OptionsMetadata, Combine,
)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProfileOptions {
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

impl ProfileOptions {
    fn into_options(self) -> Options {
        Options {
            src: self.src,
            terminal: self.terminal,
            test: self.test,
            profile: None,
        }
    }
}

fn validate_profile_names(
    profiles: &BTreeMap<String, ProfileOptions>,
) -> Result<(), KarvaTomlError> {
    for name in profiles.keys() {
        if name.is_empty() {
            return Err(KarvaTomlError::InvalidProfileName {
                name: name.clone(),
                reason: "profile name cannot be empty",
            });
        }
        if name != DEFAULT_PROFILE && name.starts_with("default-") {
            return Err(KarvaTomlError::InvalidProfileName {
                name: name.clone(),
                reason: "the `default-` prefix is reserved for built-in profiles",
            });
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(KarvaTomlError::InvalidProfileName {
                name: name.clone(),
                reason: "profile names may only contain ASCII letters, digits, `-`, and `_`",
            });
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
#[error(
    "profile `{name}` is not defined in configuration (available: {})",
    available.join(", ")
)]
pub struct UnknownProfile {
    pub name: String,
    pub available: Vec<String>,
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

    /// Test result statuses to display during the run.
    ///
    /// Modeled after `cargo-nextest`'s `--status-level`. Levels are
    /// cumulative: `pass` shows passing and failed tests, `skip` adds
    /// skipped tests on top, and so on. `retry` and `slow` are accepted
    /// for forward-compatibility but currently behave like `fail`.
    ///
    /// Defaults to `pass`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option(
        default = r#"pass"#,
        value_type = "none | fail | retry | slow | pass | skip | all",
        example = r#"
            status-level = "fail"
        "#
    )]
    pub status_level: Option<StatusLevel>,

    /// Test summary information to display at the end of the run.
    ///
    /// Modeled after `cargo-nextest`'s `--final-status-level`. Levels are
    /// cumulative in the same way as [`status_level`](#terminal_status-level).
    ///
    /// Defaults to `pass`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option(
        default = r#"pass"#,
        value_type = "none | fail | retry | slow | pass | skip | all",
        example = r#"
            final-status-level = "fail"
        "#
    )]
    pub final_status_level: Option<FinalStatusLevel>,
}

impl TerminalOptions {
    pub fn to_settings(&self) -> TerminalSettings {
        TerminalSettings {
            output_format: self.output_format.unwrap_or_default(),
            show_python_output: self.show_python_output.unwrap_or_default(),
            status_level: self.status_level.unwrap_or_default(),
            final_status_level: self.final_status_level.unwrap_or_default(),
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

    /// Configures behavior when no tests are found to run.
    ///
    /// `auto` (the default) fails when no filter expressions were given, and
    /// passes silently when filters were given. Use `fail` to always fail,
    /// `warn` to always warn, or `pass` to always succeed silently.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[option(
        default = r#"auto"#,
        value_type = "auto | pass | warn | fail",
        example = r#"
            no-tests = "warn"
        "#
    )]
    pub no_tests: Option<NoTestsMode>,
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
            no_tests: self.no_tests.unwrap_or_default(),
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
    #[error("invalid profile name `{name}`: {reason}")]
    InvalidProfileName { name: String, reason: &'static str },
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
        unknown field `nonsense`, expected one of `test-function-prefix`, `fail-fast`, `max-fail`, `try-import-fixtures`, `retry`, `no-tests`
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
        unknown field `bogus`, expected one of `src`, `terminal`, `test`, `profile`
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
            profile: None,
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
            no_tests: None,
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
            no_tests: None,
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
        assert_debug_snapshot!(overrides.apply_to(file_options).expect("resolves").test, @r#"
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
                no_tests: None,
            },
        )
        "#);
    }

    #[test]
    fn parse_profile_section() {
        let toml = r#"
[test]
test-function-prefix = "test"

[profile.ci.test]
retry = 5
no-tests = "fail"

[profile.ci.terminal]
output-format = "concise"
"#;
        let options = Options::from_toml_str(toml).expect("parse");
        assert_debug_snapshot!(options.has_profile("ci"), @"true");
        assert_debug_snapshot!(options.has_profile("default"), @"true");
        assert_debug_snapshot!(options.has_profile("missing"), @"false");
    }

    #[test]
    fn resolve_profile_layers_named_over_default_over_base() {
        let toml = r#"
[test]
test-function-prefix = "base"
retry = 1

[profile.default.test]
retry = 2
fail-fast = true

[profile.ci.test]
retry = 5
"#;
        let resolved = Options::from_toml_str(toml)
            .expect("parse")
            .resolve_profile(Some("ci"))
            .expect("resolves");
        assert_debug_snapshot!(resolved.test, @r#"
        Some(
            TestOptions {
                test_function_prefix: Some(
                    "base",
                ),
                fail_fast: Some(
                    true,
                ),
                max_fail: None,
                try_import_fixtures: None,
                retry: Some(
                    5,
                ),
                no_tests: None,
            },
        )
        "#);
        assert_debug_snapshot!(resolved.profile, @"None");
    }

    #[test]
    fn resolve_profile_default_applies_default_overrides() {
        let toml = r"
[test]
retry = 1

[profile.default.test]
retry = 9
";
        let resolved = Options::from_toml_str(toml)
            .expect("parse")
            .resolve_profile(None)
            .expect("resolves");
        assert_debug_snapshot!(resolved.test.unwrap().retry, @r"
        Some(
            9,
        )
        ");
    }

    #[test]
    fn resolve_profile_missing_profile_errors() {
        let toml = r"
[profile.ci.test]
retry = 5
";
        let err = Options::from_toml_str(toml)
            .expect("parse")
            .resolve_profile(Some("nope"))
            .expect_err("unknown");
        assert_snapshot!(
            err,
            @"profile `nope` is not defined in configuration (available: ci, default)"
        );
    }

    #[test]
    fn resolve_profile_default_when_no_profiles_defined_is_ok() {
        let options = Options::default();
        assert!(options.resolve_profile(None).is_ok());
    }

    #[test]
    fn resolve_profile_non_default_when_no_profiles_errors() {
        let options = Options::default();
        let err = options.resolve_profile(Some("ci")).expect_err("unknown");
        assert_snapshot!(
            err,
            @"profile `ci` is not defined in configuration (available: default)"
        );
    }

    #[test]
    fn from_toml_str_rejects_reserved_default_prefix() {
        let toml = r"
[profile.default-ci.test]
retry = 1
";
        assert_snapshot!(
            Options::from_toml_str(toml).expect_err("reserved"),
            @"invalid profile name `default-ci`: the `default-` prefix is reserved for built-in profiles"
        );
    }

    #[test]
    fn from_toml_str_rejects_invalid_profile_name_chars() {
        let toml = r#"
[profile."ci/fast".test]
retry = 1
"#;
        assert_snapshot!(
            Options::from_toml_str(toml).expect_err("invalid"),
            @"invalid profile name `ci/fast`: profile names may only contain ASCII letters, digits, `-`, and `_`"
        );
    }

    #[test]
    fn from_toml_str_rejects_nested_profile_table() {
        let toml = r"
[profile.ci.profile.nested.test]
retry = 1
";
        assert!(Options::from_toml_str(toml).is_err());
    }

    #[test]
    fn cli_overrides_win_over_resolved_profile() {
        let cli_options = Options {
            test: Some(TestOptions {
                retry: Some(99),
                ..TestOptions::default()
            }),
            ..Options::default()
        };
        let toml = r"
[profile.ci.test]
retry = 5
";
        let file_options = Options::from_toml_str(toml).expect("parse");
        let overrides =
            ProjectOptionsOverrides::new(None, cli_options).with_profile(Some("ci".to_string()));
        let resolved = overrides.apply_to(file_options).expect("resolves");
        assert_debug_snapshot!(resolved.test.unwrap().retry, @r"
        Some(
            99,
        )
        ");
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct ProjectOptionsOverrides {
    pub config_file_override: Option<Utf8PathBuf>,
    pub profile: Option<String>,
    pub options: Options,
}

impl ProjectOptionsOverrides {
    pub fn new(config_file_override: Option<Utf8PathBuf>, options: Options) -> Self {
        Self {
            config_file_override,
            profile: None,
            options,
        }
    }

    #[must_use]
    pub fn with_profile(mut self, profile: Option<String>) -> Self {
        self.profile = profile;
        self
    }

    /// Combine the file options with the CLI options, after first resolving
    /// the requested profile against the file options.
    pub fn apply_to(&self, options: Options) -> Result<Options, UnknownProfile> {
        let resolved = options.resolve_profile(self.profile.as_deref())?;
        Ok(self.options.clone().combine(resolved))
    }
}

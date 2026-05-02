use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use karva_combine::Combine;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::filter::{EvalContext, FilterError, Filterset};

/// Per-group concurrency configuration.
///
/// Modeled after [nextest's test groups](https://nexte.st/docs/configuration/test-groups/).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TestGroupOptions {
    /// Maximum number of workers that may run tests from this group at once.
    ///
    /// `1` enforces serial execution within the group; tests in other groups
    /// (or in no group) continue to run in parallel.
    pub max_threads: NonZeroUsize,
}

/// A profile-level override that assigns matching tests to a `test-group`.
///
/// Overrides are evaluated in order; the first one whose `filter` matches a
/// given test *and* sets `test-group` wins. Filter expressions use the same
/// DSL as the `--filter` CLI flag, except that the `group(...)` predicate is
/// rejected here to avoid circular references (a test's group cannot depend
/// on a filter that itself depends on the test's group).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct OverrideOptions {
    pub filter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_group: Option<String>,
}

/// A list of overrides with prepend-on-combine semantics.
///
/// Prepending makes higher-priority profile overrides (named profile)
/// evaluate before lower-priority ones (default profile) under
/// first-match-wins semantics.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OverridesList(pub Vec<OverrideOptions>);

/// Wrapper used purely for the TOML encoding, which cannot represent a
/// top-level array of tables.
#[derive(Debug, Serialize, Deserialize)]
struct OverridesEnvelope {
    #[serde(default)]
    overrides: Vec<OverrideOptions>,
}

impl OverridesList {
    /// Serialize to TOML for forwarding from the main process to workers via
    /// the `--group-overrides` CLI argument. TOML is used in preference to
    /// JSON because `karva_metadata` already depends on `toml`; using it
    /// avoids pulling `serde_json` into the dependency graph.
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(&OverridesEnvelope {
            overrides: self.0.clone(),
        })
    }

    /// Deserialize from the TOML form produced by [`Self::to_toml_string`].
    pub fn from_toml_str(input: &str) -> Result<Vec<OverrideOptions>, toml::de::Error> {
        Ok(toml::from_str::<OverridesEnvelope>(input)?.overrides)
    }
}

impl Combine for OverridesList {
    fn combine_with(&mut self, other: Self) {
        let mut merged = std::mem::take(&mut self.0);
        merged.extend(other.0);
        self.0 = merged;
    }
}

/// Errors raised when validating `[test-groups]` and `[[profile.*.overrides]]`.
#[derive(Debug, Error)]
pub enum TestGroupsError {
    #[error("invalid test-group name `{name}`: {reason}")]
    InvalidGroupName { name: String, reason: &'static str },
    #[error(
        "override #{index} references unknown test-group `{group}` (defined groups: {available})"
    )]
    UnknownGroup {
        index: usize,
        group: String,
        available: String,
    },
    #[error("override #{index} has invalid filter `{filter}`: {source}")]
    InvalidFilter {
        index: usize,
        filter: String,
        #[source]
        source: Box<FilterError>,
    },
    #[error(
        "override #{index} filter `{filter}` uses `group(...)`; overrides cannot reference test-groups in their own filter"
    )]
    GroupPredicateInOverride { index: usize, filter: String },
}

pub(crate) fn validate_group_names(
    groups: &BTreeMap<String, TestGroupOptions>,
) -> Result<(), TestGroupsError> {
    for name in groups.keys() {
        validate_group_name(name)?;
    }
    Ok(())
}

fn validate_group_name(name: &str) -> Result<(), TestGroupsError> {
    if name.is_empty() {
        return Err(TestGroupsError::InvalidGroupName {
            name: name.to_string(),
            reason: "test-group name cannot be empty",
        });
    }
    if name.starts_with('@') {
        return Err(TestGroupsError::InvalidGroupName {
            name: name.to_string(),
            reason: "the `@` prefix is reserved for built-in test-groups",
        });
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(TestGroupsError::InvalidGroupName {
            name: name.to_string(),
            reason: "test-group names may only contain ASCII letters, digits, `-`, and `_`",
        });
    }
    Ok(())
}

/// Compiled override + group lookup.
///
/// Maps a test (by name + tags) to its resolved group, if any. Built once
/// per run and shared between the main process (for partitioning) and
/// workers (for `group(...)` filter eval).
#[derive(Debug, Default, Clone)]
pub struct TestGroupResolver {
    compiled: Vec<CompiledOverride>,
}

#[derive(Debug, Clone)]
struct CompiledOverride {
    filterset: Filterset,
    /// `None` for overrides that match tests but do not assign a group; such
    /// overrides are skipped during group resolution rather than terminating
    /// the search, so a later override can still set the group.
    group: Option<String>,
}

impl TestGroupResolver {
    /// Build a resolver from already-validated overrides without re-checking
    /// that referenced groups exist. Used by workers, which receive overrides
    /// as JSON from the main process — group existence has already been
    /// validated there.
    pub fn from_validated_overrides(
        overrides: &[OverrideOptions],
    ) -> Result<Self, TestGroupsError> {
        Self::build(overrides, None)
    }

    /// Build a resolver from overrides + the configured `[test-groups]` table.
    /// Errors if any override references a group not present in `groups`, has
    /// an unparsable filter, or uses the `group(...)` predicate in its filter.
    pub fn new(
        overrides: &[OverrideOptions],
        groups: &BTreeMap<String, TestGroupOptions>,
    ) -> Result<Self, TestGroupsError> {
        Self::build(overrides, Some(groups))
    }

    fn build(
        overrides: &[OverrideOptions],
        groups: Option<&BTreeMap<String, TestGroupOptions>>,
    ) -> Result<Self, TestGroupsError> {
        let mut compiled = Vec::with_capacity(overrides.len());
        for (index, ov) in overrides.iter().enumerate() {
            if let (Some(groups), Some(group)) = (groups, ov.test_group.as_deref())
                && !groups.contains_key(group)
            {
                let mut available: Vec<&str> = groups.keys().map(String::as_str).collect();
                available.sort_unstable();
                return Err(TestGroupsError::UnknownGroup {
                    index,
                    group: group.to_string(),
                    available: available.join(", "),
                });
            }
            let filterset =
                Filterset::new(&ov.filter).map_err(|source| TestGroupsError::InvalidFilter {
                    index,
                    filter: ov.filter.clone(),
                    source: Box::new(source),
                })?;
            if filterset.uses_group_predicate() {
                return Err(TestGroupsError::GroupPredicateInOverride {
                    index,
                    filter: ov.filter.clone(),
                });
            }
            compiled.push(CompiledOverride {
                filterset,
                group: ov.test_group.clone(),
            });
        }
        Ok(Self { compiled })
    }

    pub fn is_empty(&self) -> bool {
        self.compiled.is_empty()
    }

    /// Returns the test-group assigned to a test by the first override that
    /// both matches and sets a `test-group`. Overrides that match but do not
    /// set a group are skipped, so they cannot accidentally shadow later
    /// assignments — mirroring nextest's per-setting first-match-wins model.
    pub fn resolve(&self, test_name: &str, tags: &[&str]) -> Option<&str> {
        let ctx = EvalContext {
            test_name,
            tags,
            group: None,
        };
        self.compiled
            .iter()
            .filter(|ov| ov.group.is_some() && ov.filterset.matches(&ctx))
            .find_map(|ov| ov.group.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group_opts(max: usize) -> TestGroupOptions {
        TestGroupOptions {
            max_threads: NonZeroUsize::new(max).expect("non-zero"),
        }
    }

    #[test]
    fn resolver_first_match_wins() {
        let mut groups = BTreeMap::new();
        groups.insert("database".to_string(), group_opts(4));
        groups.insert("serial".to_string(), group_opts(1));

        let overrides = vec![
            OverrideOptions {
                filter: "tag(serial)".to_string(),
                test_group: Some("serial".to_string()),
            },
            OverrideOptions {
                filter: "tag(db)".to_string(),
                test_group: Some("database".to_string()),
            },
        ];
        let resolver = TestGroupResolver::new(&overrides, &groups).expect("build");
        assert_eq!(resolver.resolve("x", &["serial", "db"]), Some("serial"));
        assert_eq!(resolver.resolve("x", &["db"]), Some("database"));
        assert_eq!(resolver.resolve("x", &[]), None);
    }

    #[test]
    fn resolver_skips_overrides_without_test_group() {
        let mut groups = BTreeMap::new();
        groups.insert("database".to_string(), group_opts(4));

        // An override that matches but assigns no group should NOT shadow
        // the later override that does assign one.
        let overrides = vec![
            OverrideOptions {
                filter: "tag(slow)".to_string(),
                test_group: None,
            },
            OverrideOptions {
                filter: "tag(slow)".to_string(),
                test_group: Some("database".to_string()),
            },
        ];
        let resolver = TestGroupResolver::new(&overrides, &groups).expect("build");
        assert_eq!(resolver.resolve("x", &["slow"]), Some("database"));
    }

    #[test]
    fn resolver_rejects_unknown_group() {
        let groups = BTreeMap::new();
        let overrides = vec![OverrideOptions {
            filter: "tag(slow)".to_string(),
            test_group: Some("missing".to_string()),
        }];
        let err = TestGroupResolver::new(&overrides, &groups).expect_err("unknown");
        assert!(matches!(err, TestGroupsError::UnknownGroup { .. }));
    }

    #[test]
    fn resolver_rejects_group_predicate_in_override() {
        let mut groups = BTreeMap::new();
        groups.insert("a".to_string(), group_opts(2));
        let overrides = vec![OverrideOptions {
            filter: "group(a)".to_string(),
            test_group: Some("a".to_string()),
        }];
        let err = TestGroupResolver::new(&overrides, &groups).expect_err("circular");
        assert!(matches!(
            err,
            TestGroupsError::GroupPredicateInOverride { .. }
        ));
    }

    /// `group` appearing inside a string or regex literal must not be
    /// confused with a `group(...)` predicate. The check walks the parsed
    /// AST instead of doing a lexical scan precisely so these cases work.
    #[test]
    fn resolver_accepts_literal_group_inside_other_predicates() {
        let mut groups = BTreeMap::new();
        groups.insert("a".to_string(), group_opts(2));
        for filter in [
            "tag(/group(.*)/)",
            "test(\"group(slow)\")",
            "tag(my_group)",
            "tag(grouped)",
            "test(=group)",
        ] {
            let overrides = vec![OverrideOptions {
                filter: filter.to_string(),
                test_group: Some("a".to_string()),
            }];
            TestGroupResolver::new(&overrides, &groups)
                .unwrap_or_else(|err| panic!("filter `{filter}` rejected: {err}"));
        }
    }

    #[test]
    fn resolver_rejects_filter_with_group_predicate() {
        let mut groups = BTreeMap::new();
        groups.insert("a".to_string(), group_opts(2));
        for filter in [
            "group(a)",
            " group ( a ) ",
            "tag(a) & group(b)",
            "not group(a)",
        ] {
            let overrides = vec![OverrideOptions {
                filter: filter.to_string(),
                test_group: Some("a".to_string()),
            }];
            let err = TestGroupResolver::new(&overrides, &groups)
                .err()
                .unwrap_or_else(|| panic!("filter `{filter}` accepted"));
            assert!(
                matches!(err, TestGroupsError::GroupPredicateInOverride { .. }),
                "filter `{filter}` produced unexpected error: {err}"
            );
        }
    }

    #[test]
    fn resolver_rejects_invalid_filter_syntax() {
        let groups = BTreeMap::new();
        let overrides = vec![OverrideOptions {
            filter: "tag(".to_string(),
            test_group: None,
        }];
        let err = TestGroupResolver::new(&overrides, &groups).expect_err("invalid filter");
        assert!(matches!(err, TestGroupsError::InvalidFilter { .. }));
    }

    #[test]
    fn from_validated_overrides_skips_existence_check() {
        let overrides = vec![OverrideOptions {
            filter: "tag(slow)".to_string(),
            test_group: Some("anything".to_string()),
        }];
        let resolver = TestGroupResolver::from_validated_overrides(&overrides).expect("build");
        assert_eq!(resolver.resolve("x", &["slow"]), Some("anything"));
    }

    #[test]
    fn overrides_list_combine_prepends_self() {
        let high = OverridesList(vec![OverrideOptions {
            filter: "tag(a)".to_string(),
            test_group: Some("g".to_string()),
        }]);
        let low = OverridesList(vec![OverrideOptions {
            filter: "tag(b)".to_string(),
            test_group: Some("g".to_string()),
        }]);
        let merged = high.combine(low);
        let names: Vec<&str> = merged.0.iter().map(|o| o.filter.as_str()).collect();
        assert_eq!(names, vec!["tag(a)", "tag(b)"]);
    }

    #[test]
    fn overrides_list_toml_round_trip() {
        let overrides = OverridesList(vec![
            OverrideOptions {
                filter: "tag(slow)".to_string(),
                test_group: Some("serial".to_string()),
            },
            OverrideOptions {
                filter: "tag(db)".to_string(),
                test_group: None,
            },
        ]);
        let encoded = overrides.to_toml_string().expect("serialize");
        let parsed = OverridesList::from_toml_str(&encoded).expect("parse");
        assert_eq!(parsed, overrides.0);
    }

    #[test]
    fn overrides_list_from_empty_string_yields_empty_list() {
        let parsed = OverridesList::from_toml_str("").expect("parse empty");
        assert!(parsed.is_empty());
    }

    #[test]
    fn validate_group_names_rejects_at_prefix() {
        let mut groups = BTreeMap::new();
        groups.insert("@global".to_string(), group_opts(1));
        let err = validate_group_names(&groups).expect_err("reserved");
        assert!(matches!(err, TestGroupsError::InvalidGroupName { .. }));
    }

    #[test]
    fn validate_group_names_rejects_empty() {
        let mut groups = BTreeMap::new();
        groups.insert(String::new(), group_opts(1));
        let err = validate_group_names(&groups).expect_err("empty");
        assert!(matches!(err, TestGroupsError::InvalidGroupName { .. }));
    }

    #[test]
    fn validate_group_names_rejects_invalid_chars() {
        let mut groups = BTreeMap::new();
        groups.insert("with space".to_string(), group_opts(1));
        let err = validate_group_names(&groups).expect_err("invalid");
        assert!(matches!(err, TestGroupsError::InvalidGroupName { .. }));
    }
}

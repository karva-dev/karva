# Profiles

Karva organizes configuration into named **profiles**, modeled after
[`cargo nextest`](https://nexte.st/docs/configuration/). A profile is a named
group of settings that tailors a test run for a particular context — fast local
iteration, CI, a soak run, and so on. At runtime you select the profile you
want with `--profile` (or `-P`) and Karva resolves it into the effective
settings for that run.

## Defining a profile

Profiles are declared as `[profile.<name>]` sections in `karva.toml`:

```toml
[profile.default.test]
test-function-prefix = "test"
retry = 1

[profile.ci.test]
retry = 5
no-tests = "fail"

[profile.ci.terminal]
output-format = "concise"
```

The same configuration in `pyproject.toml` lives under
`[tool.karva.profile.<name>]`:

```toml
[tool.karva.profile.default.test]
test-function-prefix = "test"
retry = 1

[tool.karva.profile.ci.test]
retry = 5
no-tests = "fail"
```

Every option group documented in [Configuration](configuration.md) — `src`,
`terminal`, `test` — may appear inside a profile. Top-level `[src]`, `[test]`,
or `[terminal]` tables (without `profile.<name>`) are not accepted.

## The default profile

The profile named `default` is always implicitly available. It is used when
no `--profile` is specified, and its settings form the base that every other
profile inherits from. You do not have to declare `[profile.default]` — an
empty configuration is equivalent to a `[profile.default]` with no overrides.

## Selecting a profile

A profile can be selected in three ways, in order of precedence:

1. The `--profile` (or `-P`) CLI flag: `karva test --profile ci`.
2. The `KARVA_PROFILE` environment variable: `KARVA_PROFILE=ci karva test`.
3. The implicit `default` profile when neither is set.

Selecting a profile that is not defined in the configuration produces an
error that lists the profiles that are available.

## Inheritance

A non-default profile is layered on top of `[profile.default]`: any field the
named profile does not set falls back to the value from `default`, which in
turn falls back to Karva's built-in defaults. Concretely, given:

```toml
[profile.default.test]
test-function-prefix = "test"
retry = 1

[profile.ci.test]
retry = 5
```

`karva test --profile ci` runs with `test-function-prefix = "test"` (inherited)
and `retry = 5` (overridden).

## Overriding a profile from the CLI

CLI flags always win over the resolved profile. This means a profile can
encode the common case while flags handle one-off tweaks:

```bash
# `[profile.ci]` says retry = 5, but this run uses retry = 0.
karva test --profile ci --retry 0
```

## Profile name rules

Profile names may contain ASCII letters, digits, `-`, and `_`. The
`default-` prefix is reserved for built-in profiles that may be added in the
future, so user-defined profiles must use a different prefix.

## Examples

### Fast local runs

```toml
[profile.default.test]
fail-fast = true

[profile.default.terminal]
status-level = "fail"
final-status-level = "fail"
```

### Stricter CI runs

```toml
[profile.ci.test]
retry = 3
no-tests = "fail"

[profile.ci.terminal]
output-format = "concise"
```

Trigger with `karva test --profile ci` (or set `KARVA_PROFILE=ci` in the CI
environment).

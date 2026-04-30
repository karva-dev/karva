# Profiles

Karva organizes configuration into named **profiles**, modeled after
[`cargo nextest`](https://nexte.st/docs/configuration/). A profile is a named
group of settings that tailors a test run for a particular context — fast local
iteration, CI, a soak run, and so on.

Configuration lives in `karva.toml` (or the `[tool.karva]` table in
`pyproject.toml`); the path can be overridden with `--config-file`.

## Profiles

To use multiple sets of configuration, define `[profile.<name>]` sections in
`karva.toml`:

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
`[tool.karva.profile.<name>]`.

A profile is selected at runtime with `--profile <name>` (or `-P <name>`), or
by setting the `KARVA_PROFILE` environment variable. If neither is set, the
implicit `default` profile is used.

Every option group documented in [Configuration](configuration.md) — `src`,
`terminal`, `test` — may appear inside a profile. Top-level `[src]`,
`[terminal]`, or `[test]` tables (without `profile.<name>`) are not accepted.

> **Warning:** Avoid custom profile names that begin with `default-`. The
> `default-` prefix is reserved for built-in profiles that Karva may add in
> the future.

### Profile inheritance

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

`karva test --profile ci` runs with `test-function-prefix = "test"`
(inherited from `default`) and `retry = 5` (overridden by `ci`).

## Hierarchical configuration

When resolving a setting for a run, Karva checks the following sources from
highest to lowest priority. The first source that defines the field wins.

1. **Command-line arguments** (e.g. `--retry 3`, `--no-fail-fast`).
1. **Environment variables** (e.g. `KARVA_PROFILE`, `KARVA_NO_TESTS`).
1. **The selected profile**, when not `default` (`[profile.<name>]`).
1. **The default profile** (`[profile.default]`).
1. **Built-in defaults** compiled into Karva.

Selecting a profile that is not defined in the configuration produces an
error that lists the profiles that are available.

## See also

- [Configuration](configuration.md) — reference for every supported field.
- [CLI](cli.md) — every flag, including `--profile` and `--config-file`.

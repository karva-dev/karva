<!-- WARNING: This file is auto-generated (cargo dev generate-all). Update the doc comments on the 'Options' struct in 'crates/karva_project/src/metadata/options.rs' if you want to change anything here. -->

# Configuration
## `src`

### `include`

A list of files and directories to check.
Including a file or directory will make it so that it (and its contents)
are tested.

- `tests` matches a directory named `tests`
- `tests/test.py` matches a file named `test.py` in the `tests` directory

**Default value**: `null`

**Type**: `list[str]`

**Example usage** (`pyproject.toml`):

```toml
[tool.karva.src]
include = ["tests"]
```

---

### `respect-ignore-files`

Whether to automatically exclude files that are ignored by `.ignore`,
`.gitignore`, `.git/info/exclude`, and global `gitignore` files.
Enabled by default.

**Default value**: `true`

**Type**: `bool`

**Example usage** (`pyproject.toml`):

```toml
[tool.karva.src]
respect-ignore-files = false
```

---

## `terminal`

### `final-status-level`

Test summary information to display at the end of the run.

Modeled after `cargo-nextest`'s `--final-status-level`. Levels are
cumulative in the same way as [`status_level`](#terminal_status-level).

Defaults to `pass`.

**Default value**: `pass`

**Type**: `none | fail | retry | slow | pass | skip | all`

**Example usage** (`pyproject.toml`):

```toml
[tool.karva.terminal]
final-status-level = "fail"
```

---

### `output-format`

The format to use for printing diagnostic messages.

Defaults to `full`.

**Default value**: `full`

**Type**: `full | concise`

**Example usage** (`pyproject.toml`):

```toml
[tool.karva.terminal]
output-format = "concise"
```

---

### `show-python-output`

Whether to show the python output.

This is the output the `print` goes to etc.

**Default value**: `true`

**Type**: `true | false`

**Example usage** (`pyproject.toml`):

```toml
[tool.karva.terminal]
show-python-output = false
```

---

### `status-level`

Test result statuses to display during the run.

Modeled after `cargo-nextest`'s `--status-level`. Levels are
cumulative: `pass` shows passing and failed tests, `skip` adds
skipped tests on top, and so on. `retry` and `slow` are accepted
for forward-compatibility but currently behave like `fail`.

Defaults to `pass`.

**Default value**: `pass`

**Type**: `none | fail | retry | slow | pass | skip | all`

**Example usage** (`pyproject.toml`):

```toml
[tool.karva.terminal]
status-level = "fail"
```

---

## `test`

### `fail-fast`

Whether to stop at the first test failure.

This is a legacy alias for [`max_fail`](#test_max-fail): `true`
corresponds to `max-fail = 1` and `false` leaves the limit unset.
When both are set, `max-fail` takes precedence.

Defaults to `false`.

**Default value**: `false`

**Type**: `true | false`

**Example usage** (`pyproject.toml`):

```toml
[tool.karva.test]
fail-fast = true
```

---

### `max-fail`

Stop scheduling new tests once this many tests have failed.

Accepts a positive integer. Omitting the field (the default) lets
every test run regardless of how many fail. Setting `max-fail = 1`
is equivalent to the legacy `fail-fast = true`.

When both [`fail_fast`](#test_fail-fast) and `max-fail` are set,
`max-fail` takes precedence.

**Default value**: `unlimited`

**Type**: `positive integer`

**Example usage** (`pyproject.toml`):

```toml
[tool.karva.test]
max-fail = 3
```

---

### `no-tests`

Configures behavior when no tests are found to run.

`auto` (the default) fails when no filter expressions were given, and
passes silently when filters were given. Use `fail` to always fail,
`warn` to always warn, or `pass` to always succeed silently.

**Default value**: `auto`

**Type**: `auto | pass | warn | fail`

**Example usage** (`pyproject.toml`):

```toml
[tool.karva.test]
no-tests = "warn"
```

---

### `retry`

When set, we will retry failed tests up to this number of times.

**Default value**: `0`

**Type**: `u32`

**Example usage** (`pyproject.toml`):

```toml
[tool.karva.test]
retry = 3
```

---

### `test-function-prefix`

The prefix to use for test functions.

Defaults to `test`.

**Default value**: `test`

**Type**: `string`

**Example usage** (`pyproject.toml`):

```toml
[tool.karva.test]
test-function-prefix = "test"
```

---

### `try-import-fixtures`

When set, we will try to import functions in each test file as well as parsing the ast to find them.

This is often slower, so it is not recommended for most projects.

**Default value**: `false`

**Type**: `true | false`

**Example usage** (`pyproject.toml`):

```toml
[tool.karva.test]
try-import-fixtures = true
```

---


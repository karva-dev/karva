# Failing fast

By default, Karva runs every test in the suite even after some have failed, so a single broken assertion does not hide the rest of the failures. Two flags change that.

## Stopping after the first failure

`--fail-fast` stops scheduling new tests once any test fails:

```bash
karva test --fail-fast
```

It is equivalent to `--max-fail=1`, kept around as the familiar pytest spelling.

## Stopping after N failures

`--max-fail=N` is the general form: stop scheduling new tests once `N` have failed.

```bash
karva test --max-fail=3
```

In-flight tests are allowed to finish, so the final failure count may exceed `N` by a small amount when running in parallel. Across workers, the limit is enforced through a shared file-based signal so workers stop scheduling cooperatively without racing.

## Configuring in `karva.toml`

```toml
[tool.karva.profile.default.test]
max-fail = 3
```

`fail-fast = true` is accepted as an alias for `max-fail = 1`. When both are set, `max-fail` wins.

## Forcing the suite to run

`--no-fail-fast` clears any `fail-fast` or `max-fail` value set in configuration and runs the entire suite:

```bash
karva test --no-fail-fast
```

When `--max-fail=N` and `--no-fail-fast` are both passed on the command line, `--max-fail` takes precedence. Use `--no-fail-fast` to override a CI profile on a one-off invocation when you want the full picture.

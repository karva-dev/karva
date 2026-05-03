# Retries

Some tests fail nondeterministically — a flaky network call, a timing-sensitive assertion, a race against a background task. Retrying lets the run continue without rewriting the test or quarantining it behind a tag.

## Enabling retries

Pass `--retry N` to retry each failed test up to `N` more times:

```bash
karva test --retry 3
```

```toml
[tool.karva.profile.default.test]
retry = 3
```

A test that passes on any attempt is treated as a pass. The first failed attempt is reported as `TRY 1 FAIL`; the test is only marked as failing once every attempt has been exhausted.

To see the per-attempt lines in the run output, raise the status level:

```bash
karva test --retry 3 --status-level=retry --final-status-level=retry
```

The summary line then includes a `N retried` counter so flake patterns are visible at a glance.

## Detecting attempts from inside a test

Tests can read `KARVA_ATTEMPT` (1-indexed) and `KARVA_TOTAL_ATTEMPTS` (`retries + 1`) from the environment:

```python
import os

def test_eventual_consistency():
    attempt = int(os.environ["KARVA_ATTEMPT"])
    if attempt == 1:
        # bail out fast on the first attempt
        ...
```

Both variables are always set, even when retries are disabled. See [Environment Variables](../../reference/env-vars.md) for the full list.

## When not to retry

Retries hide regressions. Reach for them on tests that are flaky for known infrastructure reasons; do not blanket-enable them across the suite to suppress real failures. If a test is flaky for reasons you control, prefer fixing the root cause.

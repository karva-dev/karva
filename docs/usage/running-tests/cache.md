# Cache

Karva keeps a small on-disk cache of previous test runs. Today it powers two things: per-test duration history (used to schedule the slowest tests first under parallelism) and the list of tests that failed in the last run.

The cache lives under the platform cache directory, namespaced per project root.

## Re-running just the failures

`--last-failed` (or `--lf`) restricts the run to whichever tests failed in the previous invocation:

```bash
karva test --last-failed
```

A typical fix-it-up loop:

```bash
karva test                # see the failures
karva test --last-failed  # iterate on just those
karva test                # confirm the full suite passes again
```

Combine with `--watch` to keep iterating until they all pass:

```bash
karva test --watch --last-failed
```

If the last run had no failures, `--last-failed` runs nothing.

## Disabling cache reads

`--no-cache` disables reading the cache for the current run. Tests are scheduled without duration hints and `--last-failed` becomes a no-op. Cache files are still written so subsequent runs without `--no-cache` have fresh data.

```bash
karva test --no-cache
```

## Managing the cache

Two `karva cache` subcommands manage cache contents directly:

```bash
karva cache prune  # keep only the most recent run
karva cache clean  # remove the cache directory entirely
```

`prune` is the safer of the two — it reclaims space without losing the data the next `--last-failed` would use. Reach for `clean` if the cache gets corrupted, or after upgrading karva across a cache-format change.

# Watch mode

`--watch` keeps Karva running, watches the project for Python source changes, and re-runs tests whenever a file is saved:

```bash
karva test --watch
```

It is the inner loop for local development: edit a file, hit save, see the result.

## What triggers a re-run

Karva watches Python source files under the project root. Edits to `.py` files queue a fresh run; edits to other files are ignored.

The watcher debounces rapid saves so a single editor write does not produce multiple runs.

## Combining with other flags

`--watch` composes with everything else. Two combinations are particularly useful:

Re-run only the tests that failed last time, then everything once they pass:

```bash
karva test --watch --last-failed
```

Tighten the loop when iterating on one test:

```bash
karva test --watch -E 'test(/^pkg::test_login$/)'
```

To exit, press `Ctrl-C`.

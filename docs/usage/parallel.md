# Parallel execution

Karva runs tests across multiple worker processes by default. Each worker is a separate Python interpreter, so tests are isolated from each other at the process level — a crash, signal, or interpreter-state mutation in one test cannot bleed into another.

## Worker count

The default is one worker per CPU core. Override with `-n` / `--num-workers`:

```bash
karva test -n 4
```

`--no-parallel` is a shorthand for `-n 1` and is what you want when debugging, attaching a debugger, or running tests that share resources you cannot partition:

```bash
karva test --no-parallel
```

## Partitioning shared resources

Workers do not coordinate. If your tests touch a shared resource — a database, a port, a temp directory — partition it on `KARVA_WORKER_ID` rather than locking:

```python
import os

@karva.fixture(scope="session")
def database_url():
    worker = os.environ["KARVA_WORKER_ID"]
    return f"postgresql:///test_{worker}"
```

`KARVA_WORKER_ID` is `0`-indexed and stable for the lifetime of the worker. See [Environment Variables](../env-vars.md) for the full list of variables the worker exposes to tests.

## Output capture

By default, stdout/stderr from a test is captured and emitted only when the test fails or when `--show-output` / `-s` is set. This keeps parallel output legible — without capture, output from concurrent tests would interleave on the terminal.

`--no-capture` disables capture entirely and forces a single worker, since uncaptured output from concurrent workers cannot safely interleave:

```bash
karva test --no-capture
```

Reach for `--no-capture` when debugging with `print` statements or attaching `pdb`. For ad-hoc inspection without giving up parallelism, prefer `-s` / `--show-output`, which keeps capture on but prints the captured output for every test.

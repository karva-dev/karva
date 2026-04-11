# Vendored third-party modules

This directory holds near-verbatim copies of code from other open-source
projects. Each file keeps the provenance of its source in its module-level
docstring, and the LICENSE file at the repository root carries the copyright
notice of every upstream project we vendor from.

## Upstream pin

| Module                     | Upstream                                              | Pinned commit |
| -------------------------- | ----------------------------------------------------- | ------------- |
| `_pytest_monkeypatch.py`   | `pytest-dev/pytest` — `src/_pytest/monkeypatch.py`    | `8ecf49ec2`   |
| `_pytest_recwarn.py`       | `pytest-dev/pytest` — `src/_pytest/recwarn.py`        | `8ecf49ec2`   |
| `_pytest_pathlib.py`       | `pytest-dev/pytest` — `src/_pytest/pathlib.py`        | `8ecf49ec2`   |
| `_pytest_tmpdir.py`        | `pytest-dev/pytest` — `src/_pytest/tmpdir.py`         | `8ecf49ec2`   |

All four files are MIT-licensed. Pytest's copyright notice is included in
`LICENSE` at the repository root under the "externally maintained libraries"
section.

## When to re-sync

Re-sync only for security fixes or correctness bugs that affect the pieces we
actually vendor. New pytest features do not need to come along for the ride.

## How to re-sync

1. Pick the target pytest commit hash.
2. Diff our vendored files against the upstream versions at that commit:

   ```sh
   cd ../pytest              # a local clone of pytest-dev/pytest
   git show <target>:src/_pytest/monkeypatch.py > /tmp/upstream_monkeypatch.py
   diff -u python/karva/_vendor/_pytest_monkeypatch.py /tmp/upstream_monkeypatch.py
   # repeat for the other three files
   ```

3. Replay the adaptations documented in each file's module-level docstring
   (rename, import fixes, type widening, `__repr__` additions, etc.).
4. Bump the pinned commit in this README and in every module docstring.
5. Run `just test` and `uvx prek run -a`.

## What must not change without re-syncing

- The `__all__` of each vendor module.
- The signatures of exported functions and classes.
- The control flow of exported methods. If you find a bug, fix it upstream
  first and re-sync, unless the bug is already fixed upstream and the
  adaptation is essentially a pick.
- The `ruff.lint.exclude` entry in the repository `pyproject.toml` that keeps
  these files out of ruff's `SIM`/`B` rules. Lint churn here would obscure the
  upstream diff; keep the files as close to verbatim as the type checker
  allows.

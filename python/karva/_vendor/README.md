# Vendored third-party modules

This directory holds near-verbatim copies of code from other open-source
projects. Each file keeps the provenance of its source in its module-level
docstring, and the LICENSE file at the repository root carries the copyright
notice of every upstream project we vendor from.

## Upstream pin

All modules below are vendored from `pytest-dev/pytest` at commit
`8ecf49ec2`:

- `_pytest_monkeypatch.py` from `src/_pytest/monkeypatch.py`
- `_pytest_recwarn.py` from `src/_pytest/recwarn.py`
- `_pytest_pathlib.py` from `src/_pytest/pathlib.py`
- `_pytest_tmpdir.py` from `src/_pytest/tmpdir.py`

All four files are MIT-licensed. Pytest's copyright notice is included in
`LICENSE` at the repository root under the "externally maintained libraries"
section.

## When to re-sync

Re-sync only for security fixes or correctness bugs that affect the pieces we
actually vendor. New pytest features do not need to come along for the ride.

## How to re-sync

1. Pick the target pytest commit hash.

1. Diff our vendored files against the upstream versions at that commit:

   ```sh
   cd ../pytest              # a local clone of pytest-dev/pytest
   git show <target>:src/_pytest/monkeypatch.py > /tmp/upstream_monkeypatch.py
   diff -u python/karva/_vendor/_pytest_monkeypatch.py /tmp/upstream_monkeypatch.py
   # repeat for the other three files
   ```

1. Replay the adaptations documented in each file's module-level docstring
   (rename, import fixes, type widening, `__repr__` additions, etc.).

1. Bump the pinned commit in this README and in every module docstring.

1. Run `just test` and `uvx prek run -a`.

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

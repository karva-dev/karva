# Tutorial

This tutorial walks through setting up a small project, writing a test, and running it.

## A new project

Initialise a project with `uv` and add a `tests/` directory:

```bash
uv init --lib calculator
cd calculator
mkdir tests
```

```text
.
├── pyproject.toml
├── README.md
├── src
│   └── calculator
│       ├── __init__.py
│       └── py.typed
└── tests
```

```python title="src/calculator/__init__.py"
class Calculator:
    def add(self, a: int, b: int) -> int:
        return a + b
```

```python title="tests/test_add.py"
from calculator import Calculator

def test_add():
    calculator = Calculator()
    assert calculator.add(1, 2) == 3
```

Add Karva as a dev dependency and run the suite:

```bash
uv add --dev karva
uv run karva test
```

## Where to next

- [Filtering Tests](usage/filtering.md) — pick which tests run with the `-E` filter DSL.
- [Fixtures](usage/fixtures/fixtures.md) — share setup and teardown between tests.
- [Snapshots](usage/snapshots.md) — pin large outputs to a file.
- [Coverage](usage/coverage.md) — measure line coverage with `--cov`.
- [Watch Mode](usage/watch.md) — re-run tests on save.

# Karva (0.0.1-alpha.4)

![PyPI - Version](https://img.shields.io/pypi/v/karva)

A Python test framework, written in Rust.

<div align="center">
  <img src="https://raw.githubusercontent.com/MatthewMckee4/karva/main/docs/assets/benchmark_results.svg" alt="Benchmark results" width="70%">
</div>

**We'd love for you to try Karva!** It's currently in alpha, and your feedback helps shape the project. [Get started](#getting-started) or join us on [Discord](https://discord.gg/XG95vNz4Zu).

## About Karva

Karva aims to be an efficient alternative to `pytest` and `unittest`.

Though, we do not want to support all of the features these tools support.
The main reason for this is that I don't believe all of the pytest features
promote good test quality, I think that the Rust testing ecosystem supports
writing high quality tests, some high quality Rust tests can be seen in the
[uv](https://github.com/astral-sh/uv) and [ruff](https://github.com/astral-sh/ruff)
repos. I believe that, with having a smaller set of testing features, we
can have a good impact on the Python ecosystem.

One example of a good test framework is [nextest](https://nexte.st), following
standards set in this repo is an aim of this project.

## Getting started

### Installation

Karva is available as [`karva`](https://pypi.org/project/karva/) on PyPI.

Use karva directly with `uvx`:

```bash
uvx karva test
uvx karva version
```

Or install karva with `uv`, or `pip`:

```bash
# With uv.
uv tool install karva@latest

# Add karva to your project.
uv add --dev karva

# With pip.
pip install karva
```

### Usage

By default, Karva will respect your `.gitignore` files when discovering tests in specified directories.

To run your tests, try any of the following:

```bash
# Run all tests.
karva test

# Run tests in a specific directory.
karva test tests/

# Run tests in a specific file.
karva test tests/test_example.py
```

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](https://github.com/MatthewMckee4/karva/blob/main/CONTRIBUTING.md) for more information.

You can also join us on [Discord](https://discord.gg/XG95vNz4Zu)

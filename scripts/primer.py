# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "rich",
#     "typer",
# ]
# ///
"""Primer: run karva against real-world Python projects to test compatibility.

Builds a fresh karva wheel from source via maturin, then clones a curated list
of popular Python projects, installs their dependencies, and runs karva against
each one to validate end-to-end compatibility.

Usage:
    uv run --script scripts/primer.py                            # build wheel + run all projects
    uv run --script scripts/primer.py --setup-only               # clone + install, skip running
    uv run --script scripts/primer.py --project httpx            # run a single project
    uv run --script scripts/primer.py -v                         # show full karva output
    uv run --script scripts/primer.py --wheel karva.whl          # use pre-built wheel
    uv run --script scripts/primer.py --wheel pr.whl \\
        --baseline-wheel main.whl --markdown-output out.md       # diff against baseline
"""

import enum
import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from enum import StrEnum
from pathlib import Path
from typing import Annotated, NamedTuple

import typer
from rich import box
from rich.console import Console
from rich.table import Table

console = Console()

ROOT = Path(__file__).parent.parent
PRIMER_DIR = ROOT / "target" / "primer_projects"

# Per-project karva run timeout in seconds.
KARVA_TIMEOUT = 120

# Number of times karva retries a failed test to reduce flakiness noise.
KARVA_RETRY = 10


class Verbosity(enum.Enum):
    NORMAL = "normal"
    VERBOSE = "verbose"

    @classmethod
    def from_int(cls, count: int) -> "Verbosity":
        return cls.VERBOSE if count > 0 else cls.NORMAL

    def is_normal(self) -> bool:
        return self == Verbosity.NORMAL


@dataclass
class Project:
    name: str
    repo: str
    commit: str
    test_paths: list[str]
    install_root: bool = False
    extras: list[str] = field(default_factory=list)
    # Extra packages to install beyond what uv sync provides (e.g. optional deps
    # declared in hatch envs rather than standard dependency-groups).
    extra_deps: list[str] = field(default_factory=list)
    # Set to False for projects where --all-extras causes dependency conflicts
    # (e.g. docs extras that restrict Python versions beyond the project minimum).
    all_extras: bool = True
    # Dependency groups to exclude from uv sync (e.g. ["docs"] for projects
    # where the docs group has Python version constraints that conflict).
    no_groups: list[str] = field(default_factory=list)
    # Skip uv sync entirely and use plain `uv pip install -e .` instead.
    # Needed for projects whose dependency groups can't be resolved by uv
    # (e.g. requires-python range wider than what some groups support).
    pip_only: bool = False


PROJECTS: list[Project] = [
    # --- Web / Networking ---
    Project(
        name="starlette",
        repo="https://github.com/encode/starlette",
        commit="a4148ec285825dd19102a45583f3ae27b4697031",
        test_paths=["tests/"],
    ),
    Project(
        name="fastapi",
        repo="https://github.com/fastapi/fastapi",
        commit="d128a7089a645466b789e32097de125a3b0f8979",
        test_paths=["tests/"],
    ),
    Project(
        name="flask",
        repo="https://github.com/pallets/flask",
        commit="7ef2946fb5151b745df30201b8c27790cac53875",
        test_paths=["tests/"],
    ),
    Project(
        name="werkzeug",
        repo="https://github.com/pallets/werkzeug",
        commit="9029a1ec49170a0a1f2908b7732ae9e4390c2ce3",
        test_paths=["tests/"],
    ),
    Project(
        name="aiohttp",
        repo="https://github.com/aio-libs/aiohttp",
        commit="5ed2e129ce41214adb76976e7ec43b8e639846ef",
        test_paths=["tests/"],
    ),
    Project(
        name="requests",
        repo="https://github.com/psf/requests",
        commit="bc04dfd6dad4cb02cd92f5daa81eb562d280a761",
        test_paths=["tests/"],
    ),
    Project(
        name="httpcore",
        repo="https://github.com/encode/httpcore",
        commit="10a658221deb38a4c5b16db55ab554b0bf731707",
        test_paths=["tests/"],
    ),
    Project(
        name="websockets",
        repo="https://github.com/python-websockets/websockets",
        commit="ea164d2fe0cb699dd52d28bfbe98165fb35cb13c",
        test_paths=["tests/"],
    ),
    Project(
        name="trio",
        repo="https://github.com/python-trio/trio",
        commit="82af3abcaf250aacc797b281e6815891766410fd",
        test_paths=["src/trio/_tests/"],
    ),
    # --- CLI / Terminal ---
    Project(
        name="typer",
        repo="https://github.com/fastapi/typer",
        commit="2966e4c5e584476e324a847c05e6ba17412031a1",
        test_paths=["tests/"],
    ),
    Project(
        name="click",
        repo="https://github.com/pallets/click",
        commit="cdab890e57a30a9f437b88ce9652f7bfce980c1f",
        test_paths=["tests/"],
    ),
    Project(
        name="rich",
        repo="https://github.com/Textualize/rich",
        commit="fc41075a3206d2a5fd846c6f41c4d2becab814fa",
        test_paths=["tests/"],
    ),
    Project(
        name="textual",
        repo="https://github.com/Textualize/textual",
        commit="04b03c8db64266a6a7811cc161bae9986e53b1a1",
        test_paths=["tests/"],
    ),
    Project(
        name="prompt-toolkit",
        repo="https://github.com/prompt-toolkit/python-prompt-toolkit",
        commit="940af53fa443073d9fdca26d5da6cfe6780f6ac9",
        test_paths=["tests/"],
    ),
    # --- Data / Utilities ---
    Project(
        name="pydantic",
        repo="https://github.com/pydantic/pydantic",
        commit="ac249284616890d91c746dc890fb0f6407df2843",
        test_paths=["tests/"],
    ),
    Project(
        name="arrow",
        repo="https://github.com/arrow-py/arrow",
        commit="b423717da81aaf8117313b4b377efaa6413a9639",
        test_paths=["tests/"],
    ),
    Project(
        name="more-itertools",
        repo="https://github.com/more-itertools/more-itertools",
        commit="9210d54527ddfa63ebe75cd5b5daa0201902c674",
        test_paths=["more_itertools/tests/"],
    ),
    Project(
        name="boltons",
        repo="https://github.com/mahmoud/boltons",
        commit="207651ee6055aabd0d9cdeac2e00140cdc208d44",
        test_paths=["tests/"],
    ),
    Project(
        name="toolz",
        repo="https://github.com/pytoolz/toolz",
        commit="568c2b8393973cd172a466546c9d95779c452438",
        test_paths=["tests/"],
    ),
    # --- Testing / Build / Packaging ---
    Project(
        name="pytest",
        repo="https://github.com/pytest-dev/pytest",
        commit="d46fb403bb5169b1f91db53689379e28161b1eba",
        test_paths=["testing/"],
    ),
    Project(
        name="packaging",
        repo="https://github.com/pypa/packaging",
        commit="c901ded1a6b97acee3b6b1eb17526228129c4645",
        test_paths=["tests/"],
    ),
    Project(
        name="pip",
        repo="https://github.com/pypa/pip",
        commit="fc9550be97a09d3752b7ee77791418f17c27bb6e",
        test_paths=["tests/"],
    ),
    Project(
        name="setuptools",
        repo="https://github.com/pypa/setuptools",
        commit="5a13876673a41e3cd21d4d6e587f53d0fb4fd8e5",
        test_paths=["setuptools/tests/"],
    ),
    Project(
        name="virtualenv",
        repo="https://github.com/pypa/virtualenv",
        commit="1bbeb9045b7075ba55684f9f601f64d8844fbf12",
        test_paths=["tests/"],
    ),
    Project(
        name="flit",
        repo="https://github.com/pypa/flit",
        commit="53634707f58358b94a80614f16b212f8df4c8f38",
        test_paths=["tests/"],
    ),
    Project(
        name="hatch",
        repo="https://github.com/pypa/hatch",
        commit="1e8f73c577c903436fc58d96c3de0fccae83a705",
        test_paths=["tests/backend/"],
    ),
    Project(
        name="tox",
        repo="https://github.com/tox-dev/tox",
        commit="0eda3a2840460521e8a0aeb45199fa890b7bba20",
        test_paths=["tests/"],
    ),
    Project(
        name="cibuildwheel",
        repo="https://github.com/pypa/cibuildwheel",
        commit="643b30c796cbdb68e5364c4e6bdd210e836bda49",
        test_paths=["unit_test/"],
    ),
    Project(
        name="build",
        repo="https://github.com/pypa/build",
        commit="7b7ae078aa1dabff33ea72d07ed15dd298acf80a",
        test_paths=["tests/"],
        pip_only=True,
    ),
    # --- Type Checking / Code Analysis ---
    Project(
        name="black",
        repo="https://github.com/psf/black",
        commit="9e969ddc31863a5c353b3f4e8f69d2aca05e36ae",
        test_paths=["tests/"],
        pip_only=True,
    ),
    Project(
        name="mypy",
        repo="https://github.com/python/mypy",
        commit="9790459eb33901e32ac9f0f2e2e332965bf4cad5",
        test_paths=["mypy/test/"],
    ),
    Project(
        name="pyflakes",
        repo="https://github.com/PyCQA/pyflakes",
        commit="59ec4593efd4c69ce00fdb13c40fcf5f3212ab10",
        test_paths=["pyflakes/test/"],
    ),
    Project(
        name="isort",
        repo="https://github.com/PyCQA/isort",
        commit="2fb94e188f3c0d8b3a593d437d58b0ce8bde4fca",
        test_paths=["tests/"],
    ),
    Project(
        name="pylint",
        repo="https://github.com/pylint-dev/pylint",
        commit="e039e7ba1c1ac1eed30e9067f434f30ac58189c8",
        test_paths=["tests/"],
    ),
    # --- Async ---
    Project(
        name="anyio",
        repo="https://github.com/agronholm/anyio",
        commit="96f0cf3cb9cd40c04b8effb2c4e14f67ff49a62c",
        test_paths=["tests/"],
    ),
    # --- Parsing / Serialization ---
    Project(
        name="jinja",
        repo="https://github.com/pallets/jinja",
        commit="5ef70112a1ff19c05324ff889dd30405b1002044",
        test_paths=["tests/"],
    ),
    Project(
        name="marshmallow",
        repo="https://github.com/marshmallow-code/marshmallow",
        commit="5d78e243f04d9cc07149e6ecdb9b987718ad480b",
        test_paths=["tests/"],
    ),
    Project(
        name="pyyaml",
        repo="https://github.com/yaml/pyyaml",
        commit="d51d8a138f7230834fc6e95635ff09ebd329185f",
        test_paths=["tests/lib/"],
    ),
    Project(
        name="tomlkit",
        repo="https://github.com/sdispater/tomlkit",
        commit="dd05eebc8ed9e30fc6c223088a5a450cb54c1cab",
        test_paths=["tests/"],
    ),
    Project(
        name="jsonschema",
        repo="https://github.com/python-jsonschema/jsonschema",
        commit="b747e59151ce8652e7860fc9e0639aa78676a5b1",
        test_paths=["tests/"],
    ),
    # --- Database / ORM ---
    Project(
        name="sqlalchemy",
        repo="https://github.com/sqlalchemy/sqlalchemy",
        commit="d3a8d4950e7f1c1cfcabc819e4b85f0bba61e26d",
        test_paths=["test/"],
    ),
    Project(
        name="peewee",
        repo="https://github.com/coleifer/peewee",
        commit="f8ff6af96cd8b0d3a303a5ec1d514b59837178d6",
        test_paths=["tests/"],
    ),
    Project(
        name="alembic",
        repo="https://github.com/sqlalchemy/alembic",
        commit="7b510dc52c7e931f393b6387f183bf888a08dee9",
        test_paths=["tests/"],
    ),
    # --- Security ---
    Project(
        name="pyjwt",
        repo="https://github.com/jpadilla/pyjwt",
        commit="40e3147eb5f790d8d041772e5fc00728a176c812",
        test_paths=["tests/"],
    ),
    Project(
        name="itsdangerous",
        repo="https://github.com/pallets/itsdangerous",
        commit="672971d66a2ef9f85151e53283113f33d642dabd",
        test_paths=["tests/"],
    ),
    # --- Documentation ---
    Project(
        name="mkdocs",
        repo="https://github.com/mkdocs/mkdocs",
        commit="2862536793b3c67d9d83c33e0dd6d50a791928f8",
        test_paths=["tests/"],
    ),
    Project(
        name="griffe",
        repo="https://github.com/mkdocstrings/griffe",
        commit="97106e4f56c99146f23864c7777e5bfaec89bafe",
        test_paths=["tests/"],
    ),
    # --- Data Classes / Structured Data ---
    Project(
        name="attrs",
        repo="https://github.com/python-attrs/attrs",
        commit="4885c5b1af4e9fa4d97b6bffa2fb78a2efa5f047",
        test_paths=["tests/"],
    ),
    Project(
        name="cattrs",
        repo="https://github.com/python-attrs/cattrs",
        commit="2ebbe303d48b8a31582796b346bb14645f69cd83",
        test_paths=["tests/"],
        extra_deps=["hypothesis"],
    ),
    # --- Logging / Observability ---
    Project(
        name="structlog",
        repo="https://github.com/hynek/structlog",
        commit="599fb22e271bbfa9c6951f26ea514b43ab7b2835",
        test_paths=["tests/"],
    ),
    Project(
        name="loguru",
        repo="https://github.com/Delgan/loguru",
        commit="2abeb0fa6d7be4b0455c6e0b580b1e9dab19005e",
        test_paths=["tests/"],
    ),
    Project(
        name="svcs",
        repo="https://github.com/hynek/svcs",
        commit="bfdc0b0fd960414d31948be1869daadaec45aefe",
        test_paths=["tests/"],
    ),
    # --- Miscellaneous / Utilities ---
    Project(
        name="humanize",
        repo="https://github.com/python-humanize/humanize",
        commit="ad74ae2ea0b51fa8613a44b5bc1859df7385c3db",
        test_paths=["tests/"],
    ),
    Project(
        name="python-dateutil",
        repo="https://github.com/dateutil/dateutil",
        commit="c981f9c7aa91b83cc9bd33a09ecee9e751b06e8d",
        test_paths=["dateutil/test/"],
    ),
    Project(
        name="faker",
        repo="https://github.com/joke2k/faker",
        commit="db42f6477ea15d754889a9e030b3c3d29872d947",
        test_paths=["tests/"],
    ),
    Project(
        name="tenacity",
        repo="https://github.com/jd/tenacity",
        commit="8779333a4759e56427b5d7ba23cacd3fe6054d61",
        test_paths=["tests/"],
    ),
    Project(
        name="cachetools",
        repo="https://github.com/tkem/cachetools",
        commit="5dce86fc5c9c565c6e9c912e2be5d6abb9586a1d",
        test_paths=["tests/"],
    ),
    Project(
        name="tqdm",
        repo="https://github.com/tqdm/tqdm",
        commit="75bdb6c379bcfc6c592b6342dc791a092b5d6ae0",
        test_paths=["tests/"],
    ),
    Project(
        name="tabulate",
        repo="https://github.com/astanin/python-tabulate",
        commit="268615a5c27dc40e5c22454c07b44d5c50410da0",
        test_paths=["test/"],
    ),
    Project(
        name="parse",
        repo="https://github.com/r1chardj0n3s/parse",
        commit="a285c6670773dcc3a2085b07fef281320a284a8e",
        test_paths=["test_parse.py"],
    ),
    Project(
        name="schedule",
        repo="https://github.com/dbader/schedule",
        commit="82a43db1b938d8fdf60103bd41f329e06c8d3651",
        test_paths=["test_schedule.py"],
    ),
    Project(
        name="python-dotenv",
        repo="https://github.com/theskumar/python-dotenv",
        commit="fa4e6a90b45428212452afc6ee0d5c8103b9301d",
        test_paths=["tests/"],
    ),
    Project(
        name="click-extra",
        repo="https://github.com/kdeldycke/click-extra",
        commit="98e204a6d5391e23f15b247668bea58340be5e84",
        test_paths=["tests/"],
    ),
    Project(
        name="pendulum",
        repo="https://github.com/sdispater/pendulum",
        commit="ae4c4052dc1aaf2614aa68d7ab8a3ca4396ec6aa",
        test_paths=["tests/"],
    ),
]


_ANSI_ESCAPE = re.compile(r"\x1b\[[0-9;]*m")
_TEST_RESULT_LINE = re.compile(r"\s+(PASS|FAIL|SKIP)\s+\[.*?\]\s+(\S+)")


class TestStats(NamedTuple):
    passed: int
    failed: int
    skipped: int


def parse_summary_line(line: str) -> TestStats | None:
    """Parse 'Summary [...] N tests run: X passed, Y failed, Z skipped' into TestStats."""
    if not line.startswith("Summary"):
        return None

    def extract(pattern: str) -> int:
        m = re.search(pattern, line)
        return int(m.group(1)) if m else 0

    return TestStats(
        passed=extract(r"(\d+) passed"),
        failed=extract(r"(\d+) failed"),
        skipped=extract(r"(\d+) skipped"),
    )


def parse_test_results(output: str) -> dict[str, str]:
    """Parse individual PASS/FAIL/SKIP lines from karva output.

    Returns a mapping of test id to status string.
    """
    results: dict[str, str] = {}
    for line in output.splitlines():
        clean = _ANSI_ESCAPE.sub("", line)
        if m := _TEST_RESULT_LINE.search(clean):
            results[m.group(2)] = m.group(1)
    return results


class KarvaResult(NamedTuple):
    exit_code: int
    test_stats: TestStats | None
    test_results: dict[str, str]


class ProjectRunStatus(StrEnum):
    PASS = "PASS"
    FAIL = "FAIL"
    TIMEOUT = "TIMEOUT"
    SETUP_OK = "SETUP_OK"
    SETUP_FAIL = "SETUP_FAIL"

    def style(self) -> str:
        match self:
            case ProjectRunStatus.PASS | ProjectRunStatus.SETUP_OK:
                return "green"
            case ProjectRunStatus.FAIL | ProjectRunStatus.SETUP_FAIL:
                return "red"
            case ProjectRunStatus.TIMEOUT:
                return "yellow"


@dataclass
class ProjectRunResult:
    project: str
    status: ProjectRunStatus
    exit_code: int | None = None
    error: str | None = None
    test_stats: TestStats | None = None
    test_results: dict[str, str] = field(default_factory=dict)

    def is_ok(self) -> bool:
        return self.status in (ProjectRunStatus.PASS, ProjectRunStatus.SETUP_OK)


@dataclass
class ProjectDiff:
    project: str
    baseline: ProjectRunResult
    current: ProjectRunResult

    @property
    def is_regression(self) -> bool:
        return self.baseline.is_ok() and not self.current.is_ok()

    @property
    def is_fix(self) -> bool:
        return not self.baseline.is_ok() and self.current.is_ok()

    @property
    def test_counts_changed(self) -> bool:
        return (
            self.baseline.test_stats is not None
            and self.current.test_stats is not None
            and self.baseline.test_stats != self.current.test_stats
        )

    @property
    def newly_failing(self) -> list[str]:
        """Tests that passed in the baseline but fail in the current run."""
        return sorted(
            t
            for t, s in self.current.test_results.items()
            if s == "FAIL" and self.baseline.test_results.get(t) == "PASS"
        )

    @property
    def newly_passing(self) -> list[str]:
        """Tests that failed in the baseline but pass in the current run."""
        return sorted(
            t
            for t, s in self.current.test_results.items()
            if s == "PASS" and self.baseline.test_results.get(t) == "FAIL"
        )

    @property
    def has_change(self) -> bool:
        return (
            self.baseline.status != self.current.status
            or self.test_counts_changed
            or bool(self.newly_failing)
            or bool(self.newly_passing)
        )


def compute_diff(
    baseline: list[ProjectRunResult], current: list[ProjectRunResult]
) -> list[ProjectDiff]:
    baseline_map = {r.project: r for r in baseline}
    diffs = []
    for r in current:
        if r.project not in baseline_map:
            continue
        d = ProjectDiff(r.project, baseline_map[r.project], r)
        if d.has_change:
            diffs.append(d)
    return diffs


def _change_description(d: ProjectDiff) -> str:
    if d.is_regression:
        return "Regression"
    if d.is_fix:
        return "Fixed"
    parts = []
    b, c = d.baseline.test_stats, d.current.test_stats
    if b and c:
        for attr in ("passed", "failed", "skipped"):
            delta = getattr(c, attr) - getattr(b, attr)
            if delta:
                parts.append(f"{'+' if delta > 0 else ''}{delta} {attr}")
    return ", ".join(parts) or "Changed"


def show_diff_table(diffs: list[ProjectDiff]) -> None:
    console.rule("[bold]Diff vs baseline[/bold]")
    if not diffs:
        console.print("  [green]No changes from baseline.[/green]")
        return

    table = Table(box=box.SIMPLE, show_header=True, header_style="bold dim")
    table.add_column("Project", style="bold")
    table.add_column("Baseline", justify="center")
    table.add_column("Current", justify="center")
    table.add_column("Change", justify="left")

    for d in diffs:
        b_style = "green" if d.baseline.is_ok() else "red"
        c_style = "green" if d.current.is_ok() else "red"
        change = _change_description(d)
        change_styled = (
            f"[red]{change}[/red]"
            if d.is_regression
            else (f"[green]{change}[/green]" if d.is_fix else change)
        )
        table.add_row(
            d.project,
            f"[{b_style}]{d.baseline.status}[/{b_style}]",
            f"[{c_style}]{d.current.status}[/{c_style}]",
            change_styled,
        )

    console.print(table)

    regressions = sum(1 for d in diffs if d.is_regression)
    fixes = sum(1 for d in diffs if d.is_fix)
    count_changes = len(diffs) - regressions - fixes
    parts: list[str] = []
    if regressions:
        parts.append(
            f"[red]{regressions} regression{'s' if regressions > 1 else ''}[/red]"
        )
    if fixes:
        parts.append(f"[green]{fixes} fix{'es' if fixes > 1 else ''}[/green]")
    if count_changes:
        parts.append(f"{count_changes} count change{'s' if count_changes > 1 else ''}")
    if parts:
        console.print("  " + ", ".join(parts))

    for d in diffs:
        if d.newly_failing:
            console.print(f"\n  [bold]{d.project}[/bold] — newly failing tests:")
            for t in d.newly_failing:
                console.print(f"    [red]✗[/red] {t}")
        if d.newly_passing:
            console.print(f"\n  [bold]{d.project}[/bold] — newly passing tests:")
            for t in d.newly_passing:
                console.print(f"    [green]✓[/green] {t}")


def _result_md(r: ProjectRunResult) -> str:
    if r.test_stats:
        return f"{r.status} ({r.test_stats.passed}p/{r.test_stats.failed}f)"
    return r.status


def write_markdown_comment(
    current: list[ProjectRunResult],
    diffs: list[ProjectDiff] | None,
    path: Path,
) -> None:
    """Write a GitHub-flavoured markdown PR comment to *path*."""
    lines: list[str] = ["<!-- primer-results -->", "## Primer Results\n"]

    lines += [
        "| Project | Status | Passed | Failed | Skipped |",
        "|---------|--------|--------|--------|---------|",
    ]
    for r in current:
        icon = (
            "✅"
            if r.is_ok()
            else (
                "❌"
                if r.status in (ProjectRunStatus.FAIL, ProjectRunStatus.SETUP_FAIL)
                else "⏱️"
            )
        )
        passed = str(r.test_stats.passed) if r.test_stats else ""
        failed = str(r.test_stats.failed) if r.test_stats else ""
        skipped = str(r.test_stats.skipped) if r.test_stats else ""
        lines.append(
            f"| {r.project} | {icon} {r.status} | {passed} | {failed} | {skipped} |"
        )

    lines.append("")

    if diffs is not None:
        if not diffs:
            lines.append("### ✅ No changes from baseline\n")
        else:
            regressions = [d for d in diffs if d.is_regression]
            fixes = [d for d in diffs if d.is_fix]
            count_changes = [d for d in diffs if not d.is_regression and not d.is_fix]
            heading_parts: list[str] = []
            if regressions:
                heading_parts.append(
                    f"{len(regressions)} regression{'s' if len(regressions) > 1 else ''}"
                )
            if fixes:
                heading_parts.append(
                    f"{len(fixes)} fix{'es' if len(fixes) > 1 else ''}"
                )
            if count_changes:
                heading_parts.append(
                    f"{len(count_changes)} count change{'s' if len(count_changes) > 1 else ''}"
                )
            icon = "⚠️" if regressions else "💡"
            lines.append(
                f"### {icon} Changes from baseline: {', '.join(heading_parts)}\n"
            )
            lines += [
                "| Project | Baseline | Current | Change |",
                "|---------|---------|---------|--------|",
            ]
            for d in diffs:
                b_icon = "✅" if d.baseline.is_ok() else "❌"
                c_icon = "✅" if d.current.is_ok() else "❌"
                change = _change_description(d)
                if d.is_regression:
                    change = f"🔴 {change}"
                elif d.is_fix:
                    change = f"🟢 {change}"
                lines.append(
                    f"| {d.project} | {b_icon} {_result_md(d.baseline)}"
                    f" | {c_icon} {_result_md(d.current)} | {change} |"
                )

            flaky_sections = [d for d in diffs if d.newly_failing or d.newly_passing]
            if flaky_sections:
                lines.append("")
                lines.append("<details>")
                lines.append("<summary>Test-level changes</summary>\n")
                for d in flaky_sections:
                    if d.newly_failing:
                        lines.append(f"**{d.project} — newly failing**\n")
                        for t in d.newly_failing:
                            lines.append(f"- `{t}`")
                        lines.append("")
                    if d.newly_passing:
                        lines.append(f"**{d.project} — newly passing**\n")
                        for t in d.newly_passing:
                            lines.append(f"- `{t}`")
                        lines.append("")
                lines.append("</details>")

    path.write_text("\n".join(lines) + "\n")


def clean_env() -> dict[str, str]:
    """Return os.environ without VIRTUAL_ENV so uv uses the project's own .venv."""
    return {k: v for k, v in os.environ.items() if k != "VIRTUAL_ENV"}


def build_wheel(verbosity: Verbosity) -> Path:
    """Delete stale wheels and build a fresh karva wheel via maturin."""
    wheels_dir = ROOT / "target" / "wheels"
    if wheels_dir.exists():
        shutil.rmtree(wheels_dir)
    console.print("[dim]\\[maturin] building karva wheel...[/dim]")
    capture = verbosity.is_normal()
    result = subprocess.run(
        ["uvx", "maturin", "build"],
        cwd=ROOT,
        capture_output=capture,
        check=False,
    )
    if result.returncode != 0:
        if capture:
            sys.stderr.write(result.stderr.decode())
        sys.exit("maturin build failed.")
    wheels = sorted(wheels_dir.glob("*.whl"), key=lambda p: p.stat().st_mtime)
    if not wheels:
        sys.exit("No wheel found after maturin build.")
    return wheels[-1]


def _clone(project: Project, project_dir: Path) -> None:
    console.print(f"  [dim]\\[git] cloning {project.repo}...[/dim]")
    project_dir.parent.mkdir(parents=True, exist_ok=True)
    # --filter=blob:none creates a partial clone (no blob data until checkout).
    subprocess.run(
        [
            "git",
            "clone",
            "--filter=blob:none",
            "--no-checkout",
            project.repo,
            str(project_dir),
        ],
        check=True,
        capture_output=True,
    )


def clone_or_update(project: Project, project_dir: Path) -> None:
    if project_dir.exists():
        console.print(f"  [dim]\\[git] fetching {project.commit[:8]}...[/dim]")
        # Fetch all refs so the pinned commit is reachable (fetching by exact SHA
        # is unreliable with partial clones on GitHub).
        subprocess.run(
            ["git", "fetch", "origin"],
            cwd=project_dir,
            check=True,
            capture_output=True,
        )
    else:
        _clone(project, project_dir)

    result = subprocess.run(
        ["git", "checkout", project.commit],
        cwd=project_dir,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        # Checkout failed — the commit may be absent from a stale partial clone.
        # Delete the directory and do a fresh full clone.
        console.print("  [dim]\\[git] stale clone, recloning...[/dim]")
        shutil.rmtree(project_dir)
        _clone(project, project_dir)
        subprocess.run(
            ["git", "checkout", project.commit],
            cwd=project_dir,
            check=True,
            capture_output=True,
        )


def uv_sync(project: Project, project_dir: Path) -> None:
    if project.pip_only:
        # uv sync can't resolve when requires-python is wider than some groups support.
        # Fall back to creating a bare venv and installing the project via pip.
        console.print("  [dim]\\[uv] creating venv (pip-only mode)...[/dim]")
        venv = project_dir / ".venv"
        r = subprocess.run(
            ["uv", "venv", str(venv), "--python", "3.13", "--clear"],
            cwd=project_dir,
            capture_output=True,
            env=clean_env(),
            check=False,
        )
        if r.returncode != 0:
            raise RuntimeError(f"uv venv failed:\n{r.stderr.decode()}")
        r = subprocess.run(
            ["uv", "pip", "install", "-e", ".", "--python", str(venv)],
            cwd=project_dir,
            capture_output=True,
            env=clean_env(),
            check=False,
        )
        if r.returncode != 0:
            raise RuntimeError(f"uv pip install failed:\n{r.stderr.decode()}")
        return

    cmd = ["uv", "sync", "--python", "3.13"]
    if project.all_extras:
        cmd.append("--all-extras")
    for group in project.no_groups:
        cmd.extend(["--no-group", group])
    has_lock = (project_dir / "uv.lock").exists()
    if has_lock:
        cmd.append("--frozen")
    console.print(f"  [dim]\\[uv] syncing{'  (frozen)' if has_lock else ''}...[/dim]")
    result = subprocess.run(
        cmd, cwd=project_dir, capture_output=True, env=clean_env(), check=False
    )
    if result.returncode != 0 and has_lock:
        # Lockfile may be stale at this commit; retry without --frozen.
        console.print(
            "  [dim]\\[uv] frozen sync failed, retrying without --frozen...[/dim]"
        )
        cmd.remove("--frozen")
        result = subprocess.run(
            cmd, cwd=project_dir, capture_output=True, env=clean_env(), check=False
        )
    if result.returncode != 0:
        raise RuntimeError(f"uv sync failed:\n{result.stderr.decode()}")


def install_wheel(project_dir: Path, wheel: Path, extra_deps: list[str]) -> None:
    venv = project_dir / ".venv"
    console.print(
        f"  [dim]\\[uv] installing karva + pytest into {venv.relative_to(ROOT)}...[/dim]"
    )
    # Always install pytest so test files that `import pytest` can be imported,
    # even when the project declares it only in a hatch env (not a standard dep group).
    # Resolve to absolute path — uv pip install runs with cwd=project_dir.
    packages = [str(wheel.resolve()), "pytest", *extra_deps]
    result = subprocess.run(
        ["uv", "pip", "install", "--python", str(venv), *packages],
        cwd=project_dir,
        capture_output=True,
        env=clean_env(),
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"wheel install failed:\n{result.stderr.decode()}")


def karva_bin(project_dir: Path) -> Path:
    """Return the path to the karva executable inside the project's venv."""
    if sys.platform == "win32":
        return project_dir / ".venv" / "Scripts" / "karva.exe"
    return project_dir / ".venv" / "bin" / "karva"


def run_karva(project: Project, project_dir: Path, verbosity: Verbosity) -> KarvaResult:
    paths = " ".join(project.test_paths)
    console.print(f"  [dim]\\[karva] running tests at {paths}...[/dim]")
    try:
        result = subprocess.run(
            [
                str(karva_bin(project_dir)),
                "test",
                "--retry",
                str(KARVA_RETRY),
                *project.test_paths,
            ],
            cwd=project_dir,
            env=clean_env(),
            timeout=KARVA_TIMEOUT,
            capture_output=True,
            text=True,
            check=False,
        )
        if not verbosity.is_normal():
            if result.stdout:
                print(result.stdout, end="")
            if result.stderr:
                print(result.stderr, end="", file=sys.stderr)
        test_stats = None
        for line in reversed(result.stdout.splitlines()):
            test_stats = parse_summary_line(line.strip())
            if test_stats is not None:
                break
        return KarvaResult(
            exit_code=result.returncode,
            test_stats=test_stats,
            test_results=parse_test_results(result.stdout),
        )
    except subprocess.TimeoutExpired:
        console.print(f"  [yellow][karva] timed out after {KARVA_TIMEOUT}s[/yellow]")
        return KarvaResult(exit_code=-1, test_stats=None, test_results={})


def run_project(
    project: Project,
    wheel: Path,
    *,
    setup_only: bool,
    verbosity: Verbosity,
    silent: bool = False,
    skip_setup: bool = False,
) -> ProjectRunResult:
    """Run a single project through clone → sync → install → test.

    *silent* suppresses the section rule (used for the quiet baseline pass).
    *skip_setup* skips clone/sync and just reinstalls the wheel (second pass
    when the project was already set up by the baseline run).
    """
    project_dir = PRIMER_DIR / project.name
    if silent:
        console.print(f"  [dim]{project.name}...[/dim]")
    else:
        console.rule(f"[bold]{project.name}[/bold]")

    if not skip_setup:
        try:
            clone_or_update(project, project_dir)
            uv_sync(project, project_dir)
            install_wheel(project_dir, wheel, project.extra_deps)
        except Exception as exc:
            if not silent:
                console.print(f"  [red][ERROR][/red] {exc}")
            return ProjectRunResult(
                project.name, ProjectRunStatus.SETUP_FAIL, error=str(exc)
            )
    else:
        try:
            install_wheel(project_dir, wheel, project.extra_deps)
        except Exception as exc:
            return ProjectRunResult(
                project.name, ProjectRunStatus.SETUP_FAIL, error=str(exc)
            )

    if setup_only:
        return ProjectRunResult(project.name, ProjectRunStatus.SETUP_OK)

    run_verbosity = Verbosity.NORMAL if silent else verbosity
    karva_result = run_karva(project, project_dir, run_verbosity)
    if karva_result.exit_code == -1:
        return ProjectRunResult(project.name, ProjectRunStatus.TIMEOUT)
    status = (
        ProjectRunStatus.PASS if karva_result.exit_code == 0 else ProjectRunStatus.FAIL
    )
    return ProjectRunResult(
        project.name,
        status,
        exit_code=karva_result.exit_code,
        test_stats=karva_result.test_stats,
        test_results=karva_result.test_results,
    )


app = typer.Typer(
    help="Run karva against real-world Python projects to test compatibility.",
    add_completion=False,
)


@app.command()
def main(
    setup_only: Annotated[
        bool,
        typer.Option(
            "--setup-only", help="Clone and install only; skip running karva."
        ),
    ] = False,
    project: Annotated[
        str | None,
        typer.Option("--project", metavar="NAME", help="Run only the named project."),
    ] = None,
    verbose: Annotated[
        int,
        typer.Option(
            "--verbose",
            "-v",
            count=True,
            help="Stream full karva output (-v or -vvvv).",
        ),
    ] = 0,
    wheel: Annotated[
        Path | None,
        typer.Option(
            "--wheel", help="Pre-built wheel to install (skips maturin build)."
        ),
    ] = None,
    baseline_wheel: Annotated[
        Path | None,
        typer.Option(
            "--baseline-wheel",
            help="Baseline wheel to diff against; runs all projects twice.",
        ),
    ] = None,
    markdown_output: Annotated[
        Path | None,
        typer.Option(
            "--markdown-output", help="Write PR-comment markdown to this path."
        ),
    ] = None,
) -> None:
    verbosity = Verbosity.from_int(verbose)

    current_wheel = wheel if wheel is not None else build_wheel(verbosity)
    console.print(f"Using wheel: [bold]{current_wheel.name}[/bold]")
    if baseline_wheel is not None:
        console.print(f"Baseline wheel: [bold]{baseline_wheel.name}[/bold]")

    projects_to_run = PROJECTS
    if project:
        projects_to_run = [p for p in PROJECTS if p.name == project]
        if not projects_to_run:
            names = [p.name for p in PROJECTS]
            raise typer.BadParameter(
                f"Unknown project {project!r}. Available: {names}",
                param_hint="'--project'",
            )

    PRIMER_DIR.mkdir(parents=True, exist_ok=True)

    # Optional baseline pass — runs each project with the baseline wheel so we
    # can diff the results at the end.
    baseline_results: list[ProjectRunResult] | None = None
    if baseline_wheel is not None:
        console.rule("[dim]Baseline pass[/dim]")
        baseline_results = [
            run_project(
                proj, baseline_wheel, setup_only=False, verbosity=verbosity, silent=True
            )
            for proj in projects_to_run
        ]

    # Main pass — skip clone/sync for projects that set up cleanly in the
    # baseline pass (they're already cloned and synced; we just swap the wheel).
    baseline_map = (
        {r.project: r for r in baseline_results} if baseline_results is not None else {}
    )
    results: list[ProjectRunResult] = []
    for proj in projects_to_run:
        baseline_r = baseline_map.get(proj.name)
        skip_setup = (
            baseline_r is not None and baseline_r.status != ProjectRunStatus.SETUP_FAIL
        )
        result = run_project(
            proj,
            current_wheel,
            setup_only=setup_only,
            verbosity=verbosity,
            skip_setup=skip_setup,
        )
        results.append(result)

    console.rule("[bold]Summary[/bold]")

    table = Table(box=box.SIMPLE, show_header=True, header_style="bold dim")
    table.add_column("Project", style="bold")
    table.add_column("Status", justify="center")
    table.add_column("Passed", justify="right", style="green")
    table.add_column("Failed", justify="right", style="red")
    table.add_column("Skipped", justify="right", style="yellow")

    for r in results:
        style = r.status.style()
        status_str = f"[{style}]{r.status}[/{style}]"
        passed = str(r.test_stats.passed) if r.test_stats else ""
        failed = str(r.test_stats.failed) if r.test_stats else ""
        skipped = str(r.test_stats.skipped) if r.test_stats else ""
        table.add_row(r.project, status_str, passed, failed, skipped)

    console.print(table)

    passes = sum(
        1
        for r in results
        if r.status in (ProjectRunStatus.PASS, ProjectRunStatus.SETUP_OK)
    )
    fails = sum(1 for r in results if r.status == ProjectRunStatus.FAIL)
    timeouts = sum(1 for r in results if r.status == ProjectRunStatus.TIMEOUT)
    setup_fails = sum(1 for r in results if r.status == ProjectRunStatus.SETUP_FAIL)
    console.print(
        f"  [green]{passes} passed[/green], [red]{fails} failed[/red], "
        f"[yellow]{timeouts} timed out[/yellow], [red]{setup_fails} setup errors[/red]"
    )

    diffs: list[ProjectDiff] | None = None
    if baseline_results is not None:
        diffs = compute_diff(baseline_results, results)
        show_diff_table(diffs)

    if markdown_output is not None:
        write_markdown_comment(results, diffs, markdown_output)

    if diffs is not None:
        # In diff mode: only fail on regressions (a project that was OK is now broken).
        # Pre-existing failures and minor count fluctuations are not actionable here.
        if any(d.is_regression for d in diffs):
            raise typer.Exit(1)
    elif any(
        r.status
        in (
            ProjectRunStatus.FAIL,
            ProjectRunStatus.TIMEOUT,
            ProjectRunStatus.SETUP_FAIL,
        )
        for r in results
    ):
        raise typer.Exit(1)


if __name__ == "__main__":
    app()

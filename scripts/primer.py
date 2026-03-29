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
    Project(
        name="httpx",
        repo="https://github.com/encode/httpx",
        commit="b5addb64f0161ff6bfe94c124ef76f6a1fba5254",
        test_paths=["tests/"],
        extra_deps=["chardet"],
    ),
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
        name="typer",
        repo="https://github.com/fastapi/typer",
        commit="2966e4c5e584476e324a847c05e6ba17412031a1",
        test_paths=["tests/"],
    ),
    Project(
        name="rich",
        repo="https://github.com/Textualize/rich",
        commit="fc41075a3206d2a5fd846c6f41c4d2becab814fa",
        test_paths=["tests/"],
    ),
    Project(
        name="griffe",
        repo="https://github.com/mkdocstrings/griffe",
        commit="97106e4f56c99146f23864c7777e5bfaec89bafe",
        test_paths=["tests/"],
    ),
    Project(
        name="click",
        repo="https://github.com/pallets/click",
        commit="cdab890e57a30a9f437b88ce9652f7bfce980c1f",
        test_paths=["tests/"],
    ),
    Project(
        name="flask",
        repo="https://github.com/pallets/flask",
        commit="7ef2946fb5151b745df30201b8c27790cac53875",
        test_paths=["tests/"],
    ),
    Project(
        name="jinja",
        repo="https://github.com/pallets/jinja",
        commit="5ef70112a1ff19c05324ff889dd30405b1002044",
        test_paths=["tests/"],
    ),
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
    Project(
        name="structlog",
        repo="https://github.com/hynek/structlog",
        commit="599fb22e271bbfa9c6951f26ea514b43ab7b2835",
        test_paths=["tests/"],
    ),
    Project(
        name="packaging",
        repo="https://github.com/pypa/packaging",
        commit="c901ded1a6b97acee3b6b1eb17526228129c4645",
        test_paths=["tests/"],
    ),
    Project(
        name="black",
        repo="https://github.com/psf/black",
        commit="9e969ddc31863a5c353b3f4e8f69d2aca05e36ae",
        test_paths=["tests/"],
        # uv sync can't resolve: docs group pins sphinx==8.2.3 (Python>=3.11) but
        # requires-python is >=3.10, causing uv to fail when resolving for 3.10.
        pip_only=True,
    ),
    Project(
        name="nox",
        repo="https://github.com/wntrblm/nox",
        commit="4ff681f169c4043ce3b3d19cba1eadd66720bf1d",
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
        # uv sync can't resolve: docs group requires proselint>=0.16 (Python>=3.10) but
        # requires-python is >=3.9, causing uv to fail when resolving for 3.9.
        pip_only=True,
    ),
    Project(
        name="svcs",
        repo="https://github.com/hynek/svcs",
        commit="bfdc0b0fd960414d31948be1869daadaec45aefe",
        test_paths=["tests/"],
    ),
]


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


class KarvaResult(NamedTuple):
    exit_code: int
    test_stats: TestStats | None


@dataclass
class ProjectResult:
    project: str
    status: str  # PASS, FAIL, TIMEOUT, SETUP_OK, SETUP_FAIL
    exit_code: int | None = None
    error: str | None = None
    test_stats: TestStats | None = None


def _is_ok(r: ProjectResult) -> bool:
    return r.status in ("PASS", "SETUP_OK")


@dataclass
class ProjectDiff:
    project: str
    baseline: ProjectResult
    current: ProjectResult

    @property
    def is_regression(self) -> bool:
        return _is_ok(self.baseline) and not _is_ok(self.current)

    @property
    def is_fix(self) -> bool:
        return not _is_ok(self.baseline) and _is_ok(self.current)

    @property
    def test_counts_changed(self) -> bool:
        return (
            self.baseline.test_stats is not None
            and self.current.test_stats is not None
            and self.baseline.test_stats != self.current.test_stats
        )

    @property
    def has_change(self) -> bool:
        return self.baseline.status != self.current.status or self.test_counts_changed


def compute_diff(
    baseline: list[ProjectResult], current: list[ProjectResult]
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
        b_style = "green" if _is_ok(d.baseline) else "red"
        c_style = "green" if _is_ok(d.current) else "red"
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


def _result_md(r: ProjectResult) -> str:
    if r.test_stats:
        return f"{r.status} ({r.test_stats.passed}p/{r.test_stats.failed}f)"
    return r.status


def write_markdown_comment(
    current: list[ProjectResult],
    diffs: list[ProjectDiff] | None,
    path: Path,
) -> None:
    """Write a GitHub-flavoured markdown PR comment to *path*."""
    lines: list[str] = ["<!-- primer-results -->", "## Primer Results\n"]

    lines += [
        "| Project | Status | Passed | Failed | Skipped | Exit Code |",
        "|---------|--------|--------|--------|---------|-----------|",
    ]
    for r in current:
        icon = (
            "✅" if _is_ok(r) else ("❌" if r.status in ("FAIL", "SETUP_FAIL") else "⏱️")
        )
        passed = str(r.test_stats.passed) if r.test_stats else ""
        failed = str(r.test_stats.failed) if r.test_stats else ""
        skipped = str(r.test_stats.skipped) if r.test_stats else ""
        exit_code = str(r.exit_code) if r.exit_code is not None else ""
        lines.append(
            f"| {r.project} | {icon} {r.status} | {passed} | {failed} | {skipped} | {exit_code} |"
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
                b_icon = "✅" if _is_ok(d.baseline) else "❌"
                c_icon = "✅" if _is_ok(d.current) else "❌"
                change = _change_description(d)
                if d.is_regression:
                    change = f"🔴 {change}"
                elif d.is_fix:
                    change = f"🟢 {change}"
                lines.append(
                    f"| {d.project} | {b_icon} {_result_md(d.baseline)}"
                    f" | {c_icon} {_result_md(d.current)} | {change} |"
                )

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
            [str(karva_bin(project_dir)), "test", "--retry", "3", *project.test_paths],
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
        return KarvaResult(exit_code=result.returncode, test_stats=test_stats)
    except subprocess.TimeoutExpired:
        console.print(f"  [yellow][karva] timed out after {KARVA_TIMEOUT}s[/yellow]")
        return KarvaResult(exit_code=-1, test_stats=None)


def run_project(
    project: Project,
    wheel: Path,
    *,
    setup_only: bool,
    verbosity: Verbosity,
    silent: bool = False,
    skip_setup: bool = False,
) -> ProjectResult:
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
            return ProjectResult(project.name, "SETUP_FAIL", error=str(exc))
    else:
        try:
            install_wheel(project_dir, wheel, project.extra_deps)
        except Exception as exc:
            return ProjectResult(project.name, "SETUP_FAIL", error=str(exc))

    if setup_only:
        return ProjectResult(project.name, "SETUP_OK")

    run_verbosity = Verbosity.NORMAL if silent else verbosity
    karva_result = run_karva(project, project_dir, run_verbosity)
    if karva_result.exit_code == -1:
        return ProjectResult(project.name, "TIMEOUT")
    status = "PASS" if karva_result.exit_code == 0 else "FAIL"
    return ProjectResult(
        project.name,
        status,
        exit_code=karva_result.exit_code,
        test_stats=karva_result.test_stats,
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
    baseline_results: list[ProjectResult] | None = None
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
    results: list[ProjectResult] = []
    for proj in projects_to_run:
        baseline_r = baseline_map.get(proj.name)
        skip_setup = baseline_r is not None and baseline_r.status != "SETUP_FAIL"
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
    table.add_column("Exit Code", justify="right")

    status_styles = {
        "PASS": "green",
        "SETUP_OK": "green",
        "FAIL": "red",
        "TIMEOUT": "yellow",
        "SETUP_FAIL": "red",
    }

    for r in results:
        style = status_styles.get(r.status, "")
        status_str = f"[{style}]{r.status}[/{style}]" if style else r.status
        exit_code_str = str(r.exit_code) if r.exit_code is not None else ""
        passed = str(r.test_stats.passed) if r.test_stats else ""
        failed = str(r.test_stats.failed) if r.test_stats else ""
        skipped = str(r.test_stats.skipped) if r.test_stats else ""
        table.add_row(r.project, status_str, passed, failed, skipped, exit_code_str)

    console.print(table)

    passes = sum(1 for r in results if r.status in ("PASS", "SETUP_OK"))
    fails = sum(1 for r in results if r.status == "FAIL")
    timeouts = sum(1 for r in results if r.status == "TIMEOUT")
    setup_fails = sum(1 for r in results if r.status == "SETUP_FAIL")
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
    elif any(r.status in ("FAIL", "TIMEOUT", "SETUP_FAIL") for r in results):
        raise typer.Exit(1)


if __name__ == "__main__":
    app()

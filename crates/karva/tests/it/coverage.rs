use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

#[test]
fn test_no_cov_no_coverage_table() {
    let context = TestContext::with_file(
        "test_simple.py",
        r"
def test_one():
    assert 1 + 1 == 2
",
    );

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--status-level=none")
            .arg("test_simple.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_control_flow() {
    let context = TestContext::with_file(
        "test_control.py",
        r"
def both_branches(x):
    if x > 0:
        return 'pos'
    return 'neg'

def with_loop(n):
    total = 0
    for i in range(n):
        total += i
    return total

def with_try():
    try:
        return 'ok'
    except Exception:
        return 'err'

def test_pos():
    assert both_branches(1) == 'pos'
    assert with_loop(2) == 1
    assert with_try() == 'ok'
",
    );

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--cov")
            .arg("--status-level=none")
            .arg("test_control.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    Name              Stmts   Miss   Cover
    [LONG-LINE]
    test_control.py      18      3     83%
    [LONG-LINE]
    TOTAL                18      3     83%

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_saves_on_test_failure() {
    let context = TestContext::with_file(
        "test_failing.py",
        r"
def helper():
    return 1

def test_pass():
    assert helper() == 1

def test_fail():
    assert helper() == 999
",
    );

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--cov")
            .arg("--status-level=none")
            .arg("--final-status-level=none")
            .arg("test_failing.py"),
        @"
    success: false
    exit_code: 1
    ----- stdout -----

    Name              Stmts   Miss   Cover
    [LONG-LINE]
    test_failing.py       6      0    100%
    [LONG-LINE]
    TOTAL                 6      0    100%

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_multiple_sources() {
    let context = TestContext::with_files([
        ("pkg_a/__init__.py", ""),
        ("pkg_a/code.py", "def a():\n    return 1\n"),
        ("pkg_b/__init__.py", ""),
        ("pkg_b/code.py", "def b():\n    return 2\n"),
        ("pkg_c/__init__.py", ""),
        ("pkg_c/code.py", "def c():\n    return 3\n"),
        (
            "test_multi.py",
            r"
from pkg_a.code import a
from pkg_b.code import b
from pkg_c.code import c

def test_all():
    assert a() + b() + c() == 6
",
        ),
    ]);

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--cov=pkg_a")
            .arg("--cov=pkg_b")
            .arg("--status-level=none")
            .arg("test_multi.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    Name            Stmts   Miss   Cover
    [LONG-LINE]
    pkg_a/code.py       2      0    100%
    pkg_b/code.py       2      0    100%
    [LONG-LINE]
    TOTAL               4      0    100%

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_basic() {
    let context = TestContext::with_file(
        "test_covered.py",
        r"
def add(a, b):
    return a + b

def test_add():
    assert add(1, 2) == 3
",
    );

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--cov")
            .arg("--status-level=none")
            .arg("test_covered.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    Name              Stmts   Miss   Cover
    [LONG-LINE]
    test_covered.py       4      0    100%
    [LONG-LINE]
    TOTAL                 4      0    100%

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_partial() {
    let context = TestContext::with_file(
        "test_partial.py",
        r"
def covered():
    return 1

def uncovered():
    return 2

def test_only_covered():
    assert covered() == 1
",
    );

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--cov")
            .arg("--status-level=none")
            .arg("test_partial.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    Name              Stmts   Miss   Cover
    [LONG-LINE]
    test_partial.py       6      1     83%
    [LONG-LINE]
    TOTAL                 6      1     83%

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_report_term_missing() {
    let context = TestContext::with_file(
        "test_missing.py",
        r"
def covered():
    return 1

def uncovered_a():
    return 2

def uncovered_b():
    x = 3
    y = 4
    return x + y

def test_only_covered():
    assert covered() == 1
",
    );

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--cov")
            .arg("--cov-report=term-missing")
            .arg("--status-level=none")
            .arg("test_missing.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    Name              Stmts   Miss   Cover   Missing
    [LONG-LINE]
    test_missing.py      10      4     60%   6, 9-11
    [LONG-LINE]
    TOTAL                10      4     60%

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_report_term_default_no_missing_column() {
    let context = TestContext::with_file(
        "test_partial.py",
        r"
def covered():
    return 1

def uncovered():
    return 2

def test_only_covered():
    assert covered() == 1
",
    );

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--cov")
            .arg("--cov-report=term")
            .arg("--status-level=none")
            .arg("test_partial.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    Name              Stmts   Miss   Cover
    [LONG-LINE]
    test_partial.py       6      1     83%
    [LONG-LINE]
    TOTAL                 6      1     83%

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_skips_docstrings() {
    let context = TestContext::with_file(
        "test_docstrings.py",
        r#"
"""Module docstring."""


def doc_only():
    """Function with only a docstring is never called."""


def test_nothing():
    """This docstring should not count as a statement."""
    assert True
"#,
    );

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--cov")
            .arg("--status-level=none")
            .arg("test_docstrings.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    Name                 Stmts   Miss   Cover
    [LONG-LINE]
    test_docstrings.py       3      0    100%
    [LONG-LINE]
    TOTAL                    3      0    100%

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_walks_class_bodies() {
    let context = TestContext::with_file(
        "test_class.py",
        r"
class Foo:
    def method(self):
        return 1

    def unused(self):
        return 2

def test_method():
    assert Foo().method() == 1
",
    );

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--cov")
            .arg("--status-level=none")
            .arg("test_class.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    Name            Stmts   Miss   Cover
    [LONG-LINE]
    test_class.py       7      1     86%
    [LONG-LINE]
    TOTAL               7      1     86%

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_combines_across_workers() {
    let context = TestContext::with_files([
        (
            "test_a.py",
            r"
def test_a():
    assert 1 + 1 == 2
",
        ),
        (
            "test_b.py",
            r"
def test_b():
    assert 2 + 2 == 4
",
        ),
    ]);

    assert_cmd_snapshot!(
        context.command()
            .arg("--num-workers=2")
            .arg("--cov")
            .arg("--status-level=none"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    Name        Stmts   Miss   Cover
    [LONG-LINE]
    test_a.py       2      0    100%
    test_b.py       2      0    100%
    [LONG-LINE]
    TOTAL           4      0    100%

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_with_source_filter() {
    let context = TestContext::with_files([
        (
            "src/mymod.py",
            r"
def add(a, b):
    return a + b
",
        ),
        (
            "test_mymod.py",
            r"
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from src.mymod import add

def test_add():
    assert add(2, 3) == 5
",
        ),
    ]);

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--cov=src")
            .arg("--status-level=none")
            .arg("test_mymod.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    Name           Stmts   Miss   Cover
    [LONG-LINE]
    src/mymod.py       2      0    100%
    [LONG-LINE]
    TOTAL              2      0    100%

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_sources_from_config() {
    let context = TestContext::with_files([
        (
            "karva.toml",
            r#"
[profile.default.coverage]
sources = ["src"]
"#,
        ),
        (
            "src/mymod.py",
            r"
def add(a, b):
    return a + b
",
        ),
        (
            "test_mymod.py",
            r"
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from src.mymod import add

def test_add():
    assert add(2, 3) == 5
",
        ),
    ]);

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--status-level=none")
            .arg("test_mymod.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    Name           Stmts   Miss   Cover
    [LONG-LINE]
    src/mymod.py       2      0    100%
    [LONG-LINE]
    TOTAL              2      0    100%

    ----- stderr -----
    "
    );
}

#[test]
fn test_no_cov_overrides_config_sources() {
    let context = TestContext::with_files([
        (
            "karva.toml",
            r#"
[profile.default.coverage]
sources = [""]
"#,
        ),
        (
            "test_simple.py",
            r"
def test_one():
    assert 1 + 1 == 2
",
        ),
    ]);

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--no-cov")
            .arg("--status-level=none")
            .arg("test_simple.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    "
    );
}

/// `--cov --no-cov` (no-cov last) disables coverage; clap `overrides_with`
/// makes the later flag win.
#[test]
fn test_no_cov_after_cov_disables_coverage() {
    let context = TestContext::with_file(
        "test_simple.py",
        r"
def test_one():
    assert 1 + 1 == 2
",
    );

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--cov")
            .arg("--no-cov")
            .arg("--status-level=none")
            .arg("test_simple.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_includes_unimported_files_at_zero_percent() {
    let context = TestContext::with_files([
        ("src/__init__.py", ""),
        (
            "src/imported.py",
            r"
def used():
    return 1
",
        ),
        (
            "src/unimported.py",
            r"
def lonely():
    return 2

def other():
    return 3
",
        ),
        (
            "test_partial.py",
            r"
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from src.imported import used

def test_used():
    assert used() == 1
",
        ),
    ]);

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--cov=src")
            .arg("--status-level=none")
            .arg("test_partial.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    Name                Stmts   Miss   Cover
    [LONG-LINE]
    src/imported.py         2      0    100%
    src/unimported.py       4      4      0%
    [LONG-LINE]
    TOTAL                   6      4     33%

    ----- stderr -----
    "
    );
}

#[test]
fn test_cov_report_term_missing_from_config() {
    let context = TestContext::with_files([
        (
            "karva.toml",
            r#"
[profile.default.coverage]
sources = [""]
report = "term-missing"
"#,
        ),
        (
            "test_missing.py",
            r"
def covered():
    return 1

def uncovered():
    return 2

def test_only_covered():
    assert covered() == 1
",
        ),
    ]);

    assert_cmd_snapshot!(
        context.command_no_parallel()
            .arg("--status-level=none")
            .arg("test_missing.py"),
        @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    Name              Stmts   Miss   Cover   Missing
    [LONG-LINE]
    test_missing.py       6      1     83%   6
    [LONG-LINE]
    TOTAL                 6      1     83%

    ----- stderr -----
    "
    );
}

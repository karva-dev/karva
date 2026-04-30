use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

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

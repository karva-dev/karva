use insta::Settings;
use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

/// Bind an insta filter that collapses the numeric columns of
/// `coverage report` output (`Stmts`, `Miss`, `Cover%`) so the snapshot
/// remains stable across coverage.py versions.
fn coverage_filters() -> insta::internals::SettingsBindDropGuard {
    let mut settings = Settings::clone_current();
    settings.add_filter(r"\s{2,}\d+\s+\d+\s+\d+%", "  [N]  [N]  [N]%");
    settings.bind_to_scope()
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

    let _filters = coverage_filters();

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
    test_covered.py  [N]  [N]  [N]%
    [LONG-LINE]
    TOTAL  [N]  [N]  [N]%

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

    let _filters = coverage_filters();

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
    src/mymod.py  [N]  [N]  [N]%
    [LONG-LINE]
    TOTAL  [N]  [N]  [N]%

    ----- stderr -----
    "
    );
}

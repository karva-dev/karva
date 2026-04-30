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

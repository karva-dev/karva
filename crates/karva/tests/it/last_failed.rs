use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

#[test]
fn last_failed_reruns_only_failures() {
    let context = TestContext::with_files([(
        "test_a.py",
        "
            def test_pass(): pass
            def test_fail(): assert False
            ",
    )]);

    context.command_no_parallel().output().unwrap();

    assert_cmd_snapshot!(context.command_no_parallel().arg("--last-failed"), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 2 tests across 1 worker
            FAIL [TIME] test_a::test_fail

    diagnostics:

    error[test-failure]: Test `test_fail` failed
     --> test_a.py:3:5
      |
    2 | def test_pass(): pass
    3 | def test_fail(): assert False
      |     ^^^^^^^^^
      |
    info: Test failed here
     --> test_a.py:3:1
      |
    2 | def test_pass(): pass
    3 | def test_fail(): assert False
      | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      |

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn last_failed_lf_alias() {
    let context = TestContext::with_files([(
        "test_a.py",
        "
            def test_pass(): pass
            def test_fail(): assert False
            ",
    )]);

    context.command_no_parallel().output().unwrap();

    assert_cmd_snapshot!(context.command_no_parallel().arg("--lf"), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 2 tests across 1 worker
            FAIL [TIME] test_a::test_fail

    diagnostics:

    error[test-failure]: Test `test_fail` failed
     --> test_a.py:3:5
      |
    2 | def test_pass(): pass
    3 | def test_fail(): assert False
      |     ^^^^^^^^^
      |
    info: Test failed here
     --> test_a.py:3:1
      |
    2 | def test_pass(): pass
    3 | def test_fail(): assert False
      | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      |

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn last_failed_with_no_previous_failures_runs_all() {
    let context = TestContext::with_files([(
        "test_a.py",
        "
            def test_one(): pass
            def test_two(): pass
            ",
    )]);

    context.command_no_parallel().output().unwrap();

    assert_cmd_snapshot!(context.command_no_parallel().arg("--last-failed"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test_a::test_one
            PASS [TIME] test_a::test_two

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn last_failed_without_previous_run_runs_all() {
    let context = TestContext::with_files([(
        "test_a.py",
        "
            def test_one(): pass
            def test_two(): pass
            ",
    )]);

    assert_cmd_snapshot!(context.command_no_parallel().arg("--last-failed"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test_a::test_one
            PASS [TIME] test_a::test_two

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn last_failed_with_multiple_files() {
    let context = TestContext::with_files([
        (
            "test_a.py",
            "
def test_pass(): pass
def test_fail_a(): assert False
            ",
        ),
        (
            "test_b.py",
            "
def test_pass_b(): pass
def test_fail_b(): assert False
            ",
        ),
    ]);

    context.command_no_parallel().output().unwrap();

    assert_cmd_snapshot!(context.command_no_parallel().arg("--last-failed").arg("-q"), @"
    success: false
    exit_code: 1
    ----- stdout -----
    ────────────
         Summary [TIME] 2 tests run: 0 passed, 2 failed, 0 skipped

    ----- stderr -----
    ");
}

/// `--dry-run` ignores `--last-failed` and prints every discovered test.
#[test]
fn last_failed_with_dry_run_shows_all_tests() {
    let context = TestContext::with_file(
        "test_a.py",
        "
def test_pass(): pass
def test_fail(): assert False
        ",
    );

    context.command_no_parallel().output().unwrap();

    assert_cmd_snapshot!(
        context
            .command_no_parallel()
            .args(["--last-failed", "--dry-run"]),
        @"
    success: true
    exit_code: 0
    ----- stdout -----
    <test> test_a::test_fail
    <test> test_a::test_pass

    2 tests collected

    ----- stderr -----
    "
    );
}

/// A filter combined with `--last-failed` intersects: tests that were in the
/// last-failed set but are now filtered out are skipped.
#[test]
fn last_failed_with_filter_intersects() {
    let context = TestContext::with_file(
        "test_a.py",
        "
def test_pass(): pass
def test_fail_a(): assert False
def test_fail_b(): assert False
        ",
    );

    context.command_no_parallel().output().unwrap();

    assert_cmd_snapshot!(
        context
            .command_no_parallel()
            .args(["--last-failed", "-E", "test(~fail_a)"]),
        @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 3 tests across 1 worker
            FAIL [TIME] test_a::test_fail_a
            SKIP [TIME] test_a::test_fail_b

    diagnostics:

    error[test-failure]: Test `test_fail_a` failed
     --> test_a.py:3:5
      |
    2 | def test_pass(): pass
    3 | def test_fail_a(): assert False
      |     ^^^^^^^^^^^
    4 | def test_fail_b(): assert False
      |
    info: Test failed here
     --> test_a.py:3:1
      |
    2 | def test_pass(): pass
    3 | def test_fail_a(): assert False
      | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    4 | def test_fail_b(): assert False
      |

    ────────────
         Summary [TIME] 2 tests run: 0 passed, 1 failed, 1 skipped

    ----- stderr -----
    "
    );
}

/// `--last-failed` + `--max-fail=1` still stops scheduling once a single test
/// in the rerun has failed.
#[test]
fn last_failed_with_max_fail_stops_early() {
    let context = TestContext::with_file(
        "test_a.py",
        "
def test_pass(): pass
def test_fail_a(): assert False
def test_fail_b(): assert False
        ",
    );

    context.command_no_parallel().output().unwrap();

    assert_cmd_snapshot!(
        context
            .command_no_parallel()
            .args(["--last-failed", "--max-fail=1", "-q"]),
        @"
    success: false
    exit_code: 1
    ----- stdout -----
    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    "
    );
}

/// Adding a brand new test after a run does not cause `--last-failed` to pick
/// it up — only previously-known failures are rerun.
#[test]
fn last_failed_ignores_newly_added_tests() {
    let context = TestContext::with_file(
        "test_a.py",
        "
def test_pass(): pass
def test_fail(): assert False
        ",
    );

    context.command_no_parallel().output().unwrap();

    context.write_file(
        "test_a.py",
        "
def test_pass(): pass
def test_fail(): assert False
def test_new_fail(): assert False
        ",
    );

    assert_cmd_snapshot!(
        context.command_no_parallel().args(["--last-failed", "-q"]),
        @"
    success: false
    exit_code: 1
    ----- stdout -----
    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    "
    );
}

#[test]
fn last_failed_fix_then_rerun() {
    let context = TestContext::with_file(
        "test_a.py",
        "
def test_pass(): pass
def test_fail(): assert False
        ",
    );

    context.command_no_parallel().output().unwrap();

    context.write_file(
        "test_a.py",
        "
def test_pass(): pass
def test_fail(): assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--last-failed"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test_a::test_fail

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

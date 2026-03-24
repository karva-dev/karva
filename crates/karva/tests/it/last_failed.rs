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
         Summary [TIME] 1 tests run: 0 passed, 1 failed, 0 skipped

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
         Summary [TIME] 1 tests run: 0 passed, 1 failed, 0 skipped

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
         Summary [TIME] 2 tests run: 2 passed, 0 failed, 0 skipped

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
         Summary [TIME] 2 tests run: 2 passed, 0 failed, 0 skipped

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
         Summary [TIME] 1 tests run: 1 passed, 0 failed, 0 skipped

    ----- stderr -----
    ");
}

use insta::allow_duplicates;
use insta_cmd::assert_cmd_snapshot;
use rstest::rstest;

use crate::common::TestContext;

#[test]
fn test_timeout_passes_when_under_limit() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.timeout(5.0)
def test_fast():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_fast

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_timeout_fails_when_exceeded_pytest() {
    let context = TestContext::with_file(
        "test.py",
        r"
import time
import pytest

@pytest.mark.timeout(0.1)
def test_slow():
    time.sleep(2)
        ",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 1 test across 1 worker
            FAIL [TIME] test::test_slow

    diagnostics:

    error[test-failure]: Test `test_slow` failed
     --> test.py:6:5
      |
    6 | def test_slow():
      |     ^^^^^^^^^
      |
    info: Test exceeded timeout of 0.1 seconds

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_timeout_async_test() {
    let context = TestContext::with_file(
        "test.py",
        r"
import asyncio
import karva

@karva.tags.timeout(0.1)
async def test_slow_async():
    await asyncio.sleep(2)
        ",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 1 test across 1 worker
            FAIL [TIME] test::test_slow_async

    diagnostics:

    error[test-failure]: Test `test_slow_async` failed
     --> test.py:6:11
      |
    6 | async def test_slow_async():
      |           ^^^^^^^^^^^^^^^
      |
    info: Test exceeded timeout of 0.1 seconds

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_timeout_with_retry_eventually_passes() {
    let context = TestContext::with_file(
        "test.py",
        r"
import time
import karva

attempts = [0]

@karva.tags.timeout(0.5)
def test_slow_then_fast():
    attempts[0] += 1
    if attempts[0] == 1:
        time.sleep(2)
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--retry=2"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
      TRY 1 FAIL [TIME] test::test_slow_then_fast
      TRY 2 PASS [TIME] test::test_slow_then_fast

    ────────────
         Summary [TIME] 1 test run: 1 passed (1 flaky), 0 skipped
       FLAKY 2/3 [TIME] test::test_slow_then_fast

    ----- stderr -----
    ");
}

#[rstest]
fn test_timeout_invalid_seconds_rejected(
    #[values("0", "-1", "float('nan')", "float('inf')")] arg: &str,
) {
    let context = TestContext::with_file(
        "test.py",
        &format!(
            r"
import karva

@karva.tags.timeout({arg})
def test_1():
    assert True
        "
        ),
    );

    allow_duplicates! {
        assert_cmd_snapshot!(context.command(), @"
        success: false
        exit_code: 1
        ----- stdout -----
            Starting 1 test across 1 worker
        diagnostics:

        error[failed-to-import-module]: Failed to import python module `test`: timeout seconds must be a finite, positive number

        ────────────
             Summary [TIME] 0 tests run: 0 passed, 0 skipped

        ----- stderr -----
        ");
    }
}

#[test]
fn test_timeout_with_parametrize_each_case_gets_fresh_window() {
    let context = TestContext::with_file(
        "test.py",
        r"
import time
import karva

@karva.tags.timeout(0.3)
@karva.tags.parametrize('sleep_for', [0.0, 2.0, 0.0])
def test_1(sleep_for):
    time.sleep(sleep_for)
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_1(sleep_for=0.0)
            FAIL [TIME] test::test_1(sleep_for=2.0)
            PASS [TIME] test::test_1(sleep_for=0.0)

    diagnostics:

    error[test-failure]: Test `test_1` failed
     --> test.py:7:5
      |
    7 | def test_1(sleep_for):
      |     ^^^^^^
      |
    info: Test ran with arguments:
    info: `sleep_for`: `2.0`
    info: Test exceeded timeout of 0.3 seconds

    ────────────
         Summary [TIME] 3 tests run: 2 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_timeout_combined_with_skip_does_not_run() {
    let context = TestContext::with_file(
        "test.py",
        r"
import time
import karva

@karva.tags.timeout(0.1)
@karva.tags.skip(reason='not today')
def test_1():
    time.sleep(2)
        ",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_timeout_with_retry_exhausts_on_always_timing_out() {
    let context = TestContext::with_file(
        "test.py",
        r"
import time
import karva

@karva.tags.timeout(0.1)
def test_always_slow():
    time.sleep(2)
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--retry=1"), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 1 test across 1 worker
      TRY 1 FAIL [TIME] test::test_always_slow
      TRY 2 FAIL [TIME] test::test_always_slow

    diagnostics:

    error[test-failure]: Test `test_always_slow` failed
     --> test.py:6:5
      |
    6 | def test_always_slow():
      |     ^^^^^^^^^^^^^^^^
      |
    info: Test exceeded timeout of 0.1 seconds

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

use insta_cmd::assert_cmd_snapshot;

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
    5 | @pytest.mark.timeout(0.1)
    6 | def test_slow():
      |     ^^^^^^^^^
    7 |     time.sleep(2)
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
    5 | @karva.tags.timeout(0.1)
    6 | async def test_slow_async():
      |           ^^^^^^^^^^^^^^^
    7 |     await asyncio.sleep(2)
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
    5 | @karva.tags.timeout(0.1)
    6 | def test_always_slow():
      |     ^^^^^^^^^^^^^^^^
    7 |     time.sleep(2)
      |
    info: Test exceeded timeout of 0.1 seconds

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

const MIXED_TESTS: &str = r"
import karva

@karva.tags.skip
def test_skipped():
    assert False

@karva.tags.skip('reason here')
def test_skipped_with_reason():
    assert False

def test_normal():
    assert True
";

#[test]
fn runignored_runs_only_skipped_tests() {
    let context = TestContext::with_file("test.py", MIXED_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("-E").arg("runignored(only)"), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 3 tests across 1 worker
            FAIL [TIME] test::test_skipped
            FAIL [TIME] test::test_skipped_with_reason
            SKIP [TIME] test::test_normal

    diagnostics:

    error[test-failure]: Test `test_skipped` failed
     --> test.py:5:5
      |
    4 | @karva.tags.skip
    5 | def test_skipped():
      |     ^^^^^^^^^^^^
    6 |     assert False
      |
    info: Test failed here
     --> test.py:6:5
      |
    4 | @karva.tags.skip
    5 | def test_skipped():
    6 |     assert False
      |     ^^^^^^^^^^^^
    7 |
    8 | @karva.tags.skip('reason here')
      |

    error[test-failure]: Test `test_skipped_with_reason` failed
      --> test.py:9:5
       |
     8 | @karva.tags.skip('reason here')
     9 | def test_skipped_with_reason():
       |     ^^^^^^^^^^^^^^^^^^^^^^^^
    10 |     assert False
       |
    info: Test failed here
      --> test.py:10:5
       |
     8 | @karva.tags.skip('reason here')
     9 | def test_skipped_with_reason():
    10 |     assert False
       |     ^^^^^^^^^^^^
    11 |
    12 | def test_normal():
       |

    ────────────
         Summary [TIME] 3 tests run: 0 passed, 2 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn runignored_all_runs_skipped_alongside_normal() {
    let context = TestContext::with_file("test.py", MIXED_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("-E").arg("runignored(all)"), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 3 tests across 1 worker
            FAIL [TIME] test::test_skipped
            FAIL [TIME] test::test_skipped_with_reason
            PASS [TIME] test::test_normal

    diagnostics:

    error[test-failure]: Test `test_skipped` failed
     --> test.py:5:5
      |
    4 | @karva.tags.skip
    5 | def test_skipped():
      |     ^^^^^^^^^^^^
    6 |     assert False
      |
    info: Test failed here
     --> test.py:6:5
      |
    4 | @karva.tags.skip
    5 | def test_skipped():
    6 |     assert False
      |     ^^^^^^^^^^^^
    7 |
    8 | @karva.tags.skip('reason here')
      |

    error[test-failure]: Test `test_skipped_with_reason` failed
      --> test.py:9:5
       |
     8 | @karva.tags.skip('reason here')
     9 | def test_skipped_with_reason():
       |     ^^^^^^^^^^^^^^^^^^^^^^^^
    10 |     assert False
       |
    info: Test failed here
      --> test.py:10:5
       |
     8 | @karva.tags.skip('reason here')
     9 | def test_skipped_with_reason():
    10 |     assert False
       |     ^^^^^^^^^^^^
    11 |
    12 | def test_normal():
       |

    ────────────
         Summary [TIME] 3 tests run: 1 passed, 2 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn runignored_with_no_skipped_tests_skips_all() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_alpha():
    assert True

def test_beta():
    assert True
",
    );
    assert_cmd_snapshot!(context.command_no_parallel().arg("-E").arg("runignored(only)"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            SKIP [TIME] test::test_alpha
            SKIP [TIME] test::test_beta

    ────────────
         Summary [TIME] 2 tests run: 0 passed, 2 skipped

    ----- stderr -----
    ");
}

#[test]
fn runignored_skipif_false_not_matched() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.skip(False, reason='Condition is false')
def test_conditional():
    assert True

def test_normal():
    assert True
",
    );
    assert_cmd_snapshot!(context.command_no_parallel().arg("-E").arg("runignored(only)"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_conditional
            SKIP [TIME] test::test_normal

    ────────────
         Summary [TIME] 2 tests run: 1 passed, 1 skipped

    ----- stderr -----
    ");
}

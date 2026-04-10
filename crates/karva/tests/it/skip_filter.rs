use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

const TWO_TESTS: &str = r"
def test_alpha():
    assert True

def test_beta():
    assert True
";

#[test]
fn skip_filter_substring_match() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("--skip").arg("alpha"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            SKIP [TIME] test::test_alpha
            PASS [TIME] test::test_beta

    ────────────
         Summary [TIME] 2 tests run: 1 passed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn skip_filter_multiple_flags_or_semantics() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("--skip").arg("alpha").arg("--skip").arg("beta"), @"
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
fn skip_filter_no_matches() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("--skip").arg("nonexistent"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_alpha
            PASS [TIME] test::test_beta

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn skip_filter_invalid_regex() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("--skip").arg("[invalid"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: invalid regex pattern `[invalid`: regex parse error:
        [invalid
        ^
    error: unclosed character class
    ");
}

#[test]
fn skip_filter_combined_with_match() {
    // --match selects alpha and beta, --skip removes beta → only alpha runs
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("alpha|beta").arg("--skip").arg("beta"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_alpha
            SKIP [TIME] test::test_beta

    ────────────
         Summary [TIME] 2 tests run: 1 passed, 1 skipped

    ----- stderr -----
    ");
}

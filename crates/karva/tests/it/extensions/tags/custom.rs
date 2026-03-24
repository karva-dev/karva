use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

// ── Invalid tag expression errors ──────────────────────────────────────

const MINIMAL_TEST_FILE: &str = r"
def test_placeholder():
    assert True
";

#[test]
fn test_tag_filter_unexpected_character() {
    let context = TestContext::with_file("test.py", MINIMAL_TEST_FILE);
    assert_cmd_snapshot!(context.command().arg("-t").arg("slow!"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: unexpected character `!` in tag expression `slow!`
    ");
}

#[test]
fn test_tag_filter_unclosed_parenthesis() {
    let context = TestContext::with_file("test.py", MINIMAL_TEST_FILE);
    assert_cmd_snapshot!(context.command().arg("-t").arg("(slow"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: expected closing `)` in tag expression `(slow`
    ");
}

#[test]
fn test_tag_filter_trailing_operator() {
    let context = TestContext::with_file("test.py", MINIMAL_TEST_FILE);
    assert_cmd_snapshot!(context.command().arg("-t").arg("slow and"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: unexpected end of tag expression `slow and`
    ");
}

#[test]
fn test_tag_filter_leading_operator() {
    let context = TestContext::with_file("test.py", MINIMAL_TEST_FILE);
    assert_cmd_snapshot!(context.command().arg("-t").arg("and slow"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: unexpected token `and` in tag expression `and slow`
    ");
}

#[test]
fn test_tag_filter_extra_closing_paren() {
    let context = TestContext::with_file("test.py", MINIMAL_TEST_FILE);
    assert_cmd_snapshot!(context.command().arg("-t").arg("slow)"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: unexpected token `)` in tag expression `slow)`
    ");
}

#[test]
fn test_tag_filter_empty_parentheses() {
    let context = TestContext::with_file("test.py", MINIMAL_TEST_FILE);
    assert_cmd_snapshot!(context.command().arg("-t").arg("()"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: unexpected token `)` in tag expression `()`
    ");
}

#[test]
fn test_tag_filter_double_operator() {
    let context = TestContext::with_file("test.py", MINIMAL_TEST_FILE);
    assert_cmd_snapshot!(context.command().arg("-t").arg("slow and and fast"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: unexpected token `and` in tag expression `slow and and fast`
    ");
}

#[test]
fn test_tag_filter_whitespace_only() {
    let context = TestContext::with_file("test.py", MINIMAL_TEST_FILE);
    assert_cmd_snapshot!(context.command().arg("-t").arg(" "), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: empty tag expression ` `
    ");
}

#[test]
fn test_custom_tag_basic() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
def test_1():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_1

    ────────────
         Summary [TIME] 1 tests run: 1 passed, 0 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_custom_tag_with_args() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

@karva.tags.timeout(30, "seconds")
def test_1():
    assert True
        "#,
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_1

    ────────────
         Summary [TIME] 1 tests run: 1 passed, 0 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_custom_tag_with_kwargs() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.flaky(retries=3, delay=1.5)
def test_1():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_1

    ────────────
         Summary [TIME] 1 tests run: 1 passed, 0 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_custom_tag_with_mixed_args_and_kwargs() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

@karva.tags.marker("value1", 42, key="value2")
def test_1():
    assert True
        "#,
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_1

    ────────────
         Summary [TIME] 1 tests run: 1 passed, 0 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_multiple_custom_tags() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
@karva.tags.integration
@karva.tags.priority(1)
def test_1():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_1

    ────────────
         Summary [TIME] 1 tests run: 1 passed, 0 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_custom_tags_combined_with_builtin_tags() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
@karva.tags.skip
def test_skipped():
    assert False

@karva.tags.integration
def test_runs():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            SKIP [TIME] test::test_skipped
            PASS [TIME] test::test_runs

    ────────────
         Summary [TIME] 2 tests run: 1 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_tag_filter_include() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
def test_slow():
    assert True

def test_fast():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-t").arg("slow"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_slow
            SKIP [TIME] test::test_fast

    ────────────
         Summary [TIME] 2 tests run: 1 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_tag_filter_exclude() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
def test_slow():
    assert True

def test_fast():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-t").arg("not slow"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            SKIP [TIME] test::test_slow
            PASS [TIME] test::test_fast

    ────────────
         Summary [TIME] 2 tests run: 1 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_tag_filter_and_expression() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
@karva.tags.integration
def test_slow_integration():
    assert True

@karva.tags.slow
def test_slow_only():
    assert True

@karva.tags.integration
def test_integration_only():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-t").arg("slow and integration"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test::test_slow_integration
            SKIP [TIME] test::test_slow_only
            SKIP [TIME] test::test_integration_only

    ────────────
         Summary [TIME] 3 tests run: 1 passed, 0 failed, 2 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_tag_filter_or_expression() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
def test_slow():
    assert True

@karva.tags.integration
def test_integration():
    assert True

def test_untagged():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-t").arg("slow or integration"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test::test_slow
            PASS [TIME] test::test_integration
            SKIP [TIME] test::test_untagged

    ────────────
         Summary [TIME] 3 tests run: 2 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_tag_filter_multiple_flags() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
@karva.tags.integration
def test_both():
    assert True

@karva.tags.slow
def test_slow_only():
    assert True

@karva.tags.integration
def test_integration_only():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-t").arg("slow").arg("-t").arg("integration"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test::test_both
            PASS [TIME] test::test_slow_only
            PASS [TIME] test::test_integration_only

    ────────────
         Summary [TIME] 3 tests run: 3 passed, 0 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_tag_filter_no_matches() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

def test_untagged():
    assert True

@karva.tags.fast
def test_fast():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-t").arg("slow"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            SKIP [TIME] test::test_untagged
            SKIP [TIME] test::test_fast

    ────────────
         Summary [TIME] 2 tests run: 0 passed, 0 failed, 2 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_tag_filter_with_parametrize() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
@karva.tags.parametrize('x', [1, 2])
def test_param(x):
    assert x > 0

def test_untagged():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-t").arg("slow"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_param(x=1)
            PASS [TIME] test::test_param(x=2)
            SKIP [TIME] test::test_untagged

    ────────────
         Summary [TIME] 3 tests run: 2 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_tag_filter_not_with_and() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
@karva.tags.flaky
def test_slow_flaky():
    assert True

@karva.tags.slow
def test_slow_stable():
    assert True

def test_untagged():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-t").arg("slow and not flaky"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            SKIP [TIME] test::test_slow_flaky
            PASS [TIME] test::test_slow_stable
            SKIP [TIME] test::test_untagged

    ────────────
         Summary [TIME] 3 tests run: 1 passed, 0 failed, 2 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_tag_filter_parenthesized_expression() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
@karva.tags.linux
def test_slow_linux():
    assert True

@karva.tags.fast
@karva.tags.linux
def test_fast_linux():
    assert True

@karva.tags.slow
def test_slow_only():
    assert True

@karva.tags.linux
def test_linux_only():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-t").arg("(slow or fast) and linux"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 4 tests across 1 worker
            PASS [TIME] test::test_slow_linux
            PASS [TIME] test::test_fast_linux
            SKIP [TIME] test::test_slow_only
            SKIP [TIME] test::test_linux_only

    ────────────
         Summary [TIME] 4 tests run: 2 passed, 0 failed, 2 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_pytest_custom_marks_with_tag_filter() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import pytest

@pytest.mark.slow
def test_slow():
    assert True

@pytest.mark.slow("reason", key="value")
def test_slow_with_args():
    assert True

def test_untagged():
    assert True
        "#,
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-t").arg("slow"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test::test_slow
            PASS [TIME] test::test_slow_with_args
            SKIP [TIME] test::test_untagged

    ────────────
         Summary [TIME] 3 tests run: 2 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_tag_filter_with_skip() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
@karva.tags.skip
def test_slow_skipped():
    assert False

@karva.tags.slow
def test_slow_runs():
    assert True

def test_untagged():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-t").arg("slow"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            SKIP [TIME] test::test_slow_skipped
            PASS [TIME] test::test_slow_runs
            SKIP [TIME] test::test_untagged

    ────────────
         Summary [TIME] 3 tests run: 1 passed, 0 failed, 2 skipped

    ----- stderr -----
    ");
}

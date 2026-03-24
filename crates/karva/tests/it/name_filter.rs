use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

const TWO_TESTS: &str = r"
def test_alpha():
    assert True

def test_beta():
    assert True
";

#[test]
fn name_filter_substring_match() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("alpha"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_alpha
            SKIP [TIME] test::test_beta

    ────────────
         Summary [TIME] 2 tests run: 1 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_anchored_regex() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("beta$"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            SKIP [TIME] test::test_alpha
            PASS [TIME] test::test_beta

    ────────────
         Summary [TIME] 2 tests run: 1 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_multiple_flags_or_semantics() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("alpha").arg("-m").arg("beta"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_alpha
            PASS [TIME] test::test_beta

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_no_matches() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("nonexistent"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            SKIP [TIME] test::test_alpha
            SKIP [TIME] test::test_beta

    ────────────
         Summary [TIME] 2 tests run: 0 passed, 0 failed, 2 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_invalid_regex() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("[invalid"), @r"
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
fn name_filter_parametrize() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.parametrize('x', [1, 2, 3])
def test_param(x):
    assert x > 0

def test_other():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("test_param"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_param(x=1)
            PASS [TIME] test::test_param(x=2)
            PASS [TIME] test::test_param(x=3)
            SKIP [TIME] test::test_other

    ────────────
         Summary [TIME] 4 tests run: 3 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
#[cfg(unix)]
fn name_filter_match_all() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg(".*"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_alpha
            PASS [TIME] test::test_beta

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_alternation() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_login():
    assert True

def test_logout():
    assert True

def test_signup():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("login|signup"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test::test_login
            SKIP [TIME] test::test_logout
            PASS [TIME] test::test_signup

    ────────────
         Summary [TIME] 3 tests run: 2 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_character_class() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_v1():
    assert True

def test_v2():
    assert True

def test_v10():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg(r"test_v[12]$"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test::test_v1
            PASS [TIME] test::test_v2
            SKIP [TIME] test::test_v10

    ────────────
         Summary [TIME] 3 tests run: 2 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_quantifier() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_a():
    assert True

def test_ab():
    assert True

def test_abb():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg(r"test_ab+$"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            SKIP [TIME] test::test_a
            PASS [TIME] test::test_ab
            PASS [TIME] test::test_abb

    ────────────
         Summary [TIME] 3 tests run: 2 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_qualified_name_prefix() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_login():
    assert True

def test_logout():
    assert True

def test_signup():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("^test::test_log"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test::test_login
            PASS [TIME] test::test_logout
            SKIP [TIME] test::test_signup

    ────────────
         Summary [TIME] 3 tests run: 2 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_combined_with_tag_filter() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

@karva.tags.slow
def test_slow_alpha():
    assert True

@karva.tags.slow
def test_slow_beta():
    assert True

def test_fast_alpha():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-t").arg("slow").arg("-m").arg("alpha"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test::test_slow_alpha
            SKIP [TIME] test::test_slow_beta
            SKIP [TIME] test::test_fast_alpha

    ────────────
         Summary [TIME] 3 tests run: 1 passed, 0 failed, 2 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_case_sensitive() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_Alpha():
    assert True

def test_alpha():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("Alpha"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_Alpha
            SKIP [TIME] test::test_alpha

    ────────────
         Summary [TIME] 2 tests run: 1 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_case_insensitive_regex() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_Alpha():
    assert True

def test_alpha():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("(?i)alpha"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_Alpha
            PASS [TIME] test::test_alpha

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_dot_metacharacter() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_a1():
    assert True

def test_a2():
    assert True

def test_ab():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg(r"test_a\d"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test::test_a1
            PASS [TIME] test::test_a2
            SKIP [TIME] test::test_ab

    ────────────
         Summary [TIME] 3 tests run: 2 passed, 0 failed, 1 skipped

    ----- stderr -----
    ");
}

#[test]
fn name_filter_invalid_regex_unclosed_group() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("(unclosed"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: invalid regex pattern `(unclosed`: regex parse error:
        (unclosed
        ^
    error: unclosed group
    ");
}

#[test]
fn name_filter_invalid_regex_invalid_repetition() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg("*invalid"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: invalid regex pattern `*invalid`: regex parse error:
        *invalid
        ^
    error: repetition operator missing expression
    ");
}

#[test]
fn name_filter_invalid_regex_bad_escape() {
    let context = TestContext::with_file("test.py", TWO_TESTS);
    assert_cmd_snapshot!(context.command_no_parallel().arg("-m").arg(r"\p{Invalid}"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: invalid regex pattern `\p{Invalid}`: regex parse error:
        \p{Invalid}
        ^^^^^^^^^^^
    error: Unicode property not found
    ");
}

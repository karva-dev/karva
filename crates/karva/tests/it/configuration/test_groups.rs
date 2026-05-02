use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

#[test]
fn group_filter_matches_tests_assigned_via_override() {
    let context = TestContext::with_files([
        (
            "karva.toml",
            r#"
[test-groups.database]
max-threads = 4

[[profile.default.overrides]]
filter = "tag(database)"
test-group = "database"
"#,
        ),
        (
            "test.py",
            r"
import karva

@karva.tags.database
def test_db():
    assert True

def test_other():
    assert True
        ",
        ),
    ]);

    assert_cmd_snapshot!(
        context.command_no_parallel().arg("-E").arg("group(database)"),
        @r"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_db

    ────────────
         Summary [TIME] 2 tests run: 1 passed, 1 skipped

    ----- stderr -----
    "
    );
}

#[test]
fn override_referencing_unknown_group_errors() {
    let context = TestContext::with_files([
        (
            "karva.toml",
            r#"
[test-groups.database]
max-threads = 2

[[profile.default.overrides]]
filter = "tag(slow)"
test-group = "missing"
"#,
        ),
        ("test.py", "def test_x(): assert True\n"),
    ]);

    assert_cmd_snapshot!(context.command_no_parallel(), @r"
    success: false
    exit_code: 2
    ----- stdout -----
        Starting 1 test across 1 worker

    ----- stderr -----
    Karva failed
      Cause: invalid test-groups configuration: override #0 references unknown test-group `missing` (defined groups: database)
    ");
}

#[test]
fn override_with_group_predicate_in_filter_rejected() {
    let context = TestContext::with_files([
        (
            "karva.toml",
            r#"
[test-groups.database]
max-threads = 2

[[profile.default.overrides]]
filter = "group(database)"
test-group = "database"
"#,
        ),
        ("test.py", "def test_x(): assert True\n"),
    ]);

    assert_cmd_snapshot!(context.command_no_parallel(), @r"
    success: false
    exit_code: 2
    ----- stdout -----
        Starting 1 test across 1 worker

    ----- stderr -----
    Karva failed
      Cause: invalid test-groups configuration: override #0 filter `group(database)` uses `group(...)`; overrides cannot reference test-groups in their own filter
    ");
}

#[test]
fn test_groups_zero_max_threads_rejected_at_parse() {
    let context = TestContext::with_files([
        (
            "karva.toml",
            r"
[test-groups.serial]
max-threads = 0
",
        ),
        ("test.py", "def test_x(): assert True\n"),
    ]);

    assert_cmd_snapshot!(context.command_no_parallel(), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: <temp_dir>/karva.toml is not a valid `karva.toml`: TOML parse error at line 3, column 15
      |
    3 | max-threads = 0
      |               ^
    invalid value: integer `0`, expected a nonzero usize

      Cause: TOML parse error at line 3, column 15
      |
    3 | max-threads = 0
      |               ^
    invalid value: integer `0`, expected a nonzero usize
    ");
}

#[test]
fn serial_group_runs_when_filtered_to_group() {
    let context = TestContext::with_files([
        (
            "karva.toml",
            r#"
[test-groups.serial]
max-threads = 1

[[profile.default.overrides]]
filter = "tag(exclusive)"
test-group = "serial"
"#,
        ),
        (
            "test_serial.py",
            r"
import karva

@karva.tags.exclusive
def test_one():
    assert True

@karva.tags.exclusive
def test_two():
    assert True

@karva.tags.exclusive
def test_three():
    assert True
        ",
        ),
    ]);

    assert_cmd_snapshot!(
        context.command_no_parallel().arg("-E").arg("group(serial)"),
        @r"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test_serial::test_one
            PASS [TIME] test_serial::test_two
            PASS [TIME] test_serial::test_three

    ────────────
         Summary [TIME] 3 tests run: 3 passed, 0 skipped

    ----- stderr -----
    "
    );
}

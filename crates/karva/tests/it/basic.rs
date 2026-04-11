use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

#[test]
fn test_single_file() {
    let context = TestContext::with_files([
        (
            "test_file1.py",
            r"
def test_1(): pass
def test_2(): pass",
        ),
        (
            "test_file2.py",
            r"
def test_3(): pass
def test_4(): pass",
        ),
    ]);

    assert_cmd_snapshot!(context.command_no_parallel().arg("test_file1.py"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test_file1::test_1
            PASS [TIME] test_file1::test_2

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_empty_file() {
    let context = TestContext::with_file("test.py", "");

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
    ────────────
         Summary [TIME] 0 tests run: 0 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_empty_directory() {
    let context = TestContext::new();

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
    ────────────
         Summary [TIME] 0 tests run: 0 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_single_function() {
    let context = TestContext::with_file(
        "test.py",
        r"
            def test_1(): pass
            def test_2(): pass",
    );

    assert_cmd_snapshot!(context.command().arg("test.py::test_1"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_1

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_single_function_shadowed_by_file() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_1(): pass
def test_2(): pass",
    );

    assert_cmd_snapshot!(context.command_no_parallel().args(["test.py::test_1", "test.py"]), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_1
            PASS [TIME] test::test_2

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_single_function_shadowed_by_directory() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_1(): pass
def test_2(): pass",
    );

    assert_cmd_snapshot!(context.command_no_parallel().args(["test.py::test_1", "."]), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_1
            PASS [TIME] test::test_2

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_no_tests_found() {
    let context = TestContext::with_file("test_no_tests.py", r"");

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
    ────────────
         Summary [TIME] 0 tests run: 0 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_one_test_passes() {
    let context = TestContext::with_file(
        "test_pass.py",
        r"
        def test_pass():
            assert True
        ",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test_pass::test_pass

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_one_test_fail() {
    let context = TestContext::with_file(
        "test_fail.py",
        r"
        def test_fail():
            assert False
    ",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 1 test across 1 worker
            FAIL [TIME] test_fail::test_fail

    diagnostics:

    error[test-failure]: Test `test_fail` failed
     --> test_fail.py:2:5
      |
    2 | def test_fail():
      |     ^^^^^^^^^
    3 |     assert False
      |
    info: Test failed here
     --> test_fail.py:3:5
      |
    2 | def test_fail():
    3 |     assert False
      |     ^^^^^^^^^^^^
      |

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_fail_concise_output() {
    let context = TestContext::with_file(
        "test_fail.py",
        r"
        import karva

        @karva.fixture
        def fixture_1():
            yield 1
            raise ValueError('Teardown error')

        def test_1(fixture_1):
            assert fixture == 2

        @karva.fixture
        def fixture_2():
            raise ValueError('fixture error')

        def test_2(fixture_2):
            assert False

        def test_3():
            assert False
    ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--output-format").arg("concise"), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 3 tests across 1 worker
            FAIL [TIME] test_fail::test_1(fixture_1=1)
            FAIL [TIME] test_fail::test_2
            FAIL [TIME] test_fail::test_3

    diagnostics:

    test_fail.py:5:5: warning[invalid-fixture-finalizer] Discovered an invalid fixture finalizer `fixture_1`
    test_fail.py:9:5: error[test-failure] Test `test_1` failed
    test_fail.py:16:5: error[missing-fixtures] Test `test_2` has missing fixtures: `fixture_2`
    test_fail.py:19:5: error[test-failure] Test `test_3` failed

    ────────────
         Summary [TIME] 3 tests run: 0 passed, 3 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_two_test_fails() {
    let context = TestContext::with_file(
        "tests/test_fail.py",
        r"
        def test_fail():
            assert False

        def test_fail2():
            assert False, 'Test failed'
    ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 2 tests across 1 worker
            FAIL [TIME] tests.test_fail::test_fail
            FAIL [TIME] tests.test_fail::test_fail2

    diagnostics:

    error[test-failure]: Test `test_fail` failed
     --> tests/test_fail.py:2:5
      |
    2 | def test_fail():
      |     ^^^^^^^^^
    3 |     assert False
      |
    info: Test failed here
     --> tests/test_fail.py:3:5
      |
    2 | def test_fail():
    3 |     assert False
      |     ^^^^^^^^^^^^
    4 |
    5 | def test_fail2():
      |

    error[test-failure]: Test `test_fail2` failed
     --> tests/test_fail.py:5:5
      |
    3 |     assert False
    4 |
    5 | def test_fail2():
      |     ^^^^^^^^^^
    6 |     assert False, 'Test failed'
      |
    info: Test failed here
     --> tests/test_fail.py:6:5
      |
    5 | def test_fail2():
    6 |     assert False, 'Test failed'
      |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^
      |
    info: Test failed

    ────────────
         Summary [TIME] 2 tests run: 0 passed, 2 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_file_importing_another_file() {
    let context = TestContext::with_files([
        (
            "helper.py",
            r"
            def validate_data(data):
                if not data:
                    assert False, 'Data validation failed'
                return True
        ",
        ),
        (
            "test_cross_file.py",
            r"
            from helper import validate_data

            def test_with_helper():
                validate_data([])
        ",
        ),
    ]);

    assert_cmd_snapshot!(context.command(), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 1 test across 1 worker
            FAIL [TIME] test_cross_file::test_with_helper

    diagnostics:

    error[test-failure]: Test `test_with_helper` failed
     --> test_cross_file.py:4:5
      |
    2 | from helper import validate_data
    3 |
    4 | def test_with_helper():
      |     ^^^^^^^^^^^^^^^^
    5 |     validate_data([])
      |
    info: Test failed here
     --> helper.py:4:9
      |
    2 | def validate_data(data):
    3 |     if not data:
    4 |         assert False, 'Data validation failed'
      |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    5 |     return True
      |
    info: Data validation failed

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_stdout() {
    let context = TestContext::with_file(
        "test_std_out_redirected.py",
        r"
        def test_std_out_redirected():
            print('Hello, world!')
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-s"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
    Hello, world!
            PASS [TIME] test_std_out_redirected::test_std_out_redirected

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");

    assert_cmd_snapshot!(context.command().arg("-s"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
    Hello, world!
            PASS [TIME] test_std_out_redirected::test_std_out_redirected

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test_std_out_redirected::test_std_out_redirected

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_multiple_fixtures_not_found() {
    let context = TestContext::with_file(
        "test_multiple_fixtures_not_found.py",
        "def test_multiple_fixtures_not_found(a, b, c): ...",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 1 test across 1 worker
            FAIL [TIME] test_multiple_fixtures_not_found::test_multiple_fixtures_not_found

    diagnostics:

    error[missing-fixtures]: Test `test_multiple_fixtures_not_found` has missing fixtures
     --> test_multiple_fixtures_not_found.py:1:5
      |
    1 | def test_multiple_fixtures_not_found(a, b, c): ...
      |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      |
    info: Missing fixtures: `a`, `b`, `c`

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_text_file_in_directory() {
    let context = TestContext::with_files([
        ("test_sample.py", "def test_sample(): assert True"),
        ("random.txt", "pass"),
    ]);

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test_sample::test_sample

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_text_file() {
    let context = TestContext::with_file("random.txt", "pass");

    assert_cmd_snapshot!(
        context.command().args(["random.txt"]),
        @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: path `<temp_dir>/random.txt` has a wrong file extension
    ");
}

#[test]
fn test_quiet_output_passing() {
    let context = TestContext::with_file(
        "test.py",
        "
        def test_quiet_output():
            assert True
        ",
    );

    assert_cmd_snapshot!(context.command().args(["-q"]), @"
    success: true
    exit_code: 0
    ----- stdout -----
    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_quiet_output_failing() {
    let context = TestContext::with_file(
        "test.py",
        "
        def test_quiet_output():
            assert False
        ",
    );

    assert_cmd_snapshot!(context.command().args(["-q"]), @"
    success: false
    exit_code: 1
    ----- stdout -----
    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_invalid_path() {
    let context = TestContext::new();

    assert_cmd_snapshot!(context.command().arg("non_existing_path.py"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: path `<temp_dir>/non_existing_path.py` could not be found
    ");
}

#[test]
fn test_fixture_generator_two_yields_passing_test() {
    let context = TestContext::with_file(
        "test.py",
        r"
            import karva

            @karva.fixture
            def fixture_generator():
                yield 1
                yield 2

            def test_fixture_generator(fixture_generator):
                assert fixture_generator == 1
",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_fixture_generator(fixture_generator=1)

    diagnostics:

    warning[invalid-fixture-finalizer]: Discovered an invalid fixture finalizer `fixture_generator`
     --> test.py:5:5
      |
    4 | @karva.fixture
    5 | def fixture_generator():
      |     ^^^^^^^^^^^^^^^^^
    6 |     yield 1
    7 |     yield 2
      |
    info: Fixture had more than one yield statement

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_fixture_generator_two_yields_failing_test() {
    let context = TestContext::with_file(
        "test.py",
        r"
            import karva

            @karva.fixture
            def fixture_generator():
                yield 1
                yield 2

            def test_fixture_generator(fixture_generator):
                assert fixture_generator == 2
",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 1 test across 1 worker
            FAIL [TIME] test::test_fixture_generator(fixture_generator=1)

    diagnostics:

    warning[invalid-fixture-finalizer]: Discovered an invalid fixture finalizer `fixture_generator`
     --> test.py:5:5
      |
    4 | @karva.fixture
    5 | def fixture_generator():
      |     ^^^^^^^^^^^^^^^^^
    6 |     yield 1
    7 |     yield 2
      |
    info: Fixture had more than one yield statement

    error[test-failure]: Test `test_fixture_generator` failed
      --> test.py:9:5
       |
     7 |     yield 2
     8 |
     9 | def test_fixture_generator(fixture_generator):
       |     ^^^^^^^^^^^^^^^^^^^^^^
    10 |     assert fixture_generator == 2
       |
    info: Test ran with arguments:
    info: `fixture_generator`: `1`
    info: Test failed here
      --> test.py:10:5
       |
     9 | def test_fixture_generator(fixture_generator):
    10 |     assert fixture_generator == 2
       |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
       |

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_fixture_generator_fail_in_teardown() {
    let context = TestContext::with_file(
        "test.py",
        r#"
        import karva

        @karva.fixture
        def fixture_generator():
            yield 1
            raise ValueError("fixture error")

        def test_fixture_generator(fixture_generator):
            assert fixture_generator == 1
"#,
    );

    assert_cmd_snapshot!(context.command(), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_fixture_generator(fixture_generator=1)

    diagnostics:

    warning[invalid-fixture-finalizer]: Discovered an invalid fixture finalizer `fixture_generator`
     --> test.py:5:5
      |
    4 | @karva.fixture
    5 | def fixture_generator():
      |     ^^^^^^^^^^^^^^^^^
    6 |     yield 1
    7 |     raise ValueError("fixture error")
      |
    info: Failed to reset fixture: fixture error

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    "#);
}

#[test]
fn test_invalid_fixture() {
    let context = TestContext::with_file(
        "test.py",
        r#"
        import karva

        @karva.fixture(scope='ssession')
        def fixture_generator():
            raise ValueError("fixture-error")

        def test_fixture_generator(fixture_generator):
            assert fixture_generator == 1
"#,
    );

    assert_cmd_snapshot!(context.command(), @r#"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 1 test across 1 worker
            FAIL [TIME] test::test_fixture_generator

    diagnostics:

    error[invalid-fixture]: Discovered an invalid fixture `fixture_generator`
     --> test.py:5:5
      |
    4 | @karva.fixture(scope='ssession')
    5 | def fixture_generator():
      |     ^^^^^^^^^^^^^^^^^
    6 |     raise ValueError("fixture-error")
      |
    info: Invalid fixture scope: ssession

    error[missing-fixtures]: Test `test_fixture_generator` has missing fixtures
     --> test.py:8:5
      |
    6 |     raise ValueError("fixture-error")
    7 |
    8 | def test_fixture_generator(fixture_generator):
      |     ^^^^^^^^^^^^^^^^^^^^^^
    9 |     assert fixture_generator == 1
      |
    info: Missing fixtures: `fixture_generator`

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    "#);
}

#[test]
fn test_failfast() {
    let context = TestContext::with_file(
        "test_failfast.py",
        r"
        def test_first_fail():
            assert False, 'First test fails'

        def test_second():
            assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().args(["--fail-fast"]), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 2 tests across 1 worker
            FAIL [TIME] test_failfast::test_first_fail

    diagnostics:

    error[test-failure]: Test `test_first_fail` failed
     --> test_failfast.py:2:5
      |
    2 | def test_first_fail():
      |     ^^^^^^^^^^^^^^^
    3 |     assert False, 'First test fails'
      |
    info: Test failed here
     --> test_failfast.py:3:5
      |
    2 | def test_first_fail():
    3 |     assert False, 'First test fails'
      |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    4 |
    5 | def test_second():
      |
    info: First test fails

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_failfast_multiple_threads() {
    let context = TestContext::with_file(
        "test_a.py",
        r"
import time

def test_fail():
    assert False

def test_1():
    time.sleep(0.5)
    assert True

def test_2():
    time.sleep(0.5)
    assert True

def test_3():
    time.sleep(0.5)
    assert True

def test_4():
    time.sleep(0.5)
    assert True

def test_5():
    time.sleep(0.5)
    assert True

def test_6():
    time.sleep(0.5)
    assert True

def test_7():
    time.sleep(0.5)
    assert True

def test_8():
    time.sleep(0.5)
    assert True

def test_9():
    time.sleep(0.5)
    assert True
    ",
    );

    assert_cmd_snapshot!(context.command().arg("--fail-fast").arg("--num-workers").arg("2").arg("-v"), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 10 tests across 2 workers
            FAIL [TIME] test_a::test_fail

    diagnostics:

    error[test-failure]: Test `test_fail` failed
     --> test_a.py:4:5
      |
    2 | import time
    3 |
    4 | def test_fail():
      |     ^^^^^^^^^
    5 |     assert False
      |
    info: Test failed here
     --> test_a.py:5:5
      |
    4 | def test_fail():
    5 |     assert False
      |     ^^^^^^^^^^^^
    6 |
    7 | def test_1():
      |

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    INFO Collected all tests in [TIME]
    INFO Spawning 2 workers
    INFO Worker 0 spawned with 5 tests
    INFO Worker 1 spawned with 5 tests
    INFO Waiting for 2 workers to complete (Ctrl+C to cancel)
    INFO Fail-fast signal received — stopping remaining workers
    ");
}

#[test]
fn test_test_prefix() {
    let context = TestContext::with_file(
        "test_fail.py",
        r"
import karva

def test_1(): ...
def tests_1(): ...

        ",
    );

    assert_cmd_snapshot!(context.command().arg("--test-prefix").arg("tests_"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test_fail::tests_1

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_unused_files_are_imported() {
    let context = TestContext::with_file(
        "test_fail.py",
        r"
def test_1():
    assert True

        ",
    );

    context.write_file("foo.py", "print('hello world')");

    assert_cmd_snapshot!(context.command().arg("-s"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test_fail::test_1

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_unused_files_that_fail_are_not_imported() {
    let context = TestContext::with_file(
        "test_fail.py",
        r"
def test_1():
    assert True

        ",
    );

    context.write_file(
        "foo.py",
        "
    import sys
    sys.exit(1)",
    );

    assert_cmd_snapshot!(context.command().arg("-s"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test_fail::test_1

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_fixture_argument_truncated() {
    let context = TestContext::with_file(
        "test_file.py",
        r"
import karva

@karva.fixture
def fixture_very_very_very_very_very_long_name():
    return 'fixture_very_very_very_very_very_long_name'

def test_1(fixture_very_very_very_very_very_long_name):
    assert False
        ",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 1 test across 1 worker
            FAIL [TIME] test_file::test_1(fixture_very_very_very_very...=fixture_very_very_very_very...)

    diagnostics:

    error[test-failure]: Test `test_1` failed
     --> test_file.py:8:5
      |
    6 |     return 'fixture_very_very_very_very_very_long_name'
    7 |
    8 | def test_1(fixture_very_very_very_very_very_long_name):
      |     ^^^^^^
    9 |     assert False
      |
    info: Test ran with arguments:
    info: `fixture_very_very_very_very...`: `fixture_very_very_very_very...`
    info: Test failed here
     --> test_file.py:9:5
      |
    8 | def test_1(fixture_very_very_very_very_very_long_name):
    9 |     assert False
      |     ^^^^^^^^^^^^
      |

    ────────────
         Summary [TIME] 1 test run: 0 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_finalizer() {
    let context = TestContext::with_file(
        "test.py",
        r"
import os

def test_setenv(monkeypatch):
    monkeypatch.setenv('TEST_VAR_5', 'test_value_5')
    assert os.environ['TEST_VAR_5'] == 'test_value_5'

def test_1():
    assert 'TEST_VAR_5' not in os.environ
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-s"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_setenv(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_1

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_no_progress() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_1():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--no-progress"), @"
    success: true
    exit_code: 0
    ----- stdout -----

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_try_import_fixtures() {
    let context = TestContext::with_files([
        (
            "foo.py",
            r"
import karva

@karva.fixture
def x():
    return 1

@karva.fixture()
def y():
    return 1
                ",
        ),
        (
            "test_file.py",
            "
from foo import x, y
def test_1(x): pass
def test_2(y): pass
                ",
        ),
    ]);

    assert_cmd_snapshot!(context.command_no_parallel().arg("--try-import-fixtures"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test_file::test_1(x=1)
            PASS [TIME] test_file::test_2(y=1)

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_try_import_fixtures_invalid_fixtures() {
    let context = TestContext::with_files([
        (
            "foo.py",
            r"
import karva

@karva.fixture
def x():
    raise ValueError('Invalid fixture')

@karva.fixture()
def y():
    return 1
                ",
        ),
        (
            "test_file.py",
            "
from foo import x, y
def test_1(x): pass
def test_2(y): pass
                ",
        ),
    ]);

    assert_cmd_snapshot!(context.command_no_parallel().arg("--try-import-fixtures"), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 2 tests across 1 worker
            FAIL [TIME] test_file::test_1
            PASS [TIME] test_file::test_2(y=1)

    diagnostics:

    error[missing-fixtures]: Test `test_1` has missing fixtures
     --> test_file.py:3:5
      |
    2 | from foo import x, y
    3 | def test_1(x): pass
      |     ^^^^^^
    4 | def test_2(y): pass
      |
    info: Missing fixtures: `x`
    info: Fixture `x` failed here
     --> foo.py:6:5
      |
    4 | @karva.fixture
    5 | def x():
    6 |     raise ValueError('Invalid fixture')
      |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    7 |
    8 | @karva.fixture()
      |
    info: Invalid fixture

    ────────────
         Summary [TIME] 2 tests run: 1 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_retry() {
    let context = TestContext::with_file(
        "test.py",
        r"
a = 3

def test_1():
    global a
    if a == 0:
        assert True
    else:
        a -= 1
        assert False
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--retry").arg("5"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_1

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_parallel_worker_capping() {
    let context = TestContext::with_file(
        "test_a.py",
        r"
def test_1(): pass
def test_2(): pass
def test_3(): pass",
    );

    // With 3 tests and 8 requested workers, worker capping reduces to 1 worker
    // (ceil(3/5) = 1). The -v flag shows info logs confirming "Spawning 1 workers".
    assert_cmd_snapshot!(context.command().args(["-v", "--num-workers", "8"]), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test_a::test_1
            PASS [TIME] test_a::test_2
            PASS [TIME] test_a::test_3

    ────────────
         Summary [TIME] 3 tests run: 3 passed, 0 skipped

    ----- stderr -----
    INFO Collected all tests in [TIME]
    INFO Capped worker count to avoid underutilized workers total_tests=3 requested_workers=8 capped_workers=1
    INFO Spawning 1 workers
    INFO Worker 0 spawned with 3 tests
    INFO Waiting for 1 workers to complete (Ctrl+C to cancel)
    INFO Worker 0 completed successfully in [TIME]
    INFO All workers completed
    ");
}

#[test]
fn test_dry_run() {
    let context = TestContext::with_files([
        (
            "test_file1.py",
            r"
def test_alpha(): pass
def test_beta(): pass
",
        ),
        (
            "test_file2.py",
            r"
def test_gamma(): pass
",
        ),
    ]);

    assert_cmd_snapshot!(context.command().arg("--dry-run"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    <test> test_file1::test_alpha
    <test> test_file1::test_beta
    <test> test_file2::test_gamma

    3 tests collected

    ----- stderr -----
    ");
}

#[test]
fn test_dry_run_empty() {
    let context = TestContext::new();

    assert_cmd_snapshot!(context.command().arg("--dry-run"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    0 tests collected

    ----- stderr -----
    ");
}

#[test]
fn test_dry_run_with_path_filter() {
    let context = TestContext::with_files([
        (
            "test_file1.py",
            r"
def test_alpha(): pass
def test_beta(): pass
",
        ),
        (
            "test_file2.py",
            r"
def test_gamma(): pass
",
        ),
    ]);

    assert_cmd_snapshot!(context.command().args(["--dry-run", "test_file1.py"]), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    <test> test_file1::test_alpha
    <test> test_file1::test_beta

    2 tests collected

    ----- stderr -----
    ");
}

#[test]
fn test_concise_output_format_with_failure() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_pass():
    assert True

def test_fail():
    assert False
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--output-format=concise"), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_pass
            FAIL [TIME] test::test_fail

    diagnostics:

    test.py:5:5: error[test-failure] Test `test_fail` failed

    ────────────
         Summary [TIME] 2 tests run: 1 passed, 1 failed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_concise_output_format_with_discovery_error() {
    let context = TestContext::with_file(
        "test.py",
        r"
import nonexistent_module_xyz

def test_pass():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--output-format=concise"), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 1 test across 1 worker
    diagnostics:

    error[failed-to-import-module] Failed to import python module `test`: No module named 'nonexistent_module_xyz'

    ────────────
         Summary [TIME] 0 tests run: 0 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_collection_error_with_passing_tests_exits_nonzero() {
    let context = TestContext::with_files([
        (
            "test_bad.py",
            r"
import nonexistent_module_xyz

def test_unreachable():
    assert True
            ",
        ),
        (
            "test_good.py",
            r"
def test_pass():
    assert True
            ",
        ),
    ]);

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: false
    exit_code: 1
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test_good::test_pass

    diagnostics:

    error[failed-to-import-module]: Failed to import python module `test_bad`: No module named 'nonexistent_module_xyz'

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

/// `--max-fail=2` should run exactly two failing tests and then stop scheduling
/// the rest. The summary reflects only the tests that actually ran.
#[test]
fn test_max_fail_stops_after_n_failures() {
    let context = TestContext::with_file(
        "test_max_fail.py",
        r"
def test_first_fail():
    assert False, 'boom 1'

def test_second_fail():
    assert False, 'boom 2'

def test_third_fail():
    assert False, 'boom 3'

def test_fourth_skipped():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--max-fail=2"));
}

/// `--max-fail=all` disables the limit, so every test runs even when some fail.
#[test]
fn test_max_fail_all_runs_every_test() {
    let context = TestContext::with_file(
        "test_max_fail_all.py",
        r"
def test_a():
    assert False, 'a boom'

def test_b():
    assert False, 'b boom'

def test_c():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--max-fail=all"));
}

/// `--max-fail=1` is the generalized form of `--fail-fast` and should stop
/// scheduling once a single test has failed.
#[test]
fn test_max_fail_one_is_equivalent_to_fail_fast() {
    let context = TestContext::with_file(
        "test_max_fail_one.py",
        r"
def test_first():
    assert True

def test_second():
    assert False, 'stop here'

def test_third():
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--max-fail=1"));
}

#[test]
fn test_fail_fast() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_1():
    assert True

def test_2():
    assert False

def test_3():
    assert True
        ",
    );

    let output = context
        .command_no_parallel()
        .arg("--fail-fast")
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success());
    assert!(
        stdout.contains("PASS") && stdout.contains("test::test_1"),
        "first test should pass"
    );
    assert!(
        stdout.contains("FAIL") && stdout.contains("test::test_2"),
        "second test should fail"
    );
    assert!(
        !stdout.contains("test::test_3"),
        "third test should not run due to --fail-fast"
    );
}

#[test]
fn test_fail_fast_across_modules() {
    let context = TestContext::with_files([
        (
            "test_a.py",
            r"
def test_a_fail():
    assert False
            ",
        ),
        (
            "test_b.py",
            r"
def test_b_pass():
    assert True
            ",
        ),
    ]);

    let output = context
        .command_no_parallel()
        .arg("--fail-fast")
        .arg("-q")
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("failed"), "should report failure");
}

#[test]
fn test_dry_run_nested_packages() {
    let context = TestContext::with_files([
        (
            "tests/test_root.py",
            r"
def test_root(): pass
            ",
        ),
        (
            "tests/sub/test_nested.py",
            r"
def test_nested(): pass
            ",
        ),
    ]);

    assert_cmd_snapshot!(context.command().arg("--dry-run"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    <test> tests.sub.test_nested::test_nested
    <test> tests.test_root::test_root

    2 tests collected

    ----- stderr -----
    ");
}

#[test]
fn test_show_python_output() {
    let context = TestContext::with_file(
        "test.py",
        r#"
def test_with_print():
    print("hello from test")
    assert True
        "#,
    );

    assert_cmd_snapshot!(context.command().arg("-s"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
    hello from test
            PASS [TIME] test::test_with_print

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_retry_flag() {
    let context = TestContext::with_file(
        "test.py",
        r"
counter = 0

def test_flaky():
    global counter
    counter += 1
    assert counter >= 2
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--retry=2"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_flaky

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_extra_verbose_output() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_1(): pass
def test_2(): pass
",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-vv"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_1
            PASS [TIME] test::test_2

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    [DATETIME] DEBUG Working directory cwd=<temp_dir>/
    [DATETIME] DEBUG Searching for a project in '<temp_dir>/'
    [DATETIME] DEBUG The ancestor directories contain no `pyproject.toml`. Falling back to a virtual project.
    [DATETIME] DEBUG Found test paths path_count=1
    [DATETIME] INFO Collected all tests in [TIME]
    [DATETIME] DEBUG Partitioning tests num_workers=1
    [DATETIME] INFO Spawning 1 workers
    [DATETIME] INFO Worker 0 spawned with 2 tests
    [DATETIME] INFO Waiting for 1 workers to complete (Ctrl+C to cancel)
    [DATETIME] DEBUG Trying to parse `monkeypatch` as a fixture
    [DATETIME] DEBUG Trying to parse `capsys` as a fixture
    [DATETIME] DEBUG Trying to parse `capfd` as a fixture
    [DATETIME] DEBUG Trying to parse `capsysbinary` as a fixture
    [DATETIME] DEBUG Trying to parse `capfdbinary` as a fixture
    [DATETIME] DEBUG Trying to parse `caplog` as a fixture
    [DATETIME] DEBUG Trying to parse `tmp_path` as a fixture
    [DATETIME] DEBUG Trying to parse `temp_path` as a fixture
    [DATETIME] DEBUG Trying to parse `temp_dir` as a fixture
    [DATETIME] DEBUG Trying to parse `tmpdir` as a fixture
    [DATETIME] DEBUG Trying to parse `tmp_path_factory` as a fixture
    [DATETIME] DEBUG Trying to parse `tmpdir_factory` as a fixture
    [DATETIME] DEBUG Trying to parse `recwarn` as a fixture
    [DATETIME] DEBUG Running test `test::test_1`
    [DATETIME] DEBUG Running test `test::test_2`
    [DATETIME] INFO Worker 0 completed successfully in [TIME]
    [DATETIME] INFO All workers completed
    ");
}

#[test]
fn test_trace_verbose_output() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_1(): pass
",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-vvv"), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_1

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    1   [TIME] DEBUG karva::commands::test Working directory, cwd=<temp_dir>/
    1   [TIME] DEBUG karva_metadata Searching for a project in '<temp_dir>/'
    1   [TIME] DEBUG karva_metadata The ancestor directories contain no `pyproject.toml`. Falling back to a virtual project.
    1   [TIME] DEBUG karva_runner::orchestration Found test paths, path_count=1
    1   [TIME] INFO karva_runner::orchestration Collected all tests in [TIME]
    1   [TIME] DEBUG karva_runner::orchestration Partitioning tests, num_workers=1
    1   [TIME] INFO karva_runner::orchestration Spawning 1 workers
    1   [TIME] INFO karva_runner::orchestration Worker 0 spawned with 1 tests
    1   [TIME] INFO karva_runner::orchestration Waiting for 1 workers to complete (Ctrl+C to cancel)
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `monkeypatch` as a fixture
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `capsys` as a fixture
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `capfd` as a fixture
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `capsysbinary` as a fixture
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `capfdbinary` as a fixture
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `caplog` as a fixture
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `tmp_path` as a fixture
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `temp_path` as a fixture
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `temp_dir` as a fixture
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `tmpdir` as a fixture
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `tmp_path_factory` as a fixture
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `tmpdir_factory` as a fixture
    1   [TIME] DEBUG karva_test_semantic::extensions::fixtures Trying to parse `recwarn` as a fixture
    1   [TIME] DEBUG karva_test_semantic::runner::package_runner Running test `test::test_1`
    1   [TIME] INFO karva_runner::orchestration Worker 0 completed successfully in [TIME]
    1   [TIME] INFO karva_runner::orchestration All workers completed
    ");
}

#[test]
fn test_quiet_output() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_1(): pass
def test_2(): pass
",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("-q"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

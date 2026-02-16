use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

#[test]
fn test_async_function() {
    let context = TestContext::with_file(
        "test.py",
        r"
import asyncio

async def test_async_passes():
    await asyncio.sleep(0)
    assert True
        ",
    );

    assert_cmd_snapshot!(context.command(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_async_passes ... ok

    test result: ok. 1 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");
}

#[test]
fn test_async_function_with_assertion_error() {
    let context = TestContext::with_file(
        "test.py",
        r"
import asyncio

async def test_async_fails():
    await asyncio.sleep(0)
    assert False, 'async test failed'
        ",
    );

    assert_cmd_snapshot!(context.command(), @r"
    success: false
    exit_code: 1
    ----- stdout -----
    test test::test_async_fails ... FAILED

    diagnostics:

    error[test-failure]: Test `test_async_fails` failed
     --> test.py:4:11
      |
    2 | import asyncio
    3 |
    4 | async def test_async_fails():
      |           ^^^^^^^^^^^^^^^^
    5 |     await asyncio.sleep(0)
    6 |     assert False, 'async test failed'
      |
    info: Test failed here
     --> test.py:6:5
      |
    4 | async def test_async_fails():
    5 |     await asyncio.sleep(0)
    6 |     assert False, 'async test failed'
      |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      |
    info: async test failed

    test result: FAILED. 0 passed; 1 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");
}

#[test]
fn test_async_fixture() {
    let context = TestContext::with_file(
        "test.py",
        r"
import asyncio
import karva

@karva.fixture
async def async_value():
    await asyncio.sleep(0)
    return 42

async def test_with_async_fixture(async_value):
    assert async_value == 42
        ",
    );

    assert_cmd_snapshot!(context.command(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_with_async_fixture(async_value=42) ... ok

    test result: ok. 1 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");
}

#[test]
fn test_async_generator_fixture() {
    let context = TestContext::with_files([(
        "test.py",
        r"
import asyncio
import karva

setup_done = False
teardown_done = False

@karva.fixture
async def async_resource():
    global setup_done
    setup_done = True
    await asyncio.sleep(0)
    yield 'resource'
    global teardown_done
    teardown_done = True

async def test_async_gen_fixture(async_resource):
    assert async_resource == 'resource'
    assert setup_done is True

def test_teardown_ran():
    assert teardown_done is True
        ",
    )]);

    assert_cmd_snapshot!(context.command_no_parallel(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_async_gen_fixture(async_resource=resource) ... ok
    test test::test_teardown_ran ... ok

    test result: ok. 2 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");
}

#[test]
fn test_async_test_with_sync_fixture() {
    let context = TestContext::with_file(
        "test.py",
        r"
import asyncio
import karva

@karva.fixture
def sync_value():
    return 10

async def test_async_with_sync(sync_value):
    await asyncio.sleep(0)
    assert sync_value == 10
        ",
    );

    assert_cmd_snapshot!(context.command(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_async_with_sync(sync_value=10) ... ok

    test result: ok. 1 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");
}

#[test]
fn test_sync_test_with_async_fixture() {
    let context = TestContext::with_file(
        "test.py",
        r"
import asyncio
import karva

@karva.fixture
async def async_value():
    await asyncio.sleep(0)
    return 99

def test_sync_with_async(async_value):
    assert async_value == 99
        ",
    );

    assert_cmd_snapshot!(context.command(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_sync_with_async(async_value=99) ... ok

    test result: ok. 1 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");
}

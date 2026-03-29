use insta::allow_duplicates;
use insta_cmd::assert_cmd_snapshot;
use rstest::rstest;

use crate::common::TestContext;

#[rstest]
fn test_temp_directory_fixture(
    #[values("tmp_path", "temp_path", "temp_dir", "tmpdir")] fixture_name: &str,
) {
    let test_context = TestContext::with_file(
        "test.py",
        &format!(
            r"
                import pathlib

                def test_temp_directory_fixture({fixture_name}):
                    assert {fixture_name}.exists()
                    assert {fixture_name}.is_dir()
                    assert {fixture_name}.is_absolute()
                    assert isinstance({fixture_name}, pathlib.Path)
                "
        ),
    );

    allow_duplicates! {
        assert_cmd_snapshot!(test_context.command().arg("-q"), @"
        success: true
        exit_code: 0
        ----- stdout -----
        ────────────
             Summary [TIME] 1 test run: 1 passed, 0 skipped

        ----- stderr -----
        ");
    }
}

#[test]
fn test_monkeypatch_setattr() {
    let context = TestContext::with_file(
        "test.py",
        r"
from karva import MockEnv

def test_setattr_simple(monkeypatch):
    class A:
        x = 1

    monkeypatch.setattr(A, 'x', 2)
    assert A.x == 2

def test_setattr_new_attribute(monkeypatch):
    class A:
        x = 1

    monkeypatch.setattr(A, 'y', 2, raising=False)
    assert A.y == 2

def test_setattr_undo(monkeypatch):
    class A:
        x = 1

    monkeypatch.setattr(A, 'x', 2)
    assert A.x == 2
    monkeypatch.undo()
    assert A.x == 1
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test::test_setattr_simple(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_setattr_new_attribute(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_setattr_undo(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 3 tests run: 3 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_setitem() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_setitem_dict(monkeypatch):
    d = {'x': 1}
    monkeypatch.setitem(d, 'x', 2)
    assert d['x'] == 2

def test_setitem_new_key(monkeypatch):
    d = {'x': 1}
    monkeypatch.setitem(d, 'y', 2)
    assert d['y'] == 2
    monkeypatch.undo()
    assert 'y' not in d

def test_setitem_undo(monkeypatch):
    d = {'x': 1}
    monkeypatch.setitem(d, 'x', 2)
    monkeypatch.undo()
    assert d['x'] == 1
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test::test_setitem_dict(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_setitem_new_key(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_setitem_undo(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 3 tests run: 3 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_env() {
    let context = TestContext::with_file(
        "test.py",
        r"
import os

def test_setenv(monkeypatch):
    monkeypatch.setenv('TEST_VAR', 'test_value')
    assert os.environ['TEST_VAR'] == 'test_value'

def test_setenv_undo(monkeypatch):
    monkeypatch.setenv('TEST_VAR_2', 'test_value')
    assert os.environ['TEST_VAR_2'] == 'test_value'
    monkeypatch.undo()
    assert 'TEST_VAR_2' not in os.environ

def test_delenv(monkeypatch):
    os.environ['TEST_VAR_3'] = 'value'
    monkeypatch.delenv('TEST_VAR_3')
    assert 'TEST_VAR_3' not in os.environ
    monkeypatch.undo()
    assert os.environ['TEST_VAR_3'] == 'value'
    del os.environ['TEST_VAR_3']
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 3 tests across 1 worker
            PASS [TIME] test::test_setenv(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_setenv_undo(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_delenv(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 3 tests run: 3 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_syspath() {
    let context = TestContext::with_file(
        "test.py",
        r"
import sys

def test_syspath_prepend(monkeypatch):
    old_path = sys.path.copy()
    monkeypatch.syspath_prepend('/test/path')
    assert sys.path[0] == '/test/path'
    monkeypatch.undo()
    assert sys.path == old_path

def test_syspath_prepend_multiple(monkeypatch):
    old_path = sys.path.copy()
    monkeypatch.syspath_prepend('/first')
    monkeypatch.syspath_prepend('/second')
    assert sys.path[0] == '/second'
    assert sys.path[1] == '/first'
    monkeypatch.undo()
    assert sys.path == old_path
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_syspath_prepend(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_syspath_prepend_multiple(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_delattr() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_delattr(monkeypatch):
    class A:
        x = 1

    monkeypatch.delattr(A, 'x')
    assert not hasattr(A, 'x')

def test_delattr_undo(monkeypatch):
    class A:
        x = 1

    monkeypatch.delattr(A, 'x')
    assert not hasattr(A, 'x')
    monkeypatch.undo()
    assert A.x == 1
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_delattr(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_delattr_undo(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_context_manager() {
    let context = TestContext::with_file(
        "test.py",
        r"
from karva import MockEnv

def test_context_manager():
    class A:
        x = 1

    with MockEnv() as m:
        m.setattr(A, 'x', 2)
        assert A.x == 2

    assert A.x == 1

def test_context_manager_auto_undo():
    d = {'x': 1}

    with MockEnv() as m:
        m.setitem(d, 'x', 2)
        assert d['x'] == 2

    assert d['x'] == 1
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_context_manager
            PASS [TIME] test::test_context_manager_auto_undo

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

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
    monkeypatch.setenv('TEST_VAR_4', 'test_value')
    assert os.environ['TEST_VAR_4'] == 'test_value'

def test_1():
    assert 'TEST_VAR_4' not in os.environ
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
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
fn test_monkeypatch_setattr_dotted_import_path() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import os

def test_setattr_dotted_path(monkeypatch):
    monkeypatch.setattr("os.sep", "X")
    assert os.sep == "X"

def test_setattr_dotted_path_undo(monkeypatch):
    original = os.sep
    monkeypatch.setattr("os.sep", "Y")
    assert os.sep == "Y"
    monkeypatch.undo()
    assert os.sep == original
        "#,
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_setattr_dotted_path(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_setattr_dotted_path_undo(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_delattr_dotted_import_path() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import os

def test_delattr_dotted_path(monkeypatch):
    monkeypatch.delattr("os.sep")
    assert not hasattr(os, "sep")

def test_delattr_dotted_path_undo(monkeypatch):
    original = os.sep
    monkeypatch.delattr("os.sep")
    assert not hasattr(os, "sep")
    monkeypatch.undo()
    assert os.sep == original
        "#,
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_delattr_dotted_path(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_delattr_dotted_path_undo(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_context_classmethod() {
    let context = TestContext::with_file(
        "test.py",
        r"
from karva import MockEnv

def test_context_classmethod():
    class A:
        x = 1

    with MockEnv.context() as mp:
        mp.setattr(A, 'x', 2)
        assert A.x == 2
        mp.undo()

    assert A.x == 1
        ",
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_context_classmethod

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_delitem_raising() {
    let context = TestContext::with_file(
        "test.py",
        r"
import karva

def test_delitem_raises_key_error(monkeypatch):
    d = {}
    with karva.raises(KeyError):
        monkeypatch.delitem(d, 'missing')

def test_delitem_not_raising(monkeypatch):
    d = {}
    monkeypatch.delitem(d, 'missing', raising=False)
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_delitem_raises_key_error(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_delitem_not_raising(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

/// Taken from <https://github.com/pytest-dev/pytest/blob/main/testing/test_monkeypatch.py>
#[test]
fn test_mock_env() {
    let context = TestContext::with_file(
        "test.py",
        r#"
            import os
            import re
            import sys
            from collections.abc import Generator
            from pathlib import Path

            import karva
            import pytest
            from karva import MockEnv

            skip_macos = karva.tags.skip(sys.platform == "darwin")

            @karva.fixture
            def mp() -> Generator[MockEnv]:
                cwd = os.getcwd()
                sys_path = list(sys.path)
                yield MockEnv()
                sys.path[:] = sys_path
                os.chdir(cwd)


            def test_setattr() -> None:
                class A:
                    x = 1

                monkeypatch = MockEnv()
                pytest.raises(AttributeError, monkeypatch.setattr, A, "notexists", 2)
                monkeypatch.setattr(A, "y", 2, raising=False)
                assert A.y == 2  # ty: ignore
                monkeypatch.undo()
                assert not hasattr(A, "y")

                monkeypatch = MockEnv()
                monkeypatch.setattr(A, "x", 2)
                assert A.x == 2
                monkeypatch.setattr(A, "x", 3)
                assert A.x == 3
                monkeypatch.undo()
                assert A.x == 1

                A.x = 5
                monkeypatch.undo()  # double-undo makes no modification
                assert A.x == 5

                with pytest.raises(TypeError):
                    monkeypatch.setattr(A, "y")  # type: ignore[call-overload]


            def test_delattr() -> None:
                class A:
                    x = 1

                monkeypatch = MockEnv()
                monkeypatch.delattr(A, "x")
                assert not hasattr(A, "x")
                monkeypatch.undo()
                assert A.x == 1

                monkeypatch = MockEnv()
                monkeypatch.delattr(A, "x")
                pytest.raises(AttributeError, monkeypatch.delattr, A, "y")
                monkeypatch.delattr(A, "y", raising=False)
                monkeypatch.setattr(A, "x", 5, raising=False)
                assert A.x == 5
                monkeypatch.undo()
                assert A.x == 1


            def test_setitem() -> None:
                d = {"x": 1}
                monkeypatch = MockEnv()
                monkeypatch.setitem(d, "x", 2)
                monkeypatch.setitem(d, "y", 1700)
                monkeypatch.setitem(d, "y", 1700)
                assert d["x"] == 2
                assert d["y"] == 1700
                monkeypatch.setitem(d, "x", 3)
                assert d["x"] == 3
                monkeypatch.undo()
                assert d["x"] == 1
                assert "y" not in d
                d["x"] = 5
                monkeypatch.undo()
                assert d["x"] == 5


            def test_setitem_deleted_meanwhile() -> None:
                d: dict[str, object] = {}
                monkeypatch = MockEnv()
                monkeypatch.setitem(d, "x", 2)
                del d["x"]
                monkeypatch.undo()
                assert not d


            @pytest.mark.parametrize("before", [True, False])
            def test_setenv_deleted_meanwhile(before: bool) -> None:
                key = "qwpeoip123"
                if before:
                    os.environ[key] = "world"
                monkeypatch = MockEnv()
                monkeypatch.setenv(key, "hello")
                del os.environ[key]
                monkeypatch.undo()
                if before:
                    assert os.environ[key] == "world"
                    del os.environ[key]
                else:
                    assert key not in os.environ


            def test_delitem() -> None:
                d: dict[str, object] = {"x": 1}
                monkeypatch = MockEnv()
                monkeypatch.delitem(d, "x")
                assert "x" not in d
                monkeypatch.delitem(d, "y", raising=False)
                pytest.raises(KeyError, monkeypatch.delitem, d, "y")
                assert not d
                monkeypatch.setitem(d, "y", 1700)
                assert d["y"] == 1700
                d["hello"] = "world"
                monkeypatch.setitem(d, "x", 1500)
                assert d["x"] == 1500
                monkeypatch.undo()
                assert d == {"hello": "world", "x": 1}


            def test_setenv() -> None:
                monkeypatch = MockEnv()
                monkeypatch.setenv("XYZ123", 2)  # type: ignore[arg-type]
                import os

                assert os.environ["XYZ123"] == "2"
                monkeypatch.undo()
                assert "XYZ123" not in os.environ


            def test_delenv() -> None:
                name = "xyz1234"
                assert name not in os.environ
                monkeypatch = MockEnv()
                pytest.raises(KeyError, monkeypatch.delenv, name, raising=True)
                monkeypatch.delenv(name, raising=False)
                monkeypatch.undo()
                os.environ[name] = "1"
                try:
                    monkeypatch = MockEnv()
                    monkeypatch.delenv(name)
                    assert name not in os.environ
                    monkeypatch.setenv(name, "3")
                    assert os.environ[name] == "3"
                    monkeypatch.undo()
                    assert os.environ[name] == "1"
                finally:
                    if name in os.environ:
                        del os.environ[name]

            def test_setenv_prepend() -> None:
                import os

                monkeypatch = MockEnv()
                monkeypatch.setenv("XYZ123", "2", prepend="-")
                monkeypatch.setenv("XYZ123", "3", prepend="-")
                assert os.environ["XYZ123"] == "3-2"
                monkeypatch.undo()
                assert "XYZ123" not in os.environ


            def test_syspath_prepend(mp: MockEnv) -> None:
                old = list(sys.path)
                mp.syspath_prepend("world")
                mp.syspath_prepend("hello")
                assert sys.path[0] == "hello"
                assert sys.path[1] == "world"
                mp.undo()
                assert sys.path == old
                mp.undo()
                assert sys.path == old


            def test_syspath_prepend_double_undo(mp: MockEnv) -> None:
                old_syspath = sys.path[:]
                try:
                    mp.syspath_prepend("hello world")
                    mp.undo()
                    sys.path.append("more hello world")
                    mp.undo()
                    assert sys.path[-1] == "more hello world"
                finally:
                    sys.path[:] = old_syspath


            @skip_macos
            def test_chdir_with_path_local(mp: MockEnv, tmp_path: Path) -> None:
                mp.chdir(tmp_path)
                assert os.getcwd() == str(tmp_path), f"Expected {str(tmp_path)}, got {os.getcwd()}"

            @skip_macos
            def test_chdir_with_str(mp: MockEnv, tmp_path: Path) -> None:
                mp.chdir(str(tmp_path))
                assert os.getcwd() == str(tmp_path), f"Expected {str(tmp_path)}, got {os.getcwd()}"


            def test_chdir_undo(mp: MockEnv, tmp_path: Path) -> None:
                cwd = os.getcwd()
                mp.chdir(tmp_path)
                mp.undo()
                assert os.getcwd() == cwd


            @skip_macos
            def test_chdir_double_undo(mp: MockEnv, tmp_path: Path) -> None:
                mp.chdir(str(tmp_path))
                mp.undo()
                os.chdir(tmp_path)
                mp.undo()
                assert os.getcwd() == str(tmp_path), f"Expected {str(tmp_path)}, got {os.getcwd()}"
                "#,
    );

    if cfg!(target_os = "macos") {
        assert_cmd_snapshot!(context.command().arg("-q"), @"
        success: true
        exit_code: 0
        ----- stdout -----
        ────────────
             Summary [TIME] 16 tests run: 13 passed, 3 skipped

        ----- stderr -----
        ");
    } else {
        assert_cmd_snapshot!(context.command().arg("-q"), @"
        success: true
        exit_code: 0
        ----- stdout -----
        ────────────
             Summary [TIME] 16 tests run: 16 passed, 0 skipped

        ----- stderr -----
        ");
    }
}

#[test]
fn test_monkeypatch_setattr_non_absolute_path() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_setattr_non_absolute(monkeypatch):
    with karva.raises(AttributeError, match="must be absolute import path string"):
        monkeypatch.setattr("simple_name", "value")
        "#,
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_setattr_non_absolute(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_delattr_string_target_with_name() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_delattr_string_with_name(monkeypatch):
    with karva.raises(AttributeError, match="use delattr"):
        monkeypatch.delattr("os.sep", "extra_name")
        "#,
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_delattr_string_with_name(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_delattr_missing_name() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_delattr_no_name(monkeypatch):
    class A:
        x = 1
    with karva.raises(AttributeError, match="use delattr"):
        monkeypatch.delattr(A)
        "#,
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_delattr_no_name(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_setattr_nonexistent_module() {
    let context = TestContext::with_file(
        "test.py",
        r#"
def test_setattr_nonexistent(monkeypatch):
    try:
        monkeypatch.setattr("nonexistent_module_xyz123.attr", "value")
        assert False, "Should have raised"
    except (ImportError, ModuleNotFoundError):
        pass
        "#,
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_setattr_nonexistent(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_setattr_nonexistent_attr_raising() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_setattr_missing_attr(monkeypatch):
    with karva.raises(AttributeError):
        monkeypatch.setattr("os.nonexistent_attr_xyz123", "value")
        "#,
    );

    assert_cmd_snapshot!(context.command(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_setattr_missing_attr(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_setattr_dotted_string() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import os

def test_setattr_dotted_none(monkeypatch):
    monkeypatch.setattr("os.sep", None)
    assert os.sep is None

def test_setattr_dotted_none_undo(monkeypatch):
    original = os.sep
    monkeypatch.setattr("os.sep", None)
    assert os.sep is None
    monkeypatch.undo()
    assert os.sep == original
        "#,
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_setattr_dotted_none(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_setattr_dotted_none_undo(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_monkeypatch_chdir_with_tmp_path() {
    let context = TestContext::with_file(
        "test.py",
        r"
import os

def test_chdir(monkeypatch, tmp_path):
    original = os.getcwd()
    monkeypatch.chdir(tmp_path)
    assert os.getcwd() != original
        ",
    );

    assert_cmd_snapshot!(context.command().arg("-q"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

/// Regression test: `undo` must restore `None` values rather than deleting the attribute.
/// Previously, `None` was used as the sentinel meaning "attribute didn't exist", so
/// patching an attribute whose original value was `None` would wrongly call `delattr`
/// on undo instead of restoring `None`.
#[test]
fn test_monkeypatch_setattr_none_value_undo() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_setattr_none_value_undo(monkeypatch):
    class A:
        x = None

    monkeypatch.setattr(A, 'x', 42)
    assert A.x == 42
    monkeypatch.undo()
    assert hasattr(A, 'x'), 'attribute should still exist after undo'
    assert A.x is None

def test_setitem_none_value_undo(monkeypatch):
    d = {'key': None}
    monkeypatch.setitem(d, 'key', 42)
    assert d['key'] == 42
    monkeypatch.undo()
    assert 'key' in d, 'key should still be present after undo'
    assert d['key'] is None
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_setattr_none_value_undo(monkeypatch=<MockEnv object>)
            PASS [TIME] test::test_setitem_none_value_undo(monkeypatch=<MockEnv object>)

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_caplog_records() {
    let context = TestContext::with_file(
        "test.py",
        r"
import logging

def test_caplog_records(caplog):
    with caplog.at_level(logging.WARNING):
        logging.warning('something happened')
    assert len(caplog.records) == 1
    assert caplog.records[0].levelname == 'WARNING'
    assert caplog.records[0].getMessage() == 'something happened'
        ",
    );

    assert_cmd_snapshot!(context.command().arg("-q"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_capsys_captures_stdout() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_capsys_stdout(capsys):
    print('hello')
    captured = capsys.readouterr()
    assert captured.out == 'hello\n'
    assert captured.err == ''
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_capsys_stdout(capsys=<CapsysFixture object>)

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_caplog_text() {
    let context = TestContext::with_file(
        "test.py",
        r"
import logging

def test_caplog_text(caplog):
    with caplog.at_level(logging.WARNING):
        logging.warning('text check')
    assert 'text check' in caplog.text
        ",
    );

    assert_cmd_snapshot!(context.command().arg("-q"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_capsys_captures_stderr() {
    let context = TestContext::with_file(
        "test.py",
        r"
import sys

def test_capsys_stderr(capsys):
    print('error message', file=sys.stderr)
    captured = capsys.readouterr()
    assert captured.out == ''
    assert captured.err == 'error message\n'
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_capsys_stderr(capsys=<CapsysFixture object>)

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_caplog_messages() {
    let context = TestContext::with_file(
        "test.py",
        r"
import logging

def test_caplog_messages(caplog):
    with caplog.at_level(logging.INFO):
        logging.info('first')
        logging.info('second')
    assert caplog.messages == ['first', 'second']
        ",
    );

    assert_cmd_snapshot!(context.command().arg("-q"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_capsys_readouterr_resets_buffer() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_capsys_reset(capsys):
    print('first')
    captured = capsys.readouterr()
    assert captured.out == 'first\n'

    print('second')
    captured = capsys.readouterr()
    assert captured.out == 'second\n', f'Expected only second, got: {captured.out!r}'
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_capsys_reset(capsys=<CapsysFixture object>)

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_caplog_at_level_filters() {
    let context = TestContext::with_file(
        "test.py",
        r"
import logging

def test_caplog_at_level_filters(caplog):
    with caplog.at_level(logging.WARNING):
        logging.debug('debug message')
        logging.info('info message')
        logging.warning('warning message')
    assert len(caplog.records) == 1
    assert caplog.records[0].levelname == 'WARNING'
        ",
    );

    assert_cmd_snapshot!(context.command().arg("-q"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_capsys_disabled() {
    let context = TestContext::with_file(
        "test.py",
        r"
def test_capsys_disabled(capsys):
    print('captured before')
    with capsys.disabled():
        # Anything printed here goes to real stdout; the test just checks no error is raised.
        pass
    print('captured after')
    captured = capsys.readouterr()
    assert 'captured after' in captured.out
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test::test_capsys_disabled(capsys=<CapsysFixture object>)

    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_caplog_clear() {
    let context = TestContext::with_file(
        "test.py",
        r"
import logging

def test_caplog_clear(caplog):
    with caplog.at_level(logging.WARNING):
        logging.warning('before clear')
    assert len(caplog.records) == 1
    caplog.clear()
    assert len(caplog.records) == 0
        ",
    );

    assert_cmd_snapshot!(context.command().arg("-q"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_caplog_set_level() {
    let context = TestContext::with_file(
        "test.py",
        r"
import logging

def test_caplog_set_level(caplog):
    caplog.set_level(logging.DEBUG)
    logging.debug('debug msg')
    logging.info('info msg')
    assert len(caplog.records) == 2
    assert caplog.records[0].levelname == 'DEBUG'
    assert caplog.records[1].levelname == 'INFO'
        ",
    );

    assert_cmd_snapshot!(context.command().arg("-q"), @"
    success: true
    exit_code: 0
    ----- stdout -----
    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

#[test]
fn test_caplog_finalizer_cleans_up() {
    let context = TestContext::with_file(
        "test.py",
        r"
import logging

def test_first(caplog):
    with caplog.at_level(logging.WARNING):
        logging.warning('test one')
    assert len(caplog.records) == 1

def test_second(caplog):
    assert len(caplog.records) == 0
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

/// Verify that `set_level()` changes are undone after the test completes so a
/// subsequent test without `caplog` sees the original root logger level.
#[test]
fn test_caplog_set_level_restored_after_teardown() {
    let context = TestContext::with_file(
        "test.py",
        r"
import logging

def test_sets_debug(caplog):
    caplog.set_level(logging.DEBUG)
    logging.debug('should be captured')
    assert len(caplog.records) == 1

def test_level_restored():
    # Root logger level must be back to WARNING (default) after the previous
    # test's caplog fixture is torn down, so a bare debug call emits nothing.
    import logging
    root = logging.getLogger()
    assert root.level == logging.WARNING, f'Expected WARNING, got {root.level}'
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

#[test]
fn test_capsys_restores_stdout_after_test() {
    let context = TestContext::with_file(
        "test.py",
        r"
import sys

def test_capsys_uses_capture(capsys):
    print('inside capsys test')
    captured = capsys.readouterr()
    assert captured.out == 'inside capsys test\n'

def test_stdout_works_after(capsys):
    # If capsys teardown didn't restore stdout, this would fail or hang.
    assert sys.stdout is not None
    print('after capsys test')
    captured = capsys.readouterr()
    assert 'after capsys test' in captured.out
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 2 tests across 1 worker
            PASS [TIME] test::test_capsys_uses_capture(capsys=<CapsysFixture object>)
            PASS [TIME] test::test_stdout_works_after(capsys=<CapsysFixture object>)

    ────────────
         Summary [TIME] 2 tests run: 2 passed, 0 skipped

    ----- stderr -----
    ");
}

"""Built-in fixtures provided by the Karva test framework.

These fixtures are automatically available to all tests without any imports.
"""

from __future__ import annotations

import contextlib
import os
from collections.abc import MutableMapping
from typing import Any, Self

from karva._karva import fixture


class _Missing:
    """Sentinel for "attribute or item did not exist before patching"."""

    def __repr__(self) -> str:
        return "<_Missing>"


_MISSING = _Missing()

# Alias the built-in setattr/delattr so methods with the same name can call them.
_builtin_setattr = setattr
_builtin_delattr = delattr
_builtin_getattr = getattr
_builtin_hasattr = hasattr


def _resolve(name: str) -> object:
    """Import and return the object identified by the dotted *name*.

    Traverses the attribute chain, importing submodules as needed — matching
    the resolution logic used by the Rust ``MockEnv`` implementation.
    """
    import importlib

    parts = name.split(".")
    current_path = parts[0]
    obj = importlib.import_module(current_path)

    for i, part in enumerate(parts[1:], 1):
        current_path = ".".join(parts[: i + 1])
        try:
            obj = _builtin_getattr(obj, part)
        except AttributeError:
            obj = importlib.import_module(current_path)

    return obj


def _derive_importpath(import_path: str, raising: bool = True) -> tuple[str, object]:
    """Split *import_path* into ``(attr_name, target_object)``.

    Raises ``AttributeError`` when the path has no dot or when *raising* is
    ``True`` and the attribute does not exist on the target.
    """
    if "." not in import_path:
        raise AttributeError(
            f"must be absolute import path string, not {import_path!r}"
        )
    module_path, attr = import_path.rsplit(".", 1)
    target = _resolve(module_path)
    if raising and not _builtin_hasattr(target, attr):
        type_name = type(target).__name__
        raise AttributeError(
            f"{type_name!r} object at {module_path} has no attribute {attr!r}"
        )
    return attr, target


class MockEnv:
    """Helper for patching attributes, items, environment variables, and more.

    Tracks every change so they can all be undone at once via :meth:`undo`.
    Instances can be used directly as context managers::

        with MockEnv() as mp:
            mp.setenv("HOME", "/tmp")
        # HOME is restored here

    Or obtained via the ``monkeypatch`` fixture::

        def test_something(monkeypatch):
            monkeypatch.setenv("HOME", "/tmp")
    """

    def __init__(self) -> None:
        self._setattr_ops: list[tuple[object, str, object]] = []
        self._setitem_ops: list[tuple[MutableMapping[Any, Any], Any, Any]] = []
        self._saved_cwd: str | None = None
        self._saved_syspath: list[str] | None = None

    def __repr__(self) -> str:
        return "<MockEnv object>"

    @classmethod
    def context(cls) -> _MockEnvContext:
        """Return a context manager that creates a fresh :class:`MockEnv` and undoes it on exit."""
        return _MockEnvContext()

    def __enter__(self) -> Self:
        return self

    def __exit__(self, *args: object) -> bool:
        self.undo()
        return False

    def setattr(self, *args: object, raising: bool = True) -> None:
        """Set an attribute on *target*, remembering the old value for :meth:`undo`.

        Accepts two call forms:

        - ``setattr(target, name, value)`` — direct object + attribute name + new value
        - ``setattr("module.attr", value)`` — dotted import string + new value
        """
        if len(args) == 2:
            target_str, value = args
            if not isinstance(target_str, str):
                raise TypeError(
                    "use setattr(target, name, value) or setattr(target, value)"
                    " with target being a dotted import string"
                )
            attr_name, target = _derive_importpath(target_str, raising)
        elif len(args) == 3:
            target, attr_name, value = args
            if not isinstance(attr_name, str):
                raise TypeError("attribute name must be a string")
        else:
            raise TypeError(
                f"setattr() takes 2 or 3 positional arguments but {len(args)} were given"
            )

        # For classes, read the raw __dict__ entry to capture descriptors correctly.
        if isinstance(target, type):
            old_val: object = target.__dict__.get(attr_name, _MISSING)
            if (
                isinstance(old_val, _Missing)
                and not _builtin_hasattr(target, attr_name)
                and raising
            ):
                raise AttributeError(f"{target!r} has no attribute {attr_name!r}")
        elif _builtin_hasattr(target, attr_name):
            old_val = _builtin_getattr(target, attr_name)
        elif raising:
            raise AttributeError(f"{target!r} has no attribute {attr_name!r}")
        else:
            old_val = _MISSING

        self._setattr_ops.append((target, str(attr_name), old_val))
        _builtin_setattr(target, str(attr_name), value)

    def delattr(
        self, target: object, name: object = None, raising: bool = True
    ) -> None:
        """Delete an attribute from *target*, remembering it for :meth:`undo`.

        Accepts two call forms:

        - ``delattr(target, name)`` — direct object + attribute name
        - ``delattr("module.attr")`` — dotted import string (``name`` is omitted)
        """
        if isinstance(target, str) and name is None:
            actual_attr, actual_target = _derive_importpath(target, raising)
        elif isinstance(target, str) and name is not None:
            raise AttributeError(
                "use delattr(target, name) or delattr(target) with target being"
                " a dotted import string"
            )
        else:
            if name is None:
                raise AttributeError(
                    "use delattr(target, name) or delattr(target) with target being"
                    " a dotted import string"
                )
            if not isinstance(name, str):
                raise TypeError("attribute name must be a string")
            actual_attr, actual_target = str(name), target

        if not _builtin_hasattr(actual_target, actual_attr):
            if raising:
                raise AttributeError(actual_attr)
            return

        if isinstance(actual_target, type):
            old_val: object = actual_target.__dict__.get(actual_attr, _MISSING)
        else:
            old_val = _builtin_getattr(actual_target, actual_attr, _MISSING)

        self._setattr_ops.append((actual_target, actual_attr, old_val))
        _builtin_delattr(actual_target, actual_attr)

    def setitem(self, dic: MutableMapping[Any, Any], name: Any, value: Any) -> None:
        """Set ``dic[name] = value``, remembering the old value for :meth:`undo`."""
        old_val: Any
        try:
            old_val = dic[name]
        except (KeyError, IndexError):
            old_val = _MISSING

        self._setitem_ops.append((dic, name, old_val))
        dic[name] = value

    def delitem(
        self, dic: MutableMapping[Any, Any], name: Any, raising: bool = True
    ) -> None:
        """Delete ``dic[name]``, remembering it for :meth:`undo`."""
        try:
            old_val = dic[name]
        except (KeyError, IndexError):
            if raising:
                raise
            return

        self._setitem_ops.append((dic, name, old_val))
        del dic[name]

    def setenv(self, name: str, value: object, prepend: str | None = None) -> None:
        """Set the environment variable *name* to *value*, remembering the old value."""
        value_str = str(value)

        if prepend is not None and name in os.environ:
            value_str = f"{value_str}{prepend}{os.environ[name]}"

        environ = os.environ
        old_val: object = environ.get(name, _MISSING)

        self._setitem_ops.append((environ, name, old_val))
        environ[name] = value_str

    def delenv(self, name: str, raising: bool = True) -> None:
        """Delete environment variable *name*, remembering it for :meth:`undo`."""
        environ = os.environ
        if name not in environ:
            if raising:
                raise KeyError(name)
            return

        old_val = environ[name]
        self._setitem_ops.append((environ, name, old_val))
        del environ[name]

    def syspath_prepend(self, path: str | os.PathLike[str]) -> None:
        """Prepend *path* to ``sys.path``, saving the original list for :meth:`undo`."""
        import importlib
        import sys

        path_str = os.fspath(path)

        if self._saved_syspath is None:
            self._saved_syspath = list(sys.path)

        sys.path.insert(0, path_str)
        importlib.invalidate_caches()

    def chdir(self, path: object) -> None:
        """Change the current working directory to *path*, saving the original for :meth:`undo`."""
        if self._saved_cwd is None:
            self._saved_cwd = os.getcwd()

        os.chdir(str(path))

    def undo(self) -> None:
        """Undo all patches in reverse order."""
        import sys

        for obj, name, old_val in reversed(self._setattr_ops):
            if isinstance(old_val, _Missing):
                with contextlib.suppress(AttributeError):
                    _builtin_delattr(obj, name)
            else:
                _builtin_setattr(obj, name, old_val)
        self._setattr_ops.clear()

        for dic, key, old_val in reversed(self._setitem_ops):
            if isinstance(old_val, _Missing):
                with contextlib.suppress(KeyError, IndexError):
                    del dic[key]
            else:
                dic[key] = old_val
        self._setitem_ops.clear()

        if self._saved_syspath is not None:
            sys.path[:] = self._saved_syspath
            self._saved_syspath = None

        if self._saved_cwd is not None:
            os.chdir(self._saved_cwd)
            self._saved_cwd = None


class _MockEnvContext:
    """Context manager that creates a :class:`MockEnv` and undoes it on exit."""

    def __init__(self) -> None:
        self._mock_env = MockEnv()

    def __enter__(self) -> MockEnv:
        return self._mock_env

    def __exit__(self, *args: object) -> bool:
        self._mock_env.undo()
        return False


@fixture
def monkeypatch():  # type: ignore[no-untyped-def]
    """Fixture that provides a :class:`MockEnv` for patching during a test."""
    mp = MockEnv()
    yield mp
    mp.undo()


def _capsys_impl():  # type: ignore[no-untyped-def]
    """Shared generator for capsys and capfd."""
    import io
    import logging
    import sys
    from typing import NamedTuple

    real_stdout = sys.stdout
    real_stderr = sys.stderr
    saved_disable = logging.root.manager.disable

    class CaptureResult(NamedTuple):
        out: str
        err: str

    class _CapsysFixture:
        def __init__(self) -> None:
            self._out: io.StringIO = io.StringIO()
            self._err: io.StringIO = io.StringIO()
            sys.stdout = self._out
            sys.stderr = self._err
            logging.disable(logging.NOTSET)

        def readouterr(self) -> object:
            """Return captured output and reset the buffers."""
            out = self._out.getvalue()
            err = self._err.getvalue()
            self._out = io.StringIO()
            self._err = io.StringIO()
            sys.stdout = self._out
            sys.stderr = self._err
            return CaptureResult(out, err)

        def disabled(self) -> object:
            """Context manager that temporarily restores real stdout/stderr."""
            cur_out = self._out
            cur_err = self._err

            class _Disabled:
                def __enter__(inner_self) -> object:
                    sys.stdout = real_stdout
                    sys.stderr = real_stderr
                    return inner_self

                def __exit__(inner_self, *args: object) -> bool:
                    sys.stdout = cur_out
                    sys.stderr = cur_err
                    return False

            return _Disabled()

        def __repr__(self) -> str:
            return "<CapsysFixture object>"

    f = _CapsysFixture()
    yield f
    sys.stdout = real_stdout
    sys.stderr = real_stderr
    logging.disable(saved_disable)


@fixture
def capsys():  # type: ignore[no-untyped-def]
    """Capture writes to ``sys.stdout`` and ``sys.stderr``."""
    yield from _capsys_impl()


@fixture
def capfd():  # type: ignore[no-untyped-def]
    """Capture writes to ``sys.stdout`` and ``sys.stderr`` (fd-level alias of capsys)."""
    yield from _capsys_impl()


def _capsysbinary_impl():  # type: ignore[no-untyped-def]
    """Shared generator for capsysbinary and capfdbinary."""
    import logging
    import sys
    from typing import NamedTuple

    real_stdout = sys.stdout
    real_stderr = sys.stderr
    saved_disable = logging.root.manager.disable

    class CaptureResult(NamedTuple):
        out: bytes
        err: bytes

    class _BinaryCaptureStream:
        """Accepts both str and bytes writes, stores raw bytes."""

        def __init__(self) -> None:
            self._data = bytearray()

        def write(self, obj: object) -> int:
            if isinstance(obj, str):
                b = obj.encode("utf-8")
            elif isinstance(obj, (bytes, bytearray)):
                b = bytes(obj)
            else:
                raise TypeError("write() argument must be str or bytes-like object")
            self._data += b
            return len(b)

        def flush(self) -> None:
            pass

        def getvalue(self) -> bytes:
            return bytes(self._data)

        @property
        def encoding(self) -> str:
            return "utf-8"

    class _CapsysBinaryFixture:
        def __init__(self) -> None:
            self._out = _BinaryCaptureStream()
            self._err = _BinaryCaptureStream()
            sys.stdout = self._out  # type: ignore[assignment]
            sys.stderr = self._err  # type: ignore[assignment]
            logging.disable(logging.NOTSET)

        def readouterr(self) -> object:
            """Return captured output as bytes and reset the buffers."""
            out = self._out.getvalue()
            err = self._err.getvalue()
            self._out = _BinaryCaptureStream()
            self._err = _BinaryCaptureStream()
            sys.stdout = self._out  # type: ignore[assignment]
            sys.stderr = self._err  # type: ignore[assignment]
            return CaptureResult(out, err)

        def disabled(self) -> object:
            """Context manager that temporarily restores real stdout/stderr."""
            cur_out = self._out
            cur_err = self._err

            class _Disabled:
                def __enter__(inner_self) -> object:
                    sys.stdout = real_stdout
                    sys.stderr = real_stderr
                    return inner_self

                def __exit__(inner_self, *args: object) -> bool:
                    sys.stdout = cur_out  # type: ignore[assignment]
                    sys.stderr = cur_err  # type: ignore[assignment]
                    return False

            return _Disabled()

        def __repr__(self) -> str:
            return "<CapsysBinaryFixture object>"

    f = _CapsysBinaryFixture()
    yield f
    sys.stdout = real_stdout
    sys.stderr = real_stderr
    logging.disable(saved_disable)


@fixture
def capsysbinary():  # type: ignore[no-untyped-def]
    """Capture writes to ``sys.stdout`` and ``sys.stderr`` as bytes."""
    yield from _capsysbinary_impl()


@fixture
def capfdbinary():  # type: ignore[no-untyped-def]
    """Capture writes to ``sys.stdout`` and ``sys.stderr`` as bytes (fd-level alias)."""
    yield from _capsysbinary_impl()


@fixture
def caplog():  # type: ignore[no-untyped-def]
    """Capture log records emitted during a test."""
    import logging

    saved_disable = logging.root.manager.disable

    records: list[logging.LogRecord] = []

    class _CapLogHandler(logging.Handler):
        def __init__(self) -> None:
            super().__init__(0)

        def emit(self, record: logging.LogRecord) -> None:
            record.message = record.getMessage()
            records.append(record)

    class _CapLogAtLevel:
        def __init__(
            self,
            handler: logging.Handler,
            level: int,
            logger_name: str | None,
        ) -> None:
            self._handler = handler
            self._level = level
            self._logger_name = logger_name
            self._prev_handler_level = handler.level
            self._prev_logger_level: int | None = None

        def __enter__(self) -> Self:
            logger = logging.getLogger(self._logger_name)
            self._prev_logger_level = logger.level
            logger.setLevel(self._level)
            self._handler.setLevel(self._level)
            return self

        def __exit__(self, *args: object) -> bool:
            logger = logging.getLogger(self._logger_name)
            if self._prev_logger_level is not None:
                logger.setLevel(self._prev_logger_level)
            self._handler.setLevel(self._prev_handler_level)
            return False

    class _CapLog:
        def __init__(self, handler: logging.Handler) -> None:
            self._handler = handler
            self._saved_level: int | None = None
            self._saved_level_logger: str | None = None

        @property
        def records(self) -> list[logging.LogRecord]:
            return records

        @property
        def handler(self) -> logging.Handler:
            return self._handler

        @property
        def record_tuples(self) -> list[tuple[str, int, str]]:
            return [(r.name, r.levelno, r.getMessage()) for r in records]

        @property
        def messages(self) -> list[str]:
            return [r.getMessage() for r in records]

        @property
        def text(self) -> str:
            formatter = logging.Formatter()
            return "\n".join(formatter.format(r) for r in records)

        def set_level(self, level: int, logger: str | None = None) -> None:
            """Set the capture level for the remainder of the test."""
            target = logging.getLogger(logger)
            if self._saved_level is None:
                self._saved_level = target.level
                self._saved_level_logger = logger
            target.setLevel(level)
            self._handler.setLevel(level)

        def at_level(self, level: int, logger: str | None = None) -> _CapLogAtLevel:
            """Context manager that temporarily sets the capture level."""
            return _CapLogAtLevel(self._handler, level, logger)

        def clear(self) -> None:
            """Clear all captured records."""
            records.clear()

        def __repr__(self) -> str:
            return "<CapLog object>"

    handler = _CapLogHandler()
    root_logger = logging.getLogger()
    root_logger.addHandler(handler)
    logging.disable(logging.NOTSET)

    cap = _CapLog(handler)
    yield cap

    root_logger.removeHandler(handler)
    logging.disable(saved_disable)
    if cap._saved_level is not None:
        logging.getLogger(cap._saved_level_logger).setLevel(cap._saved_level)


def _make_tmp_path() -> object:
    """Create a temporary directory and return its resolved :class:`pathlib.Path`."""
    import pathlib
    import tempfile

    # Use mkdtemp (no auto-cleanup) to match existing behaviour.
    return pathlib.Path(tempfile.mkdtemp(prefix="karva-")).resolve()


@fixture
def tmp_path():  # type: ignore[no-untyped-def]
    """Provide a temporary directory as a :class:`pathlib.Path` object."""
    yield _make_tmp_path()


@fixture
def temp_path():  # type: ignore[no-untyped-def]
    """Alias for :fixture:`tmp_path`."""
    yield _make_tmp_path()


@fixture
def temp_dir():  # type: ignore[no-untyped-def]
    """Alias for :fixture:`tmp_path`."""
    yield _make_tmp_path()


@fixture
def tmpdir():  # type: ignore[no-untyped-def]
    """Provide a temporary directory as a :class:`pathlib.Path`."""
    yield _make_tmp_path()


@fixture(scope="session")
def tmp_path_factory():  # type: ignore[no-untyped-def]
    """Session-scoped factory for creating numbered temporary directories."""
    import pathlib
    import tempfile

    base = pathlib.Path(tempfile.mkdtemp(prefix="karva-")).resolve()
    counter = [0]

    class _TmpPathFactory:
        def mktemp(self, basename: str, numbered: bool = True) -> pathlib.Path:
            """Create and return a new temporary subdirectory."""
            if numbered:
                name = f"{basename}{counter[0]}"
                counter[0] += 1
            else:
                name = basename
            path = base / name
            path.mkdir(parents=True, exist_ok=True)
            return path

        def getbasetemp(self) -> pathlib.Path:
            """Return the base temporary directory."""
            return base

        def __repr__(self) -> str:
            return f"<TmpPathFactory basetemp={base}>"

    yield _TmpPathFactory()


@fixture(scope="session")
def tmpdir_factory():  # type: ignore[no-untyped-def]
    """Session-scoped factory for creating numbered temporary directories."""
    import pathlib
    import tempfile

    base = pathlib.Path(tempfile.mkdtemp(prefix="karva-")).resolve()
    counter = [0]

    class _TmpDirFactory:
        def mktemp(self, basename: str, numbered: bool = True) -> pathlib.Path:
            """Create and return a new temporary subdirectory."""
            if numbered:
                name = f"{basename}{counter[0]}"
                counter[0] += 1
            else:
                name = basename
            path = base / name
            path.mkdir(parents=True, exist_ok=True)
            return path

        def getbasetemp(self) -> pathlib.Path:
            """Return the base temporary directory."""
            return base

        def __repr__(self) -> str:
            return f"<TmpDirFactory basetemp={base}>"

    yield _TmpDirFactory()


@fixture
def recwarn():  # type: ignore[no-untyped-def]
    """Record warnings raised during a test."""
    import warnings

    class _WarningsChecker:
        def __init__(self, warning_list: list) -> None:  # ty: ignore[invalid-type-form]
            self._list = warning_list

        @property
        def list(self) -> list:  # ty: ignore[invalid-type-form]
            return self._list

        def __len__(self) -> int:
            return len(self._list)

        def __getitem__(self, index: int) -> object:
            return self._list[index]

        def __iter__(self) -> object:
            return iter(self._list)

        def pop(self, category: type = Warning) -> object:
            """Remove and return the first warning of the given *category*."""
            for i, w in enumerate(self._list):
                if issubclass(w.category, category):
                    return self._list.pop(i)
            raise AssertionError(
                f"No warnings of type {category.__name__} were emitted."
            )

        def clear(self) -> None:
            """Clear all captured warnings."""
            self._list.clear()

        def __repr__(self) -> str:
            return "<WarningsChecker object>"

    with warnings.catch_warnings(record=True) as warning_list:
        warnings.simplefilter("always")
        yield _WarningsChecker(warning_list)

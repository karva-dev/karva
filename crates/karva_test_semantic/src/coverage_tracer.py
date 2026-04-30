"""Karva line-coverage tracer.

Installed by the karva worker via PyO3. Records every executed line under
the configured source roots, computes the set of executable lines via
`ast.walk`, and writes the result to a JSON file when stopped.

Output schema (`{data_file}`):
    {
      "files": {
        "<absolute path>": {
          "executable": [<line>, ...],
          "executed":   [<line>, ...]
        },
        ...
      }
    }
"""

from __future__ import annotations

import ast
import json
import os
import sys


def install(*, data_file: str, sources: list[str], cwd: str):
    roots = [os.path.realpath(s) for s in sources if s]
    if not roots:
        roots = [os.path.realpath(cwd)]

    excludes = ("/site-packages/", "/.venv/", "/dist-packages/", "/.tox/")

    executed: dict[str, set[int]] = {}
    track_cache: dict[str, bool] = {}

    def _should_track(filename: str) -> bool:
        cached = track_cache.get(filename)
        if cached is not None:
            return cached
        result = _compute_should_track(filename)
        track_cache[filename] = result
        return result

    def _compute_should_track(filename: str) -> bool:
        if not filename or filename.startswith("<"):
            return False
        try:
            absf = os.path.realpath(filename)
        except OSError:
            return False
        if any(ex in absf for ex in excludes):
            return False
        for root in roots:
            if absf == root or absf.startswith(root + os.sep):
                return True
        return False

    if sys.version_info >= (3, 12):
        stop = _install_monitoring(executed, _should_track)
    else:
        stop = _install_settrace(executed, _should_track)

    def _save() -> None:
        out_files: dict[str, dict[str, list[int]]] = {}
        for filename, hits in executed.items():
            executable = _executable_lines(filename)
            if not executable:
                continue
            out_files[filename] = {
                "executable": sorted(executable),
                "executed": sorted(hits & executable),
            }

        parent = os.path.dirname(data_file)
        if parent:
            os.makedirs(parent, exist_ok=True)
        with open(data_file, "w", encoding="utf-8") as f:
            json.dump({"files": out_files}, f)

    class _Controller:
        def stop(self) -> None:
            stop()
            _save()

    return _Controller()


def _install_monitoring(
    executed: dict[str, set[int]],
    should_track,
):
    mon = sys.monitoring
    tool_id = mon.COVERAGE_ID if hasattr(mon, "COVERAGE_ID") else 5

    try:
        mon.use_tool_id(tool_id, "karva")
    except ValueError:
        for candidate in range(6):
            try:
                mon.use_tool_id(candidate, "karva")
                tool_id = candidate
                break
            except ValueError:
                continue
        else:
            raise

    def line_cb(code, lineno):
        filename = code.co_filename
        if not should_track(filename):
            return mon.DISABLE
        bucket = executed.get(filename)
        if bucket is None:
            bucket = set()
            executed[filename] = bucket
        bucket.add(lineno)

    mon.register_callback(tool_id, mon.events.LINE, line_cb)
    mon.set_events(tool_id, mon.events.LINE)

    def stop():
        mon.set_events(tool_id, 0)
        mon.register_callback(tool_id, mon.events.LINE, None)
        mon.free_tool_id(tool_id)

    return stop


def _install_settrace(
    executed: dict[str, set[int]],
    should_track,
):
    def local_trace(frame, event, arg):
        if event == "line":
            filename = frame.f_code.co_filename
            bucket = executed.get(filename)
            if bucket is None:
                bucket = set()
                executed[filename] = bucket
            bucket.add(frame.f_lineno)
        return local_trace

    def trace(frame, event, arg):
        if event == "call":
            filename = frame.f_code.co_filename
            if should_track(filename):
                return local_trace
        return None

    sys.settrace(trace)

    def stop():
        sys.settrace(None)

    return stop


def _executable_lines(filename: str) -> set[int]:
    try:
        with open(filename, "rb") as f:
            source = f.read()
    except OSError:
        return set()

    try:
        tree = ast.parse(source, filename)
    except SyntaxError:
        return set()

    lines: set[int] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.stmt):
            lines.add(node.lineno)
    return lines

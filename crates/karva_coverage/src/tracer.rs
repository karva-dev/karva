//! Worker-side line tracer.
//!
//! Installs a Python tracer that records every executed line under the
//! configured source roots, then on stop computes executable lines for each
//! touched file and writes a per-worker JSON file at
//! [`CoverageConfig::data_file`].

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use camino::{Utf8Path, Utf8PathBuf};
use pyo3::prelude::*;

use crate::data::{FileEntry, WorkerFile};
use crate::executable::executable_lines;

/// Configuration for a single worker's coverage measurement.
#[derive(Debug, Clone)]
pub struct CoverageConfig {
    /// Source paths to measure. An empty entry means "measure the current
    /// working directory" (matches pytest-cov's bare `--cov`).
    pub sources: Vec<String>,

    /// Per-worker data file path. The runner combines these after the run.
    pub data_file: Utf8PathBuf,
}

/// Path components inside a source root that suppress tracking. These match
/// the conventional locations of installed third-party code.
const PATH_EXCLUDES: &[&str] = &["site-packages", "dist-packages", ".venv", ".tox"];

/// A live coverage measurement. Drop without calling [`Self::stop_and_save`]
/// to abandon a partial run; the data file is only persisted via
/// `stop_and_save`.
pub struct CoverageSession {
    tracer: Py<CoverageTracer>,
    data_file: Utf8PathBuf,
}

impl CoverageSession {
    pub fn start(py: Python<'_>, cwd: &Utf8Path, config: &CoverageConfig) -> PyResult<Self> {
        let roots: Vec<PathBuf> = config
            .sources
            .iter()
            .map(|s| {
                let raw = if s.is_empty() {
                    cwd.as_str()
                } else {
                    s.as_str()
                };
                std::fs::canonicalize(raw).unwrap_or_else(|_| PathBuf::from(raw))
            })
            .collect();

        let tracer = Py::new(
            py,
            CoverageTracer {
                roots,
                state: RefCell::new(TracerState::default()),
                monitoring_tool_id: Cell::new(None),
                monitoring_disable: RefCell::new(None),
            },
        )?;

        if py_version_at_least(py, 3, 12)? {
            install_monitoring(py, &tracer)?;
        } else {
            install_settrace(py, &tracer)?;
        }

        Ok(Self {
            tracer,
            data_file: config.data_file.clone(),
        })
    }

    pub fn stop_and_save(self, py: Python<'_>) -> PyResult<()> {
        let Self { tracer, data_file } = self;
        let bound = tracer.bind(py);
        let tool_id = bound.borrow().monitoring_tool_id.get();

        if let Some(tool_id) = tool_id {
            let mon = py.import("sys")?.getattr("monitoring")?;
            let line_event = mon.getattr("events")?.getattr("LINE")?;
            mon.call_method1("set_events", (tool_id, 0u32))?;
            mon.call_method1("register_callback", (tool_id, line_event, py.None()))?;
            mon.call_method1("free_tool_id", (tool_id,))?;
        } else {
            py.import("sys")?.call_method1("settrace", (py.None(),))?;
        }

        let executed = std::mem::take(&mut bound.borrow_mut().state.borrow_mut().executed);
        save_data(&data_file, executed).map_err(|err| {
            pyo3::exceptions::PyOSError::new_err(format!(
                "failed to write coverage data to {data_file}: {err}"
            ))
        })?;
        Ok(())
    }
}

#[derive(Default)]
struct TracerState {
    /// Files with the set of executed line numbers.
    executed: HashMap<PathBuf, HashSet<u32>>,
    /// Memoized result of [`compute_tracked_path`] per filename string.
    track_cache: HashMap<String, Option<PathBuf>>,
}

#[pyclass(module = "karva_coverage", unsendable)]
struct CoverageTracer {
    roots: Vec<PathBuf>,
    state: RefCell<TracerState>,
    monitoring_tool_id: Cell<Option<u8>>,
    /// Cached `sys.monitoring.DISABLE` sentinel. Populated when the
    /// `sys.monitoring` backend is installed; never accessed for the
    /// `sys.settrace` backend. Caching avoids importing `sys` inside the
    /// hot callback, which can re-enter the import system while `CPython`
    /// is mid-import and surface as `KeyError('__import__')`.
    monitoring_disable: RefCell<Option<Py<PyAny>>>,
}

#[pymethods]
impl CoverageTracer {
    /// `sys.monitoring` LINE event callback. Records the line if it's in a
    /// tracked file, then always returns `sys.monitoring.DISABLE` so the
    /// interpreter never calls us back for the same `(code, line)` pair.
    fn line_cb(
        &self,
        py: Python<'_>,
        code: &Bound<'_, PyAny>,
        lineno: u32,
    ) -> PyResult<Option<Py<PyAny>>> {
        let filename: String = code.getattr("co_filename")?.extract()?;
        if let Some(path) = self.tracked_path(&filename) {
            self.state
                .borrow_mut()
                .executed
                .entry(path)
                .or_default()
                .insert(lineno);
        }
        Ok(self
            .monitoring_disable
            .borrow()
            .as_ref()
            .map(|d| d.clone_ref(py)))
    }

    /// `sys.settrace` global trace function. Returns the per-frame
    /// [`Self::local_trace`] when the frame's file is under a source root.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "PyO3 requires Bound<Self> by value as a self receiver"
    )]
    fn trace<'py>(
        slf: Bound<'py, Self>,
        frame: &Bound<'py, PyAny>,
        event: &str,
        _arg: &Bound<'py, PyAny>,
    ) -> PyResult<Option<Py<PyAny>>> {
        if event == "call" {
            let filename: String = frame.getattr("f_code")?.getattr("co_filename")?.extract()?;
            if slf.borrow().tracked_path(&filename).is_some() {
                return Ok(Some(slf.getattr("local_trace")?.unbind()));
            }
        }
        Ok(None)
    }

    /// `sys.settrace` per-frame trace function. Records `line` events and
    /// returns itself so Python keeps tracing the frame.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "PyO3 requires Bound<Self> by value as a self receiver"
    )]
    fn local_trace<'py>(
        slf: Bound<'py, Self>,
        frame: &Bound<'py, PyAny>,
        event: &str,
        _arg: &Bound<'py, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        if event == "line" {
            let filename: String = frame.getattr("f_code")?.getattr("co_filename")?.extract()?;
            let path = slf.borrow().tracked_path(&filename);
            if let Some(path) = path {
                let lineno: u32 = frame.getattr("f_lineno")?.extract()?;
                slf.borrow()
                    .state
                    .borrow_mut()
                    .executed
                    .entry(path)
                    .or_default()
                    .insert(lineno);
            }
        }
        Ok(slf.getattr("local_trace")?.unbind())
    }
}

impl CoverageTracer {
    /// Resolve `filename` against the source roots. Returns the canonical
    /// path if the file should be tracked, or `None` otherwise. Memoized
    /// per filename string.
    fn tracked_path(&self, filename: &str) -> Option<PathBuf> {
        if let Some(cached) = self.state.borrow().track_cache.get(filename) {
            return cached.clone();
        }
        let resolved = compute_tracked_path(filename, &self.roots);
        self.state
            .borrow_mut()
            .track_cache
            .insert(filename.to_string(), resolved.clone());
        resolved
    }
}

fn compute_tracked_path(filename: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    if filename.is_empty() || filename.starts_with('<') {
        return None;
    }
    let canonical = std::fs::canonicalize(filename).ok()?;
    if canonical
        .components()
        .any(|c| PATH_EXCLUDES.contains(&c.as_os_str().to_str().unwrap_or("")))
    {
        return None;
    }
    for root in roots {
        if canonical == *root || canonical.starts_with(root) {
            return Some(canonical);
        }
    }
    None
}

fn py_version_at_least(py: Python<'_>, major: u8, minor: u8) -> PyResult<bool> {
    let info = py.import("sys")?.getattr("version_info")?;
    let actual_major: u8 = info.get_item(0)?.extract()?;
    let actual_minor: u8 = info.get_item(1)?.extract()?;
    Ok((actual_major, actual_minor) >= (major, minor))
}

fn install_monitoring(py: Python<'_>, tracer: &Py<CoverageTracer>) -> PyResult<()> {
    let mon = py.import("sys")?.getattr("monitoring")?;
    let line_event = mon.getattr("events")?.getattr("LINE")?;
    let disable = mon.getattr("DISABLE")?.unbind();

    let tool_id = (0u8..6u8)
        .find(|id| mon.call_method1("use_tool_id", (*id, "karva")).is_ok())
        .ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "no free sys.monitoring tool id available for coverage",
            )
        })?;

    let callback = tracer.bind(py).getattr("line_cb")?;
    mon.call_method1("register_callback", (tool_id, &line_event, callback))?;
    mon.call_method1("set_events", (tool_id, line_event))?;
    {
        let bound = tracer.bind(py).borrow();
        bound.monitoring_tool_id.set(Some(tool_id));
        *bound.monitoring_disable.borrow_mut() = Some(disable);
    }
    Ok(())
}

fn install_settrace(py: Python<'_>, tracer: &Py<CoverageTracer>) -> PyResult<()> {
    let trace = tracer.bind(py).getattr("trace")?;
    py.import("sys")?.call_method1("settrace", (trace,))?;
    Ok(())
}

fn save_data(
    data_file: &Utf8Path,
    executed: HashMap<PathBuf, HashSet<u32>>,
) -> std::io::Result<()> {
    let mut files = BTreeMap::new();
    for (path, hits) in executed {
        let executable = executable_lines(&path);
        if executable.is_empty() {
            continue;
        }
        let mut executed_lines: Vec<u32> = hits.intersection(&executable).copied().collect();
        executed_lines.sort_unstable();
        let mut executable_lines_vec: Vec<u32> = executable.into_iter().collect();
        executable_lines_vec.sort_unstable();
        files.insert(
            path.to_string_lossy().into_owned(),
            FileEntry {
                executable: executable_lines_vec,
                executed: executed_lines,
            },
        );
    }

    if let Some(parent) = data_file.parent()
        && !parent.as_str().is_empty()
    {
        std::fs::create_dir_all(parent.as_std_path())?;
    }
    let bytes = serde_json::to_vec(&WorkerFile { files })?;
    std::fs::write(data_file.as_std_path(), bytes)
}

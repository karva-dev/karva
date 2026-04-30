//! Line-coverage measurement, implemented in-tree.
//!
//! The worker installs a Python tracer (via `sys.monitoring` on 3.12+, or
//! `sys.settrace` on older versions) that records every executed line under
//! the configured source roots. On stop, the worker writes its hits and the
//! set of executable lines (computed via `ast.walk`) to a per-worker JSON
//! file. The main runner combines those files into a terminal report.

use camino::{Utf8Path, Utf8PathBuf};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

#[derive(Debug, Clone)]
pub struct CoverageConfig {
    /// Source paths to measure. An empty entry means "measure the current
    /// working directory" (matches pytest-cov's bare `--cov`).
    pub sources: Vec<String>,

    /// Per-worker data file path. The runner combines these after the run.
    pub data_file: Utf8PathBuf,
}

const TRACER_PY: &str = include_str!("coverage_tracer.py");

/// A live coverage measurement. Drop without calling [`stop_and_save`] to
/// abandon a partial run; the data file is only persisted via `stop_and_save`.
pub struct CoverageSession {
    controller: Py<PyAny>,
}

impl CoverageSession {
    pub fn start(py: Python<'_>, cwd: &Utf8Path, config: &CoverageConfig) -> PyResult<Self> {
        let resolved_sources: Vec<String> = config
            .sources
            .iter()
            .map(|s| {
                if s.is_empty() {
                    cwd.as_str().to_string()
                } else {
                    s.clone()
                }
            })
            .collect();

        let module = PyModule::from_code(
            py,
            std::ffi::CString::new(TRACER_PY)
                .expect("tracer source contains a NUL byte")
                .as_c_str(),
            std::ffi::CString::new("karva_coverage_tracer.py")
                .unwrap()
                .as_c_str(),
            std::ffi::CString::new("karva_coverage_tracer")
                .unwrap()
                .as_c_str(),
        )?;

        let kwargs = PyDict::new(py);
        kwargs.set_item("data_file", config.data_file.as_str())?;
        kwargs.set_item("sources", PyList::new(py, &resolved_sources)?)?;
        kwargs.set_item("cwd", cwd.as_str())?;

        let controller = module.getattr("install")?.call((), Some(&kwargs))?;

        Ok(Self {
            controller: controller.unbind(),
        })
    }

    pub fn stop_and_save(self, py: Python<'_>) -> PyResult<()> {
        self.controller.bind(py).call_method0("stop")?;
        Ok(())
    }
}

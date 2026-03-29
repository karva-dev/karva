use std::sync::{Arc, Mutex, MutexGuard};

use pyo3::PyResult;
use pyo3::exceptions::{PyAttributeError, PyRuntimeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple, PyType};

pub fn is_mock_env_fixture_name(fixture_name: &str) -> bool {
    matches!(fixture_name, "monkeypatch")
}

pub fn create_mock_env_fixture(py: Python<'_>) -> Option<(Py<PyAny>, Py<PyAny>)> {
    let mock = Py::new(py, MockEnv::new()).ok()?;
    let undo_method = mock.getattr(py, "undo").ok()?;

    // Return both the mock instance and its undo method as the finalizer
    Some((mock.into_any(), undo_method))
}

type SetAttrEntry = (Py<PyAny>, String, Py<PyAny>);
type SetItemEntry = (Py<PyAny>, Py<PyAny>, Py<PyAny>);
type SetAttr = Arc<Mutex<Vec<SetAttrEntry>>>;
type SetItem = Arc<Mutex<Vec<SetItemEntry>>>;

/// Sentinel type used to represent "value was absent before patching". Using a
/// dedicated Rust-backed type avoids the ambiguity of `None`, which is a
/// legitimate Python value an attribute or dict entry might actually hold.
#[pyclass]
struct Missing;

fn make_missing(py: Python<'_>) -> PyResult<Py<PyAny>> {
    Ok(Py::new(py, Missing)?.into_any())
}

fn is_missing(value: &Bound<'_, PyAny>) -> bool {
    value.is_instance_of::<Missing>()
}

/// Helper to conveniently monkeypatch attributes/items/environment variables/syspath.
#[pyclass]
pub struct MockEnv {
    setattr: SetAttr,
    setitem: SetItem,
    cwd: Arc<Mutex<Option<String>>>,
    savesyspath: Arc<Mutex<Option<Vec<String>>>>,
}

impl MockEnv {
    fn lock_setattr(&self) -> PyResult<MutexGuard<'_, Vec<SetAttrEntry>>> {
        self.setattr
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("lock poisoned: {e}")))
    }

    fn lock_setitem(&self) -> PyResult<MutexGuard<'_, Vec<SetItemEntry>>> {
        self.setitem
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("lock poisoned: {e}")))
    }

    fn lock_cwd(&self) -> PyResult<MutexGuard<'_, Option<String>>> {
        self.cwd
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("lock poisoned: {e}")))
    }

    fn lock_savesyspath(&self) -> PyResult<MutexGuard<'_, Option<Vec<String>>>> {
        self.savesyspath
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("lock poisoned: {e}")))
    }

    /// Restore all `setattr`/`delattr` changes in reverse order.
    fn undo_setattr(&self, py: Python<'_>) -> PyResult<()> {
        let mut setattr_list = self.lock_setattr()?;
        for (obj, name, value) in setattr_list.drain(..).rev() {
            if is_missing(value.bind(py)) {
                let _ = obj.bind(py).delattr(&name);
            } else {
                obj.bind(py).setattr(&name, value)?;
            }
        }
        Ok(())
    }

    /// Restore all `setitem`/`delitem` changes in reverse order.
    fn undo_setitem(&self, py: Python<'_>) -> PyResult<()> {
        let mut setitem_list = self.lock_setitem()?;
        for (dictionary, key, value) in setitem_list.drain(..).rev() {
            let bound_dict = dictionary.bind(py);

            if is_missing(value.bind(py)) {
                let _ = bound_dict.del_item(&key);
            } else {
                bound_dict.set_item(&key, value)?;
            }
        }
        Ok(())
    }

    /// Restore `sys.path` to its saved state.
    fn undo_syspath(&self, py: Python<'_>) -> PyResult<()> {
        let mut savesyspath = self.lock_savesyspath()?;
        if let Some(saved_path) = savesyspath.take() {
            drop(savesyspath);
            let sys_module = py.import("sys")?;
            let sys_path = sys_module.getattr("path")?;

            sys_path.call_method0("clear")?;
            for item in saved_path {
                sys_path.call_method1("append", (item,))?;
            }
        }
        Ok(())
    }

    /// Restore the working directory to its saved state.
    fn undo_cwd(&self, py: Python<'_>) -> PyResult<()> {
        let mut cwd = self.lock_cwd()?;
        if let Some(saved_cwd) = cwd.take() {
            drop(cwd);
            let os_module = py.import("os")?;
            os_module.call_method1("chdir", (saved_cwd,))?;
        }
        Ok(())
    }
}

#[pymethods]
impl MockEnv {
    #[new]
    fn new() -> Self {
        Self {
            setattr: Arc::new(Mutex::new(Vec::new())),
            setitem: Arc::new(Mutex::new(Vec::new())),
            cwd: Arc::new(Mutex::new(None)),
            savesyspath: Arc::new(Mutex::new(None)),
        }
    }

    /// Return a string representation of the Mock object.
    #[expect(clippy::unused_self)]
    pub fn __repr__(&self) -> String {
        "<MockEnv object>".to_string()
    }

    /// Context manager that returns a new Mock object which undoes any patching
    /// done inside the with block upon exit.
    #[classmethod]
    fn context(_cls: &Bound<'_, PyType>) -> MockEnvContext {
        MockEnvContext {
            mock_env: Self::new(),
        }
    }

    /// Set attribute value on target, memorising the old value.
    ///
    /// Accepts two call forms:
    /// - `setattr(target, name, value)` — direct object + attribute name + new value
    /// - `setattr("module.attr", value)` — dotted import string + new value
    ///
    /// Uses `*args` instead of named parameters so that an explicit `None` value
    /// is distinguishable from "argument not provided".
    #[pyo3(signature = (*args, raising = true))]
    fn setattr(
        &mut self,
        py: Python<'_>,
        args: &Bound<'_, PyTuple>,
        raising: bool,
    ) -> PyResult<()> {
        let (actual_target, actual_name, actual_value) = match args.len() {
            2 => {
                // Dotted import string form: setattr("module.attr", value)
                let target = args.get_item(0)?.unbind();
                let value = args.get_item(1)?.unbind();
                let target_str = target.extract::<String>(py).map_err(|_| {
                    PyTypeError::new_err(
                        "use setattr(target, name, value) or setattr(target, value) with target being a dotted import string"
                    )
                })?;
                let (attr_name, resolved_target) = derive_importpath(py, &target_str, raising)?;
                (resolved_target, attr_name, value)
            }
            3 => {
                // Direct form: setattr(obj, "attr", value)
                let target = args.get_item(0)?.unbind();
                let name = args.get_item(1)?.extract::<String>()?;
                let value = args.get_item(2)?.unbind();
                (target, name, value)
            }
            n => {
                return Err(PyTypeError::new_err(format!(
                    "setattr() takes 2 or 3 positional arguments but {n} were given"
                )));
            }
        };

        // Get old value
        let oldval = if let Ok(val) = actual_target.bind(py).getattr(&actual_name) {
            val.into()
        } else {
            if raising {
                return Err(PyAttributeError::new_err(format!(
                    "{actual_target:?} has no attribute {actual_name:?}"
                )));
            }
            make_missing(py)?
        };

        // Handle class descriptors
        let final_oldval = if actual_target
            .bind(py)
            .is_instance_of::<pyo3::types::PyType>()
        {
            match actual_target
                .bind(py)
                .getattr("__dict__")?
                .get_item(&actual_name)
                .ok()
            {
                Some(v) => v.into(),
                None => make_missing(py)?,
            }
        } else {
            oldval
        };

        // Store for undo
        self.lock_setattr()?.push((
            actual_target.clone_ref(py),
            actual_name.clone(),
            final_oldval,
        ));

        // Set new value
        actual_target.bind(py).setattr(&actual_name, actual_value)?;

        Ok(())
    }

    /// Delete attribute name from target.
    #[pyo3(signature = (target, name = None, raising = true))]
    fn delattr(
        &mut self,
        py: Python<'_>,
        target: Py<PyAny>,
        name: Option<Py<PyAny>>,
        raising: bool,
    ) -> PyResult<()> {
        let (actual_name, actual_target) = if target.bind(py).is_instance_of::<PyString>() {
            let target_str = target.extract::<String>(py)?;

            if name.is_some() {
                return Err(PyAttributeError::new_err(
                    "use delattr(target, name) or delattr(target) with target being a dotted import string",
                ));
            }

            derive_importpath(py, &target_str, raising)?
        } else {
            let name_str = name
                .ok_or_else(|| {
                    PyAttributeError::new_err(
                        "use delattr(target, name) or delattr(target) with target being a dotted import string"
                    )
                })?
                .extract::<String>(py)?;

            (name_str, target)
        };

        if !actual_target.bind(py).hasattr(&actual_name)? {
            if raising {
                return Err(PyAttributeError::new_err(actual_name));
            }
            return Ok(());
        }

        // Get old value
        let oldval = if let Ok(val) = actual_target.bind(py).getattr(&actual_name) {
            // Handle class descriptors
            if actual_target
                .bind(py)
                .is_instance_of::<pyo3::types::PyType>()
            {
                match actual_target
                    .bind(py)
                    .getattr("__dict__")?
                    .get_item(&actual_name)
                    .ok()
                {
                    Some(v) => v.into(),
                    None => make_missing(py)?,
                }
            } else {
                val.into()
            }
        } else {
            make_missing(py)?
        };

        // Store for undo
        self.lock_setattr()?
            .push((actual_target.clone_ref(py), actual_name.clone(), oldval));

        // Delete attribute
        actual_target.bind(py).delattr(&actual_name)?;

        Ok(())
    }

    /// Set dictionary entry name to value.
    #[expect(clippy::needless_pass_by_value)]
    fn setitem(
        &mut self,
        py: Python<'_>,
        dic: Py<PyAny>,
        name: Py<PyAny>,
        value: Py<PyAny>,
    ) -> PyResult<()> {
        let bound_dic = dic.bind(py);

        // Get old value if it exists
        let oldval = match bound_dic.get_item(&name).ok() {
            Some(v) => v.into(),
            None => make_missing(py)?,
        };

        // Store for undo
        self.lock_setitem()?
            .push((dic.clone_ref(py), name.clone_ref(py), oldval));

        // Set new value
        bound_dic.set_item(&name, value)?;

        Ok(())
    }

    /// Delete name from dict.
    #[expect(clippy::needless_pass_by_value)]
    #[pyo3(signature = (dic, name, raising = true))]
    fn delitem(
        &mut self,
        py: Python<'_>,
        dic: Py<PyAny>,
        name: Py<PyAny>,
        raising: bool,
    ) -> PyResult<()> {
        let bound_dic = dic.bind(py);

        if !bound_dic.contains(&name)? {
            if raising {
                return Err(pyo3::exceptions::PyKeyError::new_err(format!("{name:?}")));
            }
            return Ok(());
        }

        let oldval = bound_dic.get_item(&name)?.into();

        self.lock_setitem()?
            .push((dic.clone_ref(py), name.clone_ref(py), oldval));

        bound_dic.del_item(&name)?;

        Ok(())
    }

    /// Set environment variable name to value.
    #[pyo3(signature = (name, value, prepend = None))]
    #[expect(clippy::needless_pass_by_value)]
    fn setenv(
        &mut self,
        py: Python<'_>,
        name: String,
        value: Py<PyAny>,
        prepend: Option<String>,
    ) -> PyResult<()> {
        let os_module = py.import("os")?;
        let environ = os_module.getattr("environ")?;

        let value_string = value.to_string();

        let final_value = if let Some(prep_char) = prepend {
            if environ.contains(&name)? {
                let current = environ.get_item(&name)?.extract::<String>()?;
                format!("{value_string}{prep_char}{current}")
            } else {
                value_string
            }
        } else {
            value_string
        };

        let name_key = name.into_pyobject(py)?.into_any().unbind();
        let value_obj = final_value.into_pyobject(py)?.into_any().unbind();

        let oldval = match environ.get_item(&name_key).ok() {
            Some(v) => v.into(),
            None => make_missing(py)?,
        };

        self.lock_setitem()?
            .push((environ.clone().unbind(), name_key.clone_ref(py), oldval));

        environ.set_item(&name_key, value_obj)?;

        Ok(())
    }

    /// Delete name from the environment.
    #[pyo3(signature = (name, raising = true))]
    #[expect(clippy::needless_pass_by_value)]
    fn delenv(&mut self, py: Python<'_>, name: String, raising: bool) -> PyResult<()> {
        let os_module = py.import("os")?;
        let environ = os_module.getattr("environ")?;

        let name_key = name.clone().into_pyobject(py)?.into_any().unbind();

        if !environ.contains(&name_key)? {
            if raising {
                return Err(pyo3::exceptions::PyKeyError::new_err(format!("{name:?}")));
            }
            return Ok(());
        }

        let oldval = environ.get_item(&name_key)?.into();

        self.lock_setitem()?
            .push((environ.clone().unbind(), name_key.clone_ref(py), oldval));

        environ.del_item(&name_key)?;

        Ok(())
    }

    /// Prepend path to sys.path list of import locations.
    ///
    /// Accepts `str`, `bytes`, or any `os.PathLike` object (e.g. `pathlib.Path`).
    fn syspath_prepend(&mut self, py: Python<'_>, path: &Bound<'_, PyAny>) -> PyResult<()> {
        let sys_module = py.import("sys")?;
        let sys_path = sys_module.getattr("path")?;

        // Convert PathLike objects (e.g. pathlib.Path) to str via os.fspath.
        let os_module = py.import("os")?;
        let path_str = os_module.call_method1("fspath", (path,))?;

        // Save original sys.path if not already saved
        let mut save = self.lock_savesyspath()?;
        if save.is_none() {
            let saved: Vec<String> = sys_path.extract()?;
            *save = Some(saved);
        }

        sys_path.call_method1("insert", (0, path_str))?;

        let importlib = py.import("importlib")?;
        importlib.call_method0("invalidate_caches")?;

        Ok(())
    }

    /// Change the current working directory to the specified path.
    #[expect(clippy::needless_pass_by_value)]
    fn chdir(&mut self, py: Python<'_>, path: Py<PyAny>) -> PyResult<()> {
        let os_module = py.import("os")?;
        let path_string = path.to_string();

        // Save current directory if not already saved
        let mut cwd = self.lock_cwd()?;
        if cwd.is_none() {
            let current = os_module.call_method0("getcwd")?.extract::<String>()?;
            *cwd = Some(current);
        }

        // Change directory
        os_module.call_method1("chdir", (path_string,))?;

        Ok(())
    }

    /// Undo previous changes.
    fn undo(&mut self, py: Python<'_>) -> PyResult<()> {
        self.undo_setattr(py)?;
        self.undo_setitem(py)?;
        self.undo_syspath(py)?;
        self.undo_cwd(py)?;
        Ok(())
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __exit__(
        &mut self,
        py: Python<'_>,
        _exc_type: Py<PyAny>,
        _exc_val: Py<PyAny>,
        _exc_tb: Py<PyAny>,
    ) -> PyResult<bool> {
        self.undo(py)?;
        Ok(false)
    }
}

/// Context manager wrapper for `MockEnv`
#[pyclass]
struct MockEnvContext {
    mock_env: MockEnv,
}

#[pymethods]
impl MockEnvContext {
    #[expect(clippy::needless_pass_by_value)]
    fn __enter__(slf: PyRef<'_, Self>) -> PyResult<Py<MockEnv>> {
        let py = slf.py();
        Py::new(py, MockEnv::new())
    }

    fn __exit__(
        &mut self,
        py: Python<'_>,
        _exc_type: Py<PyAny>,
        _exc_val: Py<PyAny>,
        _exc_tb: Py<PyAny>,
    ) -> PyResult<bool> {
        self.mock_env.undo(py)?;
        Ok(false)
    }
}

/// Helper function to resolve dotted import paths
fn resolve(py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
    let parts: Vec<&str> = name.split('.').collect();

    let importlib = py.import("importlib")?;
    let mut used = parts[0].to_string();
    let mut found = importlib.call_method1("import_module", (used.clone(),))?;

    for part in &parts[1..] {
        used.push('.');
        used.push_str(part);

        if let Ok(attr) = found.getattr(part) {
            found = attr;
            continue;
        }

        // Try importing as module
        match importlib.call_method1("import_module", (used.clone(),)) {
            Ok(module) => {
                found = module;
            }
            Err(e) => {
                // Check if this is the expected import error
                let err_str = format!("{e}");
                if err_str.contains(&used) {
                    return Err(e);
                }
                return Err(pyo3::exceptions::PyImportError::new_err(format!(
                    "import error in {used}: {e}"
                )));
            }
        }

        found = annotated_getattr(py, &found, part, &used)?;
    }

    Ok(found.into())
}

/// Helper to get attribute with better error messages
fn annotated_getattr<'py>(
    _py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    name: &str,
    ann: &str,
) -> PyResult<Bound<'py, PyAny>> {
    obj.getattr(name).map_err(|_e| {
        let type_name = obj
            .get_type()
            .name()
            .map_or_else(|_| "Unknown".to_string(), |n| n.to_string());
        PyAttributeError::new_err(format!(
            "{type_name:?} object at {ann} has no attribute {name:?}"
        ))
    })
}

/// Derive import path into (`attribute_name`, `target_object`)
fn derive_importpath(
    py: Python<'_>,
    import_path: &str,
    raising: bool,
) -> PyResult<(String, Py<PyAny>)> {
    if !import_path.contains('.') {
        return Err(PyAttributeError::new_err(format!(
            "must be absolute import path string, not {import_path:?}"
        )));
    }

    let parts: Vec<&str> = import_path.rsplitn(2, '.').collect();
    let attr = parts[0].to_string();
    let module = parts[1];

    let target = resolve(py, module)?;

    if raising {
        annotated_getattr(py, target.bind(py), &attr, module)?;
    }

    Ok((attr, target))
}

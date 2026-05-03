use camino::Utf8PathBuf;
use serde::{Serialize, Serializer};

use crate::module_name;

/// Represents a fully qualified function name including its module path.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QualifiedFunctionName {
    function_name: String,
    module_path: ModulePath,
}

impl QualifiedFunctionName {
    /// Create a new qualified function name.
    pub fn new(function_name: String, module_path: ModulePath) -> Self {
        Self {
            function_name,
            module_path,
        }
    }

    /// Return the unqualified function name.
    pub fn function_name(&self) -> &str {
        &self.function_name
    }

    /// Return the module path this function belongs to.
    pub fn module_path(&self) -> &ModulePath {
        &self.module_path
    }
}

impl std::fmt::Display for QualifiedFunctionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}::{}",
            self.module_path.module_name(),
            self.function_name
        )
    }
}

impl Serialize for QualifiedFunctionName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Represents a fully qualified test name, optionally including a parametrized variant.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QualifiedTestName {
    function_name: QualifiedFunctionName,
    full_name: Option<String>,
    case_index: Option<usize>,
}

impl QualifiedTestName {
    /// Create a new qualified test name.
    pub fn new(function_name: QualifiedFunctionName, full_name: Option<String>) -> Self {
        Self {
            function_name,
            full_name,
            case_index: None,
        }
    }

    /// Attach a parametrize case index. Used for stable cache/duration keys
    /// that survive renaming of parameter values across runs.
    #[must_use]
    pub fn with_case_index(mut self, case_index: Option<usize>) -> Self {
        self.case_index = case_index;
        self
    }

    /// Return the underlying qualified function name.
    pub fn function_name(&self) -> &QualifiedFunctionName {
        &self.function_name
    }

    /// Return the parametrize case index if this name refers to a specific case.
    pub fn case_index(&self) -> Option<usize> {
        self.case_index
    }

    /// Return the parameter portion of the test name (e.g., `"(a=1, b=2)"`), if any.
    pub fn params(&self) -> Option<&str> {
        let full_name = self.full_name.as_deref()?;
        let base = self.function_name.to_string();
        full_name.strip_prefix(&base)
    }

    /// Stable string identifier for cache and partitioning, of the form
    /// `module::test_name` (no parametrize) or `module::test_name[idx]`.
    ///
    /// Distinct from `Display`, which renders the human-facing name with
    /// parameter values.
    pub fn cache_key(&self) -> String {
        match self.case_index {
            Some(idx) => format!("{}[{idx}]", self.function_name),
            None => self.function_name.to_string(),
        }
    }
}

impl std::fmt::Display for QualifiedTestName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(full_name) = &self.full_name {
            write!(f, "{full_name}")
        } else {
            write!(f, "{}", self.function_name)
        }
    }
}

/// A Python module path combining the filesystem path with its dotted module name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModulePath {
    path: Utf8PathBuf,
    module_name: String,
}

impl ModulePath {
    /// Create a new module path by computing the dotted module name relative to `cwd`.
    pub fn new<P: Into<Utf8PathBuf>>(path: P, cwd: &Utf8PathBuf) -> Option<Self> {
        let path = path.into();
        let module_name = module_name(cwd, path.as_ref())?;
        Some(Self { path, module_name })
    }

    /// Create a new module path with an explicit dotted module name.
    ///
    /// Use this when the module name cannot be computed from the file path
    /// (e.g. framework modules installed into a venv).
    pub fn new_with_name<P: Into<Utf8PathBuf>>(path: P, module_name: String) -> Self {
        Self {
            path: path.into(),
            module_name,
        }
    }

    /// Return the dotted module name (e.g., `"tests.test_add"`).
    pub fn module_name(&self) -> &str {
        self.module_name.as_str()
    }

    /// Return the filesystem path of this module.
    pub fn path(&self) -> &Utf8PathBuf {
        &self.path
    }
}

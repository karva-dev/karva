use std::rc::Rc;

use karva_python_semantic::QualifiedFunctionName;
use pyo3::prelude::*;
use ruff_python_ast::StmtFunctionDef;

use crate::discovery::DiscoveredModule;
use crate::extensions::tags::Tags;

/// Represents a single test function discovered from Python source code.
///
/// Contains all the information needed to execute a test, including the
/// function's qualified name, AST representation, Python callable, and
/// any associated decorator tags.
#[derive(Debug)]
pub struct DiscoveredTestFunction {
    /// Fully qualified name including module path and function name.
    pub(crate) name: QualifiedFunctionName,

    /// AST representation of the function definition.
    pub(crate) stmt_function_def: Rc<StmtFunctionDef>,

    /// Reference to the actual Python callable object.
    pub(crate) py_function: Py<PyAny>,

    /// Decorator tags like parametrize, skip, xfail, etc.
    pub(crate) tags: Tags,

    /// Restrict execution to these parametrize case indices when `Some`,
    /// or run every case when `None`. Set by the worker CLI when the user
    /// (or partitioner) requested a subset like `file::test[3]`.
    pub(crate) case_filter: Option<Vec<usize>>,
}

impl DiscoveredTestFunction {
    pub(crate) fn new(
        py: Python<'_>,
        module: &DiscoveredModule,
        stmt_function_def: Rc<StmtFunctionDef>,
        py_function: Py<PyAny>,
        case_filter: Option<Vec<usize>>,
    ) -> Self {
        let name = QualifiedFunctionName::new(
            stmt_function_def.name.to_string(),
            module.module_path().clone(),
        );

        let tags = Tags::from_py_any(py, &py_function, Some(&stmt_function_def));

        Self {
            name,
            stmt_function_def,
            py_function,
            tags,
            case_filter,
        }
    }
}

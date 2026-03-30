use std::path::Path;
use std::rc::Rc;

use camino::Utf8Path;
use karva_collector::{CollectedModule, CollectedPackage};
use karva_project::path::{TestPath, TestPathError};
use karva_python_semantic::ModulePath;
use pyo3::prelude::*;
use ruff_python_ast::{PythonVersion, Stmt};
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};

use crate::Context;
use crate::collection::TestFunctionCollector;
use crate::discovery::visitor::{discover, is_generator};
use crate::discovery::{DiscoveredModule, DiscoveredPackage};
use crate::extensions::fixtures::DiscoveredFixture;
use crate::utils::add_to_sys_path;

/// Discovers test functions and fixtures from Python source files.
///
/// Handles the conversion from collected AST information to fully discovered
/// test entities by importing Python modules and resolving function references.
pub struct StandardDiscoverer<'ctx, 'a> {
    /// Reference to the test execution context.
    context: &'ctx Context<'a>,
}

impl<'ctx, 'a> StandardDiscoverer<'ctx, 'a> {
    pub fn new(context: &'ctx Context<'a>) -> Self {
        Self { context }
    }

    pub(crate) fn discover_with_py(
        self,
        py: Python<'_>,
        test_paths: Vec<Result<TestPath, TestPathError>>,
    ) -> DiscoveredPackage {
        let cwd = self.context.cwd();

        if add_to_sys_path(py, cwd, 0).is_err() {
            return DiscoveredPackage::new(cwd.to_path_buf());
        }

        let test_paths = test_paths
            .into_iter()
            .filter_map(|path| match path {
                Ok(path) => match path {
                    TestPath::Directory(_) | TestPath::File(_) => None,
                    TestPath::Function(function) => Some(function),
                },
                Err(_) => None,
            })
            .collect();

        let collector =
            TestFunctionCollector::new(self.context.cwd(), self.context.collection_settings());

        let collected_package = collector.collect_all(test_paths);

        let mut session_package = self.convert_package(py, collected_package);

        session_package.shrink();

        session_package
    }

    /// Convert a collected package to a discovered package by importing Python modules
    /// and resolving test functions and fixtures.
    fn convert_package(
        &self,
        py: Python,
        collected_package: CollectedPackage,
    ) -> DiscoveredPackage {
        let CollectedPackage {
            path,
            modules,
            packages,
            configuration_module,
        } = collected_package;

        let mut discovered_package = DiscoveredPackage::new(path);

        if let Some(collected_module) = configuration_module {
            discovered_package
                .set_configuration_module(Some(self.convert_module(py, collected_module)));
        }

        for collected_module in modules.into_values() {
            discovered_package.add_direct_module(self.convert_module(py, collected_module));
        }

        for collected_subpackage in packages.into_values() {
            discovered_package
                .add_direct_subpackage(self.convert_package(py, collected_subpackage));
        }

        discovered_package
    }

    fn convert_module(&self, py: Python, collected_module: CollectedModule) -> DiscoveredModule {
        let CollectedModule {
            path,
            module_type: _,
            source_text,
            test_function_defs,
            fixture_function_defs,
        } = collected_module;

        let mut module = DiscoveredModule::new_with_source(path, source_text);

        discover(
            self.context,
            py,
            &mut module,
            test_function_defs,
            fixture_function_defs,
        );

        module
    }
}

/// Discovers all fixtures defined in `karva._builtins` by importing the module at
/// runtime and parsing its source file.
///
/// Returns an empty list if the module cannot be imported or parsed, so callers
/// always have a valid (possibly empty) slice to work with.
pub fn discover_framework_fixtures(
    py: Python<'_>,
    python_version: PythonVersion,
) -> Vec<DiscoveredFixture> {
    let Ok(builtins_module) = py.import("karva._builtins") else {
        return vec![];
    };

    let Ok(file_path_obj) = builtins_module.getattr("__file__") else {
        return vec![];
    };
    let Ok(file_path_str) = file_path_obj.extract::<String>() else {
        return vec![];
    };
    let Some(utf8_path) = Utf8Path::from_path(Path::new(&file_path_str)) else {
        return vec![];
    };

    let Ok(source_text) = std::fs::read_to_string(utf8_path) else {
        return vec![];
    };

    let module_path = ModulePath::new_with_name(utf8_path, "karva._builtins".to_string());

    let mut parse_options = ParseOptions::from(Mode::Module);
    parse_options = parse_options.with_target_version(python_version);
    let Some(parsed) = parse_unchecked(&source_text, parse_options).try_into_module() else {
        return vec![];
    };

    let mut fixtures = Vec::new();
    for stmt in parsed.into_syntax().body {
        let Stmt::FunctionDef(function_def) = stmt else {
            continue;
        };
        if function_def.name.starts_with('_') {
            continue;
        }
        let is_gen = is_generator(&function_def);
        let stmt_rc = Rc::new(function_def);
        if let Ok(fixture) = DiscoveredFixture::try_from_function(
            py,
            stmt_rc,
            &builtins_module,
            &module_path,
            is_gen,
        ) {
            fixtures.push(fixture);
        }
    }

    fixtures
}

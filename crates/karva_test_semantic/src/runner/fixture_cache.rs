use std::collections::HashMap;

use pyo3::prelude::*;

use crate::extensions::fixtures::FixtureScope;
use crate::runner::scoped_storage::ScopedStorage;

/// Caches fixture values at different scope levels.
///
/// Fixtures are cached based on their declared scope to avoid redundant
/// setup when the same fixture is used multiple times within a scope.
#[derive(Debug, Default)]
pub struct FixtureCache {
    storage: ScopedStorage<HashMap<String, Py<PyAny>>>,
}

impl FixtureCache {
    /// Get a fixture value from the cache based on its scope
    pub(crate) fn get(&self, py: Python, name: &str, scope: FixtureScope) -> Option<Py<PyAny>> {
        self.storage
            .get(scope)
            .borrow()
            .get(name)
            .map(|v| v.clone_ref(py))
    }

    /// Insert a fixture value into the cache based on its scope
    pub(crate) fn insert(&self, name: String, value: Py<PyAny>, scope: FixtureScope) {
        self.storage.get(scope).borrow_mut().insert(name, value);
    }

    pub(crate) fn clear_fixtures(&self, scope: FixtureScope) {
        self.storage.get(scope).borrow_mut().clear();
    }
}

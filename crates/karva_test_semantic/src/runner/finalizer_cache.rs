use pyo3::prelude::*;

use crate::Context;
use crate::extensions::fixtures::{Finalizer, FixtureScope};
use crate::runner::scoped_storage::ScopedStorage;

/// Manages fixture teardown callbacks at different scope levels.
///
/// Finalizers are collected during fixture setup and executed in LIFO
/// order when their scope ends (e.g., after a test, module, or package).
#[derive(Debug, Default)]
pub struct FinalizerCache {
    storage: ScopedStorage<Vec<Finalizer>>,
}

impl FinalizerCache {
    pub(crate) fn add_finalizer(&self, finalizer: Finalizer) {
        self.storage
            .get(finalizer.scope)
            .borrow_mut()
            .push(finalizer);
    }

    pub(crate) fn run_and_clear_scope(
        &self,
        context: &Context,
        py: Python<'_>,
        scope: FixtureScope,
    ) {
        // Run finalizers in reverse order (LIFO)
        self.storage
            .get(scope)
            .borrow_mut()
            .drain(..)
            .rev()
            .for_each(|finalizer| finalizer.run(context, py));
    }
}

use std::cell::RefCell;

use crate::extensions::fixtures::FixtureScope;

/// A per-scope storage container that maps each `FixtureScope` to its own `RefCell<T>`.
///
/// Used by both `FixtureCache` and `FinalizerCache` to avoid duplicating the same
/// four-field struct and `match`-based accessor.
#[derive(Debug, Default)]
pub(super) struct ScopedStorage<T: Default> {
    session: RefCell<T>,
    package: RefCell<T>,
    module: RefCell<T>,
    function: RefCell<T>,
}

impl<T: Default> ScopedStorage<T> {
    pub(super) fn get(&self, scope: FixtureScope) -> &RefCell<T> {
        match scope {
            FixtureScope::Session => &self.session,
            FixtureScope::Package => &self.package,
            FixtureScope::Module => &self.module,
            FixtureScope::Function => &self.function,
        }
    }
}

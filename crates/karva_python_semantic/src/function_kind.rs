/// The kind of a collected Python function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKind {
    /// A test function (e.g., `test_something`).
    Test,
    /// A fixture function (e.g., decorated with `@fixture`).
    Fixture,
}

impl FunctionKind {
    /// Return the kind as a capitalised string (e.g., `"Test"` or `"Fixture"`).
    pub fn capitalised(self) -> &'static str {
        match self {
            Self::Test => "Test",
            Self::Fixture => "Fixture",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalised_test() {
        assert_eq!(FunctionKind::Test.capitalised(), "Test");
    }

    #[test]
    fn test_capitalised_fixture() {
        assert_eq!(FunctionKind::Fixture.capitalised(), "Fixture");
    }
}

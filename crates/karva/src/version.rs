use std::fmt;

pub struct VersionInfo {
    version: String,
}

impl fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version)?;

        Ok(())
    }
}

pub fn version() -> Option<VersionInfo> {
    option_env!("CARGO_PKG_VERSION").map(|version| VersionInfo {
        version: version.to_string(),
    })
}

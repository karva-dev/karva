use camino::Utf8PathBuf;
use karva_combine::Combine;

use super::{Config, Options, UnknownProfile};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct ProjectOptionsOverrides {
    pub config_file_override: Option<Utf8PathBuf>,
    pub profile: Option<String>,
    pub options: Options,
}

impl ProjectOptionsOverrides {
    pub fn new(config_file_override: Option<Utf8PathBuf>, options: Options) -> Self {
        Self {
            config_file_override,
            profile: None,
            options,
        }
    }

    #[must_use]
    pub fn with_profile(mut self, profile: Option<String>) -> Self {
        self.profile = profile;
        self
    }

    /// Resolve the requested profile from `config` and combine the CLI
    /// overrides on top.
    pub fn apply_to(&self, config: Config) -> Result<Options, UnknownProfile> {
        let resolved = config.resolve_profile(self.profile.as_deref())?;
        Ok(self.options.clone().combine(resolved))
    }
}

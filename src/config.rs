use miette::{IntoDiagnostic, Result, SourceSpan};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct BaseURL {
    /// The URL format for the git server
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Map of prefixes to git server URL formats
    pub prefixes: HashMap<String, BaseURL>,
    /// Default prefix when one is not specified at the command line
    pub default_prefix: String,
}

// Adapted from https://github.com/zkat/miette/blob/main/examples/serde_json.rs, Thank you!
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("malformed config provided")]
pub struct ConfigError {
    cause: toml::de::Error,
    #[source_code]
    input: String,
    #[label("{cause}")]
    location: SourceSpan,
}

impl ConfigError {
    pub fn from_serde_error(input: impl Into<String>, cause: toml::de::Error) -> Self {
        let input = input.into();
        let location = SourceSpan::from(cause.span().unwrap());
        Self {
            cause,
            input,
            location,
        }
    }
}

impl Config {
    pub fn init(path: &PathBuf) -> Result<()> {
        let mut config = Config {
            prefixes: HashMap::new(),
            default_prefix: "gh".to_string(),
        };
        config.prefixes.insert("gh".to_string(), BaseURL { url: "https://github.com/{{ repo }}.git".to_string() });
        let toml = toml::to_string(&config).into_diagnostic()?;
        
        log::debug!("writing default config to file: {:?}", path);

        // ensure parent directories exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).into_diagnostic()?;
        }

        std::fs::write(path, toml).into_diagnostic()
    }

    pub fn from_file(path: PathBuf) -> Result<Self> {
        log::debug!("reading config from file: {:?}", path);
        let input = std::fs::read_to_string(path).into_diagnostic()?;
        toml::from_str(&input)
            .map_err(|e| ConfigError::from_serde_error(input, e))
            .into_diagnostic()
    }

    pub fn clone_url(&self, prefix: &str, repo_path: &str) -> String {
        log::debug!("querying config for `prefix:path` -> {}:{}", prefix, repo_path);
        let base_url = self.prefixes.get(prefix).unwrap_or_else(|| {
            log::warn!("prefix not found, using default: {}", self.default_prefix);
            self.prefixes
                .get(&self.default_prefix)
                .expect("No default prefix found")
        });
        log::debug!("using base URL: {}", base_url.url);
        base_url.url.replace("{{ repo }}", repo_path)
    }
}

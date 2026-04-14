/// Errors from webdown-core operations.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("failed to read config file '{path}': {source}")]
    ConfigRead {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to parse config: {0}")]
    ConfigParse(#[from] serde_yaml::Error),

    #[error("no matching rule for domain '{0}'")]
    NoMatchingRule(String),

    #[error("environment variable '{name}' not set (required by auth for domain '{domain}')")]
    MissingEnvVar { name: String, domain: String },

    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    #[error("HTTP {status} for URL '{url}'")]
    HttpStatus { status: u16, url: String },

    #[error("failed to extract body_path '{path}' from JSON response")]
    JsonPathExtract { path: String },

    #[error("CSS selector '{0}' matched no elements")]
    SelectorNoMatch(String),

    #[error("invalid CSS selector '{selector}': {reason}")]
    InvalidSelector { selector: String, reason: String },

    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
}

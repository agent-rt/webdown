use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::CoreError;

/// Resolve the config file path in priority order:
/// 1. Explicit `--config` argument
/// 2. `$WEBDOWN_CONFIG` environment variable
/// 3. `~/.config/webdown/config.yaml`
///
/// Returns `None` if no config file is found (defaults will be used).
///
/// ## Complexity
/// - **Time**: O(1) — a few path/env lookups.
/// - **Heap allocation**: One `PathBuf` for the resolved path.
/// - **Concurrency**: Stateless.
fn resolve_config_path(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p.to_path_buf());
    }

    if let Ok(env_path) = std::env::var("WEBDOWN_CONFIG") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return Some(p);
        }
    }

    if let Some(home) = dirs_path() {
        let p = home.join("config.yaml");
        if p.exists() {
            return Some(p);
        }
    }

    None
}

/// XDG-style config dir: `~/.config/webdown/`
fn dirs_path() -> Option<PathBuf> {
    // Respect XDG_CONFIG_HOME if set, else ~/.config
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .ok()
        .or_else(|| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".config"))
                .ok()
        })?;
    Some(base.join("webdown"))
}

/// Load config from the resolved path, or return defaults if no config exists.
///
/// ## Complexity
/// - **Time**: O(n) where n = config file size.
/// - **Heap allocation**: One `String` for file contents + deserialized `Config`.
/// - **Concurrency**: Stateless, file read is blocking.
pub fn load_config(explicit_path: Option<&Path>) -> Result<Config, CoreError> {
    match resolve_config_path(explicit_path) {
        Some(path) => {
            let content = std::fs::read_to_string(&path).map_err(|e| CoreError::ConfigRead {
                path: path.display().to_string(),
                source: e,
            })?;
            let config: Config = serde_yaml::from_str(&content)?;
            Ok(config)
        }
        None => Ok(Config::default()),
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            defaults: crate::config::Defaults::default(),
            rules: vec![crate::config::Rule {
                domain: "*".into(),
                auth: None,
                source: crate::config::Source::default(),
                turndown: None,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_default_when_no_config() {
        // Remove env var to ensure fallback
        std::env::remove_var("WEBDOWN_CONFIG");
        let config = load_config(None).unwrap();
        assert!(!config.rules.is_empty());
        assert_eq!(config.rules[0].domain, "*");
    }

    #[test]
    fn load_from_explicit_path() {
        let yaml = r#"
defaults:
  turndown:
    heading_style: setext
rules:
  - domain: "example.com"
    source:
      type: html
      selector: "main"
  - domain: "*"
    source:
      type: html
"#;
        let dir = std::env::temp_dir().join("webdown_test_load");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.yaml");
        std::fs::write(&path, yaml).unwrap();

        let config = load_config(Some(&path)).unwrap();
        assert_eq!(config.defaults.turndown.heading_style, "setext");
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.rules[0].domain, "example.com");
        assert_eq!(
            config.rules[0].source.selector.as_deref(),
            Some("main")
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_invalid_yaml_returns_error() {
        let dir = std::env::temp_dir().join("webdown_test_invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.yaml");
        std::fs::write(&path, "{{{{invalid yaml").unwrap();

        let result = load_config(Some(&path));
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_missing_file_returns_error() {
        let path = PathBuf::from("/nonexistent/webdown/config.yaml");
        let result = load_config(Some(&path));
        assert!(result.is_err());
    }

    #[test]
    fn yaml_roundtrip() {
        let yaml = r#"
defaults:
  turndown:
    heading_style: atx
    code_block_style: fenced
    bullet_list_marker: "-"
rules:
  - domain: "*.atlassian.net"
    auth:
      type: token
      value_env: CONFLUENCE_TOKEN
      header: Authorization
      prefix: Bearer
    source:
      type: api
      url_template: "{scheme}://{host}/wiki/rest/api/content/{id}"
      body_path: "body.storage.value"
  - domain: "*"
    source:
      type: html
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let serialized = serde_yaml::to_string(&config).unwrap();
        let config2: Config = serde_yaml::from_str(&serialized).unwrap();

        assert_eq!(config.rules.len(), config2.rules.len());
        assert_eq!(config.rules[0].domain, config2.rules[0].domain);
        assert_eq!(
            config.defaults.turndown.heading_style,
            config2.defaults.turndown.heading_style,
        );
    }
}

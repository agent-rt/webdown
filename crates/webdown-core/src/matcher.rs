use url::Url;

use crate::config::{Config, TurndownOptions};
use crate::error::CoreError;

/// A fully resolved rule with defaults merged in.
#[derive(Debug, Clone)]
pub struct ResolvedRule {
    pub domain: String,
    pub auth: Option<crate::config::Auth>,
    pub source: crate::config::Source,
    pub turndown: TurndownOptions,
}

/// Match a URL against the config rules and return the first matching rule
/// with defaults merged.
///
/// ## Matching order
/// Rules are evaluated top-to-bottom. First match wins.
///
/// ## Glob patterns
/// - `"example.com"` — exact domain match
/// - `"*.example.com"` — matches any subdomain of example.com
/// - `"*"` — matches everything (catch-all, should be last)
///
/// ## Complexity
/// - **Time**: O(n) where n = number of rules (linear scan).
/// - **Heap allocation**: Zero additional allocations beyond the returned `ResolvedRule`.
/// - **Concurrency**: Stateless, `Send + Sync` friendly.
pub fn match_rule(config: &Config, url: &Url) -> Result<ResolvedRule, CoreError> {
    let host = url
        .host_str()
        .ok_or_else(|| CoreError::InvalidUrl(url::ParseError::EmptyHost))?;

    for rule in &config.rules {
        if domain_matches(&rule.domain, host) {
            let turndown = rule
                .turndown
                .clone()
                .unwrap_or_else(|| config.defaults.turndown.clone());

            return Ok(ResolvedRule {
                domain: rule.domain.clone(),
                auth: rule.auth.clone(),
                source: rule.source.clone(),
                turndown,
            });
        }
    }

    // Fallback: if no explicit "*" rule, return defaults
    Ok(ResolvedRule {
        domain: "*".into(),
        auth: None,
        source: crate::config::Source::default(),
        turndown: config.defaults.turndown.clone(),
    })
}

/// Check if a domain glob pattern matches a given host.
fn domain_matches(pattern: &str, host: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let pattern_lower = pattern.to_ascii_lowercase();
    let host_lower = host.to_ascii_lowercase();

    if let Some(suffix) = pattern_lower.strip_prefix("*.") {
        // "*.example.com" matches "sub.example.com" and "a.b.example.com"
        // but NOT "example.com" itself
        host_lower.ends_with(suffix)
            && host_lower.len() > suffix.len()
            && host_lower.as_bytes()[host_lower.len() - suffix.len() - 1] == b'.'
    } else {
        // Exact match
        host_lower == pattern_lower
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    fn make_config(rules: Vec<Rule>) -> Config {
        Config {
            defaults: Defaults::default(),
            rules,
        }
    }

    fn simple_rule(domain: &str) -> Rule {
        Rule {
            domain: domain.into(),
            auth: None,
            source: Source::default(),
            turndown: None,
        }
    }

    #[test]
    fn exact_match() {
        assert!(domain_matches("example.com", "example.com"));
        assert!(domain_matches("Example.COM", "example.com"));
        assert!(!domain_matches("example.com", "other.com"));
    }

    #[test]
    fn wildcard_subdomain() {
        assert!(domain_matches("*.example.com", "sub.example.com"));
        assert!(domain_matches("*.example.com", "a.b.example.com"));
        assert!(!domain_matches("*.example.com", "example.com"));
        assert!(!domain_matches("*.example.com", "notexample.com"));
    }

    #[test]
    fn global_wildcard() {
        assert!(domain_matches("*", "anything.com"));
        assert!(domain_matches("*", "localhost"));
    }

    #[test]
    fn match_rule_first_wins() {
        let config = make_config(vec![
            simple_rule("specific.com"),
            simple_rule("*.com"),
            simple_rule("*"),
        ]);
        let url = Url::parse("https://specific.com/page").unwrap();
        let resolved = match_rule(&config, &url).unwrap();
        assert_eq!(resolved.domain, "specific.com");
    }

    #[test]
    fn match_rule_fallback_to_wildcard() {
        let config = make_config(vec![simple_rule("other.com"), simple_rule("*")]);
        let url = Url::parse("https://example.com/page").unwrap();
        let resolved = match_rule(&config, &url).unwrap();
        assert_eq!(resolved.domain, "*");
    }

    #[test]
    fn match_rule_no_rules_uses_defaults() {
        let config = make_config(vec![]);
        let url = Url::parse("https://example.com/page").unwrap();
        let resolved = match_rule(&config, &url).unwrap();
        assert_eq!(resolved.domain, "*");
        assert_eq!(resolved.turndown.heading_style, "atx");
    }

    #[test]
    fn match_rule_merges_turndown_defaults() {
        let custom_opts = TurndownOptions {
            heading_style: "setext".into(),
            ..Default::default()
        };
        let config = Config {
            defaults: Defaults {
                turndown: TurndownOptions {
                    heading_style: "atx".into(),
                    code_block_style: "indented".into(),
                    bullet_list_marker: "+".into(),
                },
            },
            rules: vec![Rule {
                domain: "example.com".into(),
                auth: None,
                source: Source::default(),
                turndown: Some(custom_opts),
            }],
        };
        let url = Url::parse("https://example.com/x").unwrap();
        let resolved = match_rule(&config, &url).unwrap();
        // Rule-level turndown overrides defaults entirely
        assert_eq!(resolved.turndown.heading_style, "setext");
    }

    #[test]
    fn subdomain_matching_atlassian() {
        let config = make_config(vec![simple_rule("*.atlassian.net")]);
        let url = Url::parse("https://myteam.atlassian.net/wiki/page").unwrap();
        let resolved = match_rule(&config, &url).unwrap();
        assert_eq!(resolved.domain, "*.atlassian.net");
    }
}

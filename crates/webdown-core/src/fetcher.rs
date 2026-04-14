use reqwest::Client;
use url::Url;

use crate::config::{Auth, AuthType, SourceType};
use crate::error::CoreError;
use crate::matcher::ResolvedRule;

/// Fetch HTML content from a URL according to the resolved rule.
///
/// ## Behavior by source type
/// - **Html**: GET the URL directly, optionally extract content via CSS selector.
/// - **Api**: GET the URL (or rewritten via `url_template`), parse JSON response,
///   extract HTML fragment via `body_path`.
///
/// ## Complexity
/// - **Time**: O(response_size) for parsing/extraction.
/// - **Heap allocation**: Response body + optional DOM parse tree.
/// - **Concurrency**: Async, `Send`-safe. `Client` is reusable across calls.
pub async fn fetch(client: &Client, url: &Url, rule: &ResolvedRule) -> Result<String, CoreError> {
    let request_url = resolve_url(url, rule);
    let mut builder = client.get(request_url.as_str());

    // Inject authentication headers
    if let Some(ref auth) = rule.auth {
        builder = apply_auth(builder, auth, &rule.domain)?;
    }

    let response = builder.send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(CoreError::HttpStatus {
            status: status.as_u16(),
            url: request_url.to_string(),
        });
    }

    let body = response.text().await?;

    match rule.source.source_type {
        SourceType::Api => extract_from_json(&body, rule),
        SourceType::Html => extract_from_html(&body, rule),
    }
}

/// Resolve the actual request URL, applying `url_template` if configured.
fn resolve_url(url: &Url, rule: &ResolvedRule) -> Url {
    if let Some(ref template) = rule.source.url_template {
        let scheme = url.scheme();
        let host = url.host_str().unwrap_or("");
        let path = url.path().trim_start_matches('/');

        // Simple template substitution
        let resolved = template
            .replace("{scheme}", scheme)
            .replace("{host}", host)
            .replace("{path}", path)
            // Extract last path segment for APIs like Confluence
            .replace("{path_segment}", path.rsplit('/').next().unwrap_or(path));

        resolved.parse().unwrap_or_else(|_| url.clone())
    } else {
        url.clone()
    }
}

/// Apply authentication to a request builder.
fn apply_auth(
    builder: reqwest::RequestBuilder,
    auth: &Auth,
    domain: &str,
) -> Result<reqwest::RequestBuilder, CoreError> {
    let value = std::env::var(&auth.value_env).map_err(|_| CoreError::MissingEnvVar {
        name: auth.value_env.clone(),
        domain: domain.to_owned(),
    })?;

    let builder = match auth.auth_type {
        AuthType::Token => {
            let header_name = auth.header.as_deref().unwrap_or("Authorization");
            let prefix = auth.prefix.as_deref().unwrap_or("Bearer");
            let header_value = format!("{prefix} {value}");
            builder.header(header_name, header_value)
        }
        AuthType::Cookie => builder.header("Cookie", value),
        AuthType::Header => {
            let header_name = auth.header.as_deref().unwrap_or("Authorization");
            builder.header(header_name, value)
        }
    };

    Ok(builder)
}

/// Extract an HTML fragment from a JSON response using a dot-separated path.
///
/// Supports paths like `"body.storage.value"` to traverse nested objects.
fn extract_from_json(body: &str, rule: &ResolvedRule) -> Result<String, CoreError> {
    let path = rule.source.body_path.as_deref().unwrap_or("");

    if path.is_empty() {
        return Ok(body.to_owned());
    }

    let json: serde_json::Value =
        serde_json::from_str(body).map_err(|_| CoreError::JsonPathExtract {
            path: path.to_owned(),
        })?;

    let mut current = &json;
    for segment in path.split('.') {
        current = current
            .get(segment)
            .ok_or_else(|| CoreError::JsonPathExtract {
                path: path.to_owned(),
            })?;
    }

    match current {
        serde_json::Value::String(s) => Ok(s.clone()),
        other => Ok(other.to_string()),
    }
}

/// Extract content from HTML, optionally filtered by a CSS selector.
fn extract_from_html(body: &str, rule: &ResolvedRule) -> Result<String, CoreError> {
    match rule.source.selector.as_deref() {
        Some(selector_str) => {
            let document = scraper::Html::parse_document(body);
            let selector =
                scraper::Selector::parse(selector_str).map_err(|e| CoreError::InvalidSelector {
                    selector: selector_str.to_owned(),
                    reason: format!("{e:?}"),
                })?;

            let fragments: Vec<String> = document
                .select(&selector)
                .map(|el| el.inner_html())
                .collect();

            if fragments.is_empty() {
                // Fallback: return full body rather than failing
                Ok(body.to_owned())
            } else {
                Ok(fragments.join("\n"))
            }
        }
        None => Ok(body.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    fn default_rule() -> ResolvedRule {
        ResolvedRule {
            domain: "*".into(),
            auth: None,
            source: Source::default(),
            turndown: TurndownOptions::default(),
        }
    }

    #[test]
    fn extract_json_nested_path() {
        let json = r#"{"body":{"storage":{"value":"<h1>Hello</h1>"}}}"#;
        let mut rule = default_rule();
        rule.source.source_type = SourceType::Api;
        rule.source.body_path = Some("body.storage.value".into());

        let result = extract_from_json(json, &rule).unwrap();
        assert_eq!(result, "<h1>Hello</h1>");
    }

    #[test]
    fn extract_json_missing_path() {
        let json = r#"{"body":{}}"#;
        let mut rule = default_rule();
        rule.source.source_type = SourceType::Api;
        rule.source.body_path = Some("body.storage.value".into());

        let result = extract_from_json(json, &rule);
        assert!(result.is_err());
    }

    #[test]
    fn extract_json_empty_path_returns_body() {
        let json = r#"{"data":"test"}"#;
        let mut rule = default_rule();
        rule.source.source_type = SourceType::Api;
        rule.source.body_path = None;

        let result = extract_from_json(json, &rule).unwrap();
        assert_eq!(result, json);
    }

    #[test]
    fn extract_html_with_selector() {
        let html = r#"
            <html><body>
                <nav>menu</nav>
                <article class="content"><h1>Title</h1><p>Text</p></article>
                <footer>foot</footer>
            </body></html>
        "#;
        let mut rule = default_rule();
        rule.source.selector = Some("article.content".into());

        let result = extract_from_html(html, &rule).unwrap();
        assert!(result.contains("<h1>Title</h1>"));
        assert!(result.contains("<p>Text</p>"));
        assert!(!result.contains("menu"));
        assert!(!result.contains("foot"));
    }

    #[test]
    fn extract_html_no_selector_returns_full() {
        let html = "<html><body><p>Hello</p></body></html>";
        let rule = default_rule();

        let result = extract_from_html(html, &rule).unwrap();
        assert_eq!(result, html);
    }

    #[test]
    fn extract_html_selector_no_match_fallback() {
        let html = "<html><body><p>Hello</p></body></html>";
        let mut rule = default_rule();
        rule.source.selector = Some("div.nonexistent".into());

        let result = extract_from_html(html, &rule).unwrap();
        assert_eq!(result, html);
    }

    #[test]
    fn resolve_url_no_template() {
        let url = Url::parse("https://example.com/page").unwrap();
        let rule = default_rule();
        assert_eq!(
            resolve_url(&url, &rule).as_str(),
            "https://example.com/page"
        );
    }

    #[test]
    fn resolve_url_with_template() {
        let url = Url::parse("https://myteam.atlassian.net/wiki/content/12345").unwrap();
        let mut rule = default_rule();
        rule.source.url_template = Some(
            "{scheme}://{host}/wiki/rest/api/content/{path_segment}?expand=body.storage".into(),
        );

        let resolved = resolve_url(&url, &rule);
        assert_eq!(
            resolved.as_str(),
            "https://myteam.atlassian.net/wiki/rest/api/content/12345?expand=body.storage"
        );
    }

    #[test]
    fn apply_auth_token() {
        std::env::set_var("TEST_TOKEN_123", "my-secret");
        let auth = Auth {
            auth_type: AuthType::Token,
            value_env: "TEST_TOKEN_123".into(),
            header: None,
            prefix: Some("Bearer".into()),
        };
        let client = Client::new();
        let builder = client.get("https://example.com");
        let result = apply_auth(builder, &auth, "example.com");
        assert!(result.is_ok());
        std::env::remove_var("TEST_TOKEN_123");
    }

    #[test]
    fn apply_auth_missing_env() {
        std::env::remove_var("NONEXISTENT_VAR_WEBDOWN");
        let auth = Auth {
            auth_type: AuthType::Token,
            value_env: "NONEXISTENT_VAR_WEBDOWN".into(),
            header: None,
            prefix: None,
        };
        let client = Client::new();
        let builder = client.get("https://example.com");
        let result = apply_auth(builder, &auth, "test.com");
        assert!(matches!(result, Err(CoreError::MissingEnvVar { .. })));
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Defaults {
    #[serde(default)]
    pub turndown: TurndownOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub domain: String,
    #[serde(default)]
    pub auth: Option<Auth>,
    #[serde(default)]
    pub source: Source,
    #[serde(default)]
    pub turndown: Option<TurndownOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    Token,
    Cookie,
    Header,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    #[serde(rename = "type")]
    pub auth_type: AuthType,
    pub value_env: String,
    #[serde(default)]
    pub header: Option<String>,
    #[serde(default)]
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Html,
    Api,
}

impl Default for SourceType {
    fn default() -> Self {
        Self::Html
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    #[serde(rename = "type", default)]
    pub source_type: SourceType,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub url_template: Option<String>,
    #[serde(default)]
    pub body_path: Option<String>,
}

impl Default for Source {
    fn default() -> Self {
        Self {
            source_type: SourceType::Html,
            selector: None,
            url_template: None,
            body_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurndownOptions {
    #[serde(default = "default_heading_style")]
    pub heading_style: String,
    #[serde(default = "default_code_block_style")]
    pub code_block_style: String,
    #[serde(default = "default_bullet_list_marker")]
    pub bullet_list_marker: String,
}

impl Default for TurndownOptions {
    fn default() -> Self {
        Self {
            heading_style: default_heading_style(),
            code_block_style: default_code_block_style(),
            bullet_list_marker: default_bullet_list_marker(),
        }
    }
}

fn default_heading_style() -> String {
    "atx".to_owned()
}

fn default_code_block_style() -> String {
    "fenced".to_owned()
}

fn default_bullet_list_marker() -> String {
    "-".to_owned()
}

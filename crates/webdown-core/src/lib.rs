pub mod config;
pub mod error;
pub mod fetcher;
pub mod loader;
pub mod matcher;

pub use config::{Auth, AuthType, Config, Rule, Source, SourceType, TurndownOptions};
pub use error::CoreError;
pub use fetcher::fetch;
pub use loader::load_config;
pub use matcher::{match_rule, ResolvedRule};

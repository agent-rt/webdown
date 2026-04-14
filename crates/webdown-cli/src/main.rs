use std::path::PathBuf;

use clap::Parser;
use url::Url;

#[derive(Parser)]
#[command(name = "webdown", version, about = "Convert web pages to Markdown")]
struct Args {
    /// Target URL (web page or API endpoint)
    url: String,

    /// Config file path (default: ~/.config/webdown/config.yaml)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Override CSS selector (HTML mode only)
    #[arg(short, long)]
    selector: Option<String>,

    /// Output to file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Print debug info to stderr
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("{0}")]
    Core(#[from] webdown_core::CoreError),

    #[error("{0}")]
    Engine(#[from] webdown_engine::EngineError),

    #[error("invalid URL '{url}': {source}")]
    InvalidUrl {
        url: String,
        source: url::ParseError,
    },

    #[error("failed to write output: {0}")]
    WriteOutput(#[from] std::io::Error),
}

fn main() {
    let args = Args::parse();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");

    if let Err(e) = runtime.block_on(run(args)) {
        eprintln!("error: {e}");
        // Print cause chain
        let mut source = std::error::Error::source(&e);
        while let Some(cause) = source {
            eprintln!("  caused by: {cause}");
            source = std::error::Error::source(cause);
        }
        std::process::exit(1);
    }
}

async fn run(args: Args) -> Result<(), AppError> {
    // Parse URL
    let url = Url::parse(&args.url).map_err(|e| AppError::InvalidUrl {
        url: args.url.clone(),
        source: e,
    })?;

    // Load config
    let config = webdown_core::load_config(args.config.as_deref())?;
    if args.verbose {
        eprintln!("[webdown] config loaded: {} rules", config.rules.len());
    }

    // Match rule
    let mut rule = webdown_core::match_rule(&config, &url)?;
    if args.verbose {
        eprintln!("[webdown] matched rule: domain={}", rule.domain);
    }

    // CLI --selector overrides rule selector
    if let Some(ref selector) = args.selector {
        rule.source.selector = Some(selector.clone());
    }

    // Fetch content
    let client = reqwest::Client::builder()
        .user_agent(format!("webdown/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(webdown_core::CoreError::from)?;

    let html = webdown_core::fetch(&client, &url, &rule).await?;
    if args.verbose {
        eprintln!("[webdown] fetched {} bytes", html.len());
    }

    // Convert to Markdown (run in blocking thread to avoid tokio/wasmtime runtime conflict)
    let engine_opts = webdown_engine::TurndownOptions {
        heading_style: rule.turndown.heading_style.clone(),
        code_block_style: rule.turndown.code_block_style.clone(),
        bullet_list_marker: rule.turndown.bullet_list_marker.clone(),
    };
    let markdown = tokio::task::spawn_blocking(move || {
        let engine = webdown_engine::TurndownEngine::new()?;
        engine.convert(&html, &engine_opts)
    })
    .await
    .expect("blocking task panicked")?;
    if args.verbose {
        eprintln!("[webdown] converted to {} bytes markdown", markdown.len());
    }

    // Output
    match args.output {
        Some(path) => std::fs::write(&path, &markdown)?,
        None => print!("{markdown}"),
    }

    Ok(())
}

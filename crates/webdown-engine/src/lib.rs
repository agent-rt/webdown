mod error;

pub use error::EngineError;

use serde::{Deserialize, Serialize};
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
use wasmtime_wasi::WasiCtxBuilder;

/// Pre-compiled turndown.wasm embedded in the binary.
const TURNDOWN_WASM: &[u8] = include_bytes!("../../../engine/turndown-wasm/dist/turndown.wasm");

/// Maximum stdout capture size (4 MiB).
const MAX_OUTPUT_SIZE: usize = 4 * 1024 * 1024;

/// Options passed to the Turndown.js engine.
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
    "atx".into()
}

fn default_code_block_style() -> String {
    "fenced".into()
}

fn default_bullet_list_marker() -> String {
    "-".into()
}

/// JSON payload sent to the Wasm module via stdin.
#[derive(Serialize)]
struct WasmInput<'a> {
    html: &'a str,
    options: &'a TurndownOptions,
}

/// JSON payload received from the Wasm module via stdout.
#[derive(Deserialize)]
struct WasmOutput {
    markdown: String,
}

/// HTML-to-Markdown conversion engine backed by Turndown.js running in Wasmtime.
///
/// The `Engine` and `Module` are initialized once and reused across calls.
/// Each `convert()` invocation creates a fresh `Store` + WASI context,
/// ensuring isolation between calls with zero cross-contamination.
pub struct TurndownEngine {
    engine: Engine,
    module: Module,
}

impl TurndownEngine {
    /// Create a new engine by compiling the embedded turndown.wasm.
    ///
    /// # Performance
    /// - **Time**: One-time Wasm compilation cost (~50-100ms).
    /// - **Memory**: `Module` holds the compiled code in memory.
    /// - **Concurrency**: `Engine` and `Module` are `Send + Sync`.
    pub fn new() -> Result<Self, EngineError> {
        let engine = Engine::default();
        let module = Module::new(&engine, TURNDOWN_WASM)
            .map_err(EngineError::WasmCompile)?;
        Ok(Self { engine, module })
    }

    /// Convert HTML to Markdown using the Turndown.js Wasm module.
    ///
    /// # Performance
    /// - **Time complexity**: O(n) where n is the HTML input size.
    /// - **Heap allocation**: Creates a new `Store` + WASI context per call (~KB).
    ///   stdin/stdout use in-memory pipes (no OS file descriptors).
    /// - **Concurrency**: Safe to call from multiple threads (`&self` is immutable).
    ///   Each call gets its own isolated Wasm instance.
    pub fn convert(&self, html: &str, options: &TurndownOptions) -> Result<String, EngineError> {
        let input = WasmInput { html, options };
        let input_json = serde_json::to_string(&input)
            .map_err(EngineError::SerializeInput)?;

        // Set up piped stdin/stdout
        let stdin_pipe = MemoryInputPipe::new(input_json);
        let stdout_pipe = MemoryOutputPipe::new(MAX_OUTPUT_SIZE);
        let stdout_reader = stdout_pipe.clone();

        // Build WASI context with piped IO
        let wasi_ctx = WasiCtxBuilder::new()
            .stdin(stdin_pipe)
            .stdout(stdout_pipe)
            .build_p1();

        // Create linker with WASI p1 bindings
        let mut linker: Linker<WasiP1Ctx> = Linker::new(&self.engine);
        p1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(EngineError::WasmLink)?;

        // Instantiate and run _start
        let mut store = Store::new(&self.engine, wasi_ctx);
        let instance = linker.instantiate(&mut store, &self.module)
            .map_err(EngineError::WasmInstantiate)?;

        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(EngineError::WasmEntrypoint)?;

        start.call(&mut store, ())
            .map_err(EngineError::WasmExec)?;

        // Read stdout output
        let output_bytes = stdout_reader.contents();
        let output_str = std::str::from_utf8(&output_bytes)
            .map_err(EngineError::InvalidUtf8)?;

        let output: WasmOutput = serde_json::from_str(output_str)
            .map_err(EngineError::DeserializeOutput)?;

        Ok(output.markdown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> TurndownEngine {
        TurndownEngine::new().expect("failed to create engine")
    }

    #[test]
    fn heading() {
        let md = engine().convert("<h1>Hello</h1>", &TurndownOptions::default()).unwrap();
        assert_eq!(md.trim(), "# Hello");
    }

    #[test]
    fn bold_and_italic() {
        let md = engine()
            .convert("<p><strong>bold</strong> and <em>italic</em></p>", &TurndownOptions::default())
            .unwrap();
        assert!(md.contains("**bold**"));
        assert!(md.contains("_italic_") || md.contains("*italic*"));
    }

    #[test]
    fn link() {
        let md = engine()
            .convert(r#"<a href="https://example.com">Click</a>"#, &TurndownOptions::default())
            .unwrap();
        assert!(md.contains("[Click](https://example.com)"));
    }

    #[test]
    fn code_block() {
        let md = engine()
            .convert("<pre><code>let x = 1;</code></pre>", &TurndownOptions::default())
            .unwrap();
        assert!(md.contains("```"));
        assert!(md.contains("let x = 1;"));
    }

    #[test]
    fn unordered_list() {
        let md = engine()
            .convert("<ul><li>a</li><li>b</li></ul>", &TurndownOptions::default())
            .unwrap();
        assert!(md.contains("- ") || md.contains("-   "));
    }

    #[test]
    fn custom_bullet_marker() {
        let opts = TurndownOptions {
            bullet_list_marker: "*".into(),
            ..Default::default()
        };
        let md = engine()
            .convert("<ul><li>item</li></ul>", &opts)
            .unwrap();
        assert!(md.contains("*") && !md.contains("- "));
    }

    #[test]
    fn empty_html() {
        let md = engine().convert("", &TurndownOptions::default()).unwrap();
        assert!(md.trim().is_empty());
    }

    #[test]
    fn malformed_html_no_panic() {
        let result = engine().convert("<div><p>unclosed", &TurndownOptions::default());
        assert!(result.is_ok());
    }
}

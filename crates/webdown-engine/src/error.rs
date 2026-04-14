/// Errors that can occur during Wasm engine operations.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("failed to compile turndown.wasm: {0}")]
    WasmCompile(#[source] wasmtime::Error),

    #[error("failed to link WASI: {0}")]
    WasmLink(#[source] wasmtime::Error),

    #[error("failed to instantiate wasm module: {0}")]
    WasmInstantiate(#[source] wasmtime::Error),

    #[error("failed to find _start entrypoint: {0}")]
    WasmEntrypoint(#[source] wasmtime::Error),

    #[error("wasm execution failed: {0}")]
    WasmExec(#[source] wasmtime::Error),

    #[error("failed to serialize input: {0}")]
    SerializeInput(#[source] serde_json::Error),

    #[error("failed to deserialize output: {0}")]
    DeserializeOutput(#[source] serde_json::Error),

    #[error("wasm output is not valid UTF-8: {0}")]
    InvalidUtf8(#[source] std::str::Utf8Error),
}

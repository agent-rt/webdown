[中文](README.zh-CN.md)

# webdown

Convert web pages to Markdown — a Rust CLI powered by [Turndown.js](https://github.com/mixmark-io/turndown) via WebAssembly.

## Features

- **Fast & portable** — single binary, no Node.js runtime required
- **Smart content extraction** — CSS selectors for HTML pages, JSON path for APIs
- **Configurable rules** — per-domain auth, source type, and Turndown options
- **Multiple auth methods** — Bearer token, cookie, custom header (values from env vars)
- **WASM-sandboxed** — Turndown.js runs in an isolated Wasmtime instance

## Installation

### npm (recommended)

```bash
npx @agent-rt/webdown <URL>

# or install globally
npm install -g @agent-rt/webdown
```

### Cargo

```bash
cargo install webdown-cli
```

## Usage

```bash
# Convert a web page to Markdown
webdown https://example.com

# Extract specific content with CSS selector
webdown -s "article.markdown-body" https://github.com/user/repo

# Save to file
webdown -o output.md https://example.com

# Use a custom config
webdown -c ./my-config.yaml https://example.com
```

### Options

```
Usage: webdown [OPTIONS] <URL>

Arguments:
  <URL>  Target URL (web page or API endpoint)

Options:
  -c, --config <PATH>        Config file path (default: ~/.config/webdown/config.yaml)
  -s, --selector <SELECTOR>  Override CSS selector (HTML mode only)
  -o, --output <PATH>        Output to file instead of stdout
  -v, --verbose              Print debug info to stderr
  -h, --help                 Print help
  -V, --version              Print version
```

## Configuration

Place a config file at `~/.config/webdown/config.yaml` or set `$WEBDOWN_CONFIG`.

```yaml
defaults:
  turndown:
    heading_style: atx        # atx | setext
    code_block_style: fenced  # fenced | indented
    bullet_list_marker: "-"   # - | + | *

rules:
  # Confluence — API mode with token auth
  - domain: "*.atlassian.net"
    auth:
      type: token
      value_env: CONFLUENCE_TOKEN
      header: Authorization
      prefix: Bearer
    source:
      type: api
      url_template: "{scheme}://{host}/wiki/rest/api/content/{path_segment}?expand=body.storage"
      body_path: "body.storage.value"

  # GitHub — extract article content only
  - domain: "github.com"
    source:
      type: html
      selector: "article.markdown-body"

  # Default — fetch full HTML
  - domain: "*"
    source:
      type: html
```

### Rule matching

Rules are matched by `domain` in order. Supported patterns:

| Pattern | Matches |
|---------|---------|
| `example.com` | Exact domain |
| `*.example.com` | Any subdomain of example.com |
| `*` | Catch-all (fallback) |

### Auth types

| Type | Description |
|------|-------------|
| `token` | Sends `{prefix} {value}` in the specified header (default: `Authorization: Bearer <token>`) |
| `cookie` | Sends value as `Cookie` header |
| `header` | Sends raw value in the specified header |

Auth values are read from environment variables specified by `value_env`.

### Source types

| Type | Description |
|------|-------------|
| `html` | Fetches HTML directly. Use `selector` to extract specific elements. |
| `api` | Fetches JSON from a rewritten URL (`url_template`), extracts HTML via `body_path`. |

URL templates support `{scheme}`, `{host}`, `{path}`, and `{path_segment}` placeholders.

## Architecture

```
webdown-cli          CLI interface (clap)
  └── webdown-core   Config, rule matching, HTTP fetching
  └── webdown-engine WASM runtime (Wasmtime + Turndown.js)
        └── turndown.wasm  Turndown.js compiled via Javy
```

The HTML-to-Markdown conversion runs Turndown.js inside a WebAssembly sandbox — no external JavaScript runtime needed. The WASM module is embedded in the binary at compile time.

## License

MIT

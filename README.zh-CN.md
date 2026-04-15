[English](README.md)

# webdown

将网页转换为 Markdown — 基于 Rust 开发，通过 WebAssembly 运行 [Turndown.js](https://github.com/mixmark-io/turndown)。

## 特性

- **快速且便携** — 单一二进制文件，无需 Node.js 运行时
- **智能内容提取** — 支持 CSS 选择器提取 HTML，JSON 路径提取 API 响应
- **可配置规则** — 按域名配置认证、数据源和 Turndown 选项
- **多种认证方式** — Bearer Token、Cookie、自定义 Header（从环境变量读取）
- **WASM 沙箱** — Turndown.js 运行在隔离的 Wasmtime 实例中

## 安装

### npm（推荐）

```bash
npx @agent-rt/webdown <URL>

# 或全局安装
npm install -g @agent-rt/webdown
```

### Cargo

```bash
cargo install webdown-cli
```

## 使用

```bash
# 将网页转换为 Markdown
webdown https://example.com

# 使用 CSS 选择器提取特定内容
webdown -s "article.markdown-body" https://github.com/user/repo

# 保存到文件
webdown -o output.md https://example.com

# 使用自定义配置文件
webdown -c ./my-config.yaml https://example.com
```

### 选项

```
用法: webdown [选项] <URL>

参数:
  <URL>  目标 URL（网页或 API 端点）

选项:
  -c, --config <PATH>        配置文件路径（默认: ~/.config/webdown/config.yaml）
  -s, --selector <SELECTOR>  覆盖 CSS 选择器（仅 HTML 模式）
  -o, --output <PATH>        输出到文件而非标准输出
  -v, --verbose              输出调试信息到标准错误
  -h, --help                 打印帮助信息
  -V, --version              打印版本号
```

## 配置

将配置文件放在 `~/.config/webdown/config.yaml`，或设置环境变量 `$WEBDOWN_CONFIG`。

```yaml
defaults:
  turndown:
    heading_style: atx        # atx | setext
    code_block_style: fenced  # fenced | indented
    bullet_list_marker: "-"   # - | + | *

rules:
  # Confluence — API 模式 + Token 认证
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

  # GitHub — 仅提取文章内容
  - domain: "github.com"
    source:
      type: html
      selector: "article.markdown-body"

  # 默认 — 获取完整 HTML
  - domain: "*"
    source:
      type: html
```

### 规则匹配

规则按 `domain` 字段顺序匹配，支持以下模式：

| 模式 | 匹配范围 |
|------|---------|
| `example.com` | 精确匹配域名 |
| `*.example.com` | 匹配任意子域名 |
| `*` | 通配（兜底规则） |

### 认证类型

| 类型 | 说明 |
|------|------|
| `token` | 在指定 Header 中发送 `{prefix} {value}`（默认: `Authorization: Bearer <token>`） |
| `cookie` | 将值作为 `Cookie` Header 发送 |
| `header` | 在指定 Header 中发送原始值 |

认证值通过 `value_env` 指定的环境变量读取。

### 数据源类型

| 类型 | 说明 |
|------|------|
| `html` | 直接获取 HTML，可用 `selector` 提取特定元素 |
| `api` | 通过 `url_template` 重写 URL 获取 JSON，用 `body_path` 提取 HTML 内容 |

URL 模板支持 `{scheme}`、`{host}`、`{path}` 和 `{path_segment}` 占位符。

## 架构

```
webdown-cli          命令行界面（clap）
  └── webdown-core   配置、规则匹配、HTTP 请求
  └── webdown-engine WASM 运行时（Wasmtime + Turndown.js）
        └── turndown.wasm  Turndown.js 通过 Javy 编译
```

HTML 到 Markdown 的转换在 WebAssembly 沙箱中运行 Turndown.js — 无需外部 JavaScript 运行时。WASM 模块在编译时嵌入到二进制文件中。

## 许可证

MIT

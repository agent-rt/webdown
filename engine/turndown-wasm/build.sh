#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "==> Installing dependencies..."
bun install

echo "==> Bundling TS with Bun..."
mkdir -p dist
bun build src/index.ts --outfile=dist/bundle.js --target=node --minify

echo "==> Compiling to Wasm with Javy..."
javy build dist/bundle.js -o dist/turndown.wasm

echo "==> Done: dist/turndown.wasm ($(du -h dist/turndown.wasm | cut -f1))"

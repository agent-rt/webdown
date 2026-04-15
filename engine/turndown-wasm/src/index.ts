import "./polyfill";
import TurndownService from "turndown";
// @ts-ignore — no type definitions available
import { gfm } from "turndown-plugin-gfm";

declare const Javy: {
  IO: {
    readSync(fd: number, buf: Uint8Array): number;
    writeSync(fd: number, buf: Uint8Array): void;
  };
};

interface Input {
  html: string;
  options?: {
    heading_style?: string;
    code_block_style?: string;
    bullet_list_marker?: string;
  };
}

interface Output {
  markdown: string;
}

function readStdin(): string {
  const chunks: Uint8Array[] = [];
  const buf = new Uint8Array(1024);
  let n: number;
  while (true) {
    n = Javy.IO.readSync(0, buf);
    if (n === 0) break;
    chunks.push(buf.slice(0, n));
  }
  const total = chunks.reduce((s, c) => s + c.length, 0);
  const result = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.length;
  }
  return new TextDecoder().decode(result);
}

function writeStdout(str: string): void {
  const encoded = new TextEncoder().encode(str);
  Javy.IO.writeSync(1, encoded);
}

const input: Input = JSON.parse(readStdin());
const html = input.html || "";
const options = input.options || {};

const turndown = new TurndownService({
  headingStyle: (options.heading_style as "atx" | "setext") || "atx",
  codeBlockStyle: (options.code_block_style as "fenced" | "indented") || "fenced",
  bulletListMarker: (options.bullet_list_marker as "-" | "+" | "*") || "-",
});

turndown.use(gfm);

const markdown: string = turndown.turndown(html);

const output: Output = { markdown };
writeStdout(JSON.stringify(output));

// Minimal polyfills for Javy's QuickJS environment
// domino requires these Node.js globals but never exercises them
// during synchronous HTML parsing

declare const Javy: {
  IO: {
    readSync(fd: number, buf: Uint8Array): number;
    writeSync(fd: number, buf: Uint8Array): void;
  };
};

if (typeof globalThis.setTimeout === "undefined") {
  (globalThis as any).setTimeout = (fn: () => void, _ms?: number) => {
    fn();
    return 0;
  };
}
if (typeof globalThis.clearTimeout === "undefined") {
  (globalThis as any).clearTimeout = () => {};
}
if (typeof globalThis.setInterval === "undefined") {
  (globalThis as any).setInterval = () => 0;
}
if (typeof globalThis.clearInterval === "undefined") {
  (globalThis as any).clearInterval = () => {};
}
if (typeof globalThis.Buffer === "undefined") {
  (globalThis as any).Buffer = {
    isBuffer: () => false,
  };
}
if (typeof globalThis.process === "undefined") {
  (globalThis as any).process = { env: {} };
}


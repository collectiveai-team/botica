// Minimal botica extension SDK: NDJSON JSON-RPC 2.0 over stdio.
//
// An extension registers rules with `rule(id, languages, check, { severity })`; `check` receives
// `(path, text)` and returns `{ start, end, message }` for each finding, with `start` and `end` as
// byte offsets. `serve()` answers `initialize`, `lint/files` and `shutdown`.

import { createInterface } from "node:readline";

export const PROTOCOL_VERSION = 1;

const LANGUAGES = { py: "python", ts: "typescript", tsx: "typescript", js: "javascript" };
const RULES = new Map();

export function rule(id, languages, check, { severity = "warning" } = {}) {
  RULES.set(id, { languages, severity, check });
  return check;
}

/** A string index (UTF-16 code units) to a byte offset of the UTF-8 text; the protocol uses bytes. */
export function byteOffsets(text) {
  if (/^[\x00-\x7f]*$/.test(text)) {
    return (index) => index;
  }
  return (index) => Buffer.byteLength(text.slice(0, index));
}

function languageOf(path) {
  return LANGUAGES[path.slice(path.lastIndexOf(".") + 1)];
}

function lint(files) {
  const diagnostics = [];
  for (const file of files) {
    const language = languageOf(file.path);
    for (const [id, { languages, severity, check }] of RULES) {
      if (!languages.includes(language)) {
        continue;
      }
      for (const { start, end, message } of check(file.path, file.text)) {
        diagnostics.push({ path: file.path, rule: id, severity, start, end, message });
      }
    }
  }
  return diagnostics;
}

function answer(request) {
  switch (request.method) {
    case "initialize":
      return {
        result: {
          protocolVersion: PROTOCOL_VERSION,
          rules: [...RULES].map(([id, { languages }]) => ({ id, languages })),
        },
      };
    case "lint/files":
      return { result: { diagnostics: lint(request.params.files) } };
    case "shutdown":
      return { result: null };
    default:
      return { error: { code: -32601, message: request.method } };
  }
}

export async function serve() {
  for await (const line of createInterface({ input: process.stdin })) {
    const request = JSON.parse(line);
    console.log(JSON.stringify({ jsonrpc: "2.0", id: request.id, ...answer(request) }));
    if (request.method === "shutdown") {
      break;
    }
  }
  process.stdin.destroy();
}

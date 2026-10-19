// Minimal botica extension SDK: NDJSON JSON-RPC 2.0 over stdio.
//
// An extension registers rules with `rule(id, languages, check, { severity, scope })`. A rule of
// file scope gets `(path, text, options)` and returns `{ start, end, message }` for each finding,
// with `start` and `end` as byte offsets. A rule with `scope: "project"` gets
// `(root, paths, options)`, reads the files itself and returns `{ path, start, end, message }`.
// `options` holds the options that the configuration of the project gives the rule. `serve()`
// answers `initialize`, `lint/files`, `lint/project` and `shutdown`, and runs the rules that each
// request names.

import { createInterface } from "node:readline";

export const PROTOCOL_VERSION = 1;

const LANGUAGES = { py: "python", ts: "typescript", tsx: "typescript", js: "javascript" };
const RULES = new Map();

export function rule(id, languages, check, { severity = "warning", scope = "file" } = {}) {
  RULES.set(id, { languages, severity, scope, check });
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

/** The rules of `scope` that the request names (every one when it names none), with their options. */
function named(params, scope) {
  const options = params.options ?? {};
  return [...RULES]
    .filter(([id, registered]) => registered.scope === scope && (!params.rules || params.rules.includes(id)))
    .map(([id, registered]) => [id, registered, options[id] ?? {}]);
}

function lint(params) {
  const diagnostics = [];
  const rules = named(params, "file");
  for (const file of params.files) {
    const language = languageOf(file.path);
    for (const [id, { languages, severity, check }, options] of rules) {
      if (!languages.includes(language)) {
        continue;
      }
      for (const { start, end, message } of check(file.path, file.text, options)) {
        diagnostics.push({ path: file.path, rule: id, severity, start, end, message });
      }
    }
  }
  return diagnostics;
}

function lintProject(params) {
  const diagnostics = [];
  for (const [id, { severity, check }, options] of named(params, "project")) {
    for (const { path, start, end, message } of check(params.root, params.files, options)) {
      diagnostics.push({ path, rule: id, severity, start, end, message });
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
      return { result: { diagnostics: lint(request.params) } };
    case "lint/project":
      return { result: { diagnostics: lintProject(request.params) } };
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

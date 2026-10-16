# @botica/sdk

The SDK to write a rule of botica as an extension in Node (ADR 0003). It answers the protocol of
the extension host on stdin and stdout: `initialize`, `lint/files` and `shutdown`, one JSON-RPC 2.0
message per line.

It is not published to npm yet. A project that wants it copies `src/` or points a `file:`
dependency at this folder.

```js
// .botica/rules/acme/structure/no-blobs/no-blobs.mjs
import { byteOffsets, rule, serve } from "@botica/sdk";

rule("acme/no-blobs", ["python"], (path, text) => {
  const toByte = byteOffsets(text);
  return [{ start: toByte(0), end: toByte(1), message: "acme/no-blobs: a finding is a start, an end and a message" }];
}, { severity: "error" });

serve();
```

- `rule(id, languages, check, { severity })` registers a rule. `check(path, text)` returns the
  findings as `{ start, end, message }`. `severity` is `"warning"` unless given.
- `start` and `end` are byte offsets in the UTF-8 text. `byteOffsets(text)` converts a string index
  into one.
- The SDK knows the languages of the extensions `.py` (`python`), `.ts` and `.tsx` (`typescript`)
  and `.js` (`javascript`).
- `serve()` returns after `shutdown`, and the process ends when stdin is closed.

`@ast-grep/napi` is an optional peer dependency, for a rule that parses its text with ast-grep.

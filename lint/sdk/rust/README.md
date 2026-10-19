# botica-ext-sdk

The SDK to write a rule of botica as an extension in Rust (ADR 0003). It answers the protocol of
the extension host on stdin and stdout: `initialize`, `lint/files`, `lint/project` and `shutdown`,
one JSON-RPC 2.0 message per line. It is a member of the workspace of `lint/` and is not published
to crates.io.

The binary of the extension runs as a rule of the runtime `command`, since the runtime `wasm` is
not built:

```toml
[[implementations]]
kind = "extension"
runtime = "command"
languages = ["python"]
command = ["target/release/no-blobs"]
```

```rust
use botica_ext_sdk::{serve, Finding, Rule};

fn main() -> std::io::Result<()> {
    let rules = [Rule::new("acme/no-blobs", &["python"], |_path, text, _options| {
        vec![Finding::new(0, 1, "acme/no-blobs: a finding is a start, an end and a message")]
    })
    .severity("error")];
    serve(&rules)
}
```

- `Rule::new(id, languages, check)` registers a rule of file scope; `check(path, text, options)`
  returns its `Finding`s, made with `Finding::new(start, end, message)`. `severity` is `warning`
  unless `.severity(…)` says otherwise.
- `options` is a `serde_json::Value` with the options that the configuration of the project gives
  the rule, and an empty object when it gives none.
- `Rule::project(id, languages, check)` registers a rule of project scope, which answers
  `lint/project`: `check(root, paths, options)` reads the files itself and returns findings made
  with `Finding::in_file(path, start, end, message)`. The SDK skips a finding without a path.
- Each request names the rules it asks for, and the SDK runs those alone.
- `start` and `end` are byte offsets in the UTF-8 text, which is what a `&str` indexes.
- The SDK knows the languages of the extensions `.py` (`python`), `.ts` and `.tsx` (`typescript`)
  and `.js` (`javascript`).
- `serve` returns after `shutdown`; `serve_on(rules, input, output)` serves any reader and writer.

`examples/no_leading_x.rs` is a complete extension.

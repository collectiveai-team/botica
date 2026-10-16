# botica-ext-sdk

The SDK to write a rule of botica as an extension in Rust (ADR 0003). It answers the protocol of
the extension host on stdin and stdout: `initialize`, `lint/files` and `shutdown`, one JSON-RPC 2.0
message per line. It is a member of the workspace of `lint/` and is not published to crates.io.

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
    let rules = [Rule::new("acme/no-blobs", &["python"], |_path, text| {
        vec![Finding::new(0, 1, "acme/no-blobs: a finding is a start, an end and a message")]
    })
    .severity("error")];
    serve(&rules)
}
```

- `Rule::new(id, languages, check)` registers a rule; `check(path, text)` returns its `Finding`s.
  `severity` is `warning` unless `.severity(…)` says otherwise.
- `start` and `end` are byte offsets in the UTF-8 text, which is what a `&str` indexes.
- The SDK knows the languages of the extensions `.py` (`python`), `.ts` and `.tsx` (`typescript`)
  and `.js` (`javascript`).
- `serve` returns after `shutdown`; `serve_on(rules, input, output)` serves any reader and writer.

`examples/no_leading_x.rs` is a complete extension.

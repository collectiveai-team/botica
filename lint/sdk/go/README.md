# botica SDK for Go

The SDK to write a rule of botica as an extension in Go (ADR 0003). It answers the protocol of the
extension host on stdin and stdout: `initialize`, `lint/files` and `shutdown`, one JSON-RPC 2.0
message per line. The module is `github.com/collectiveai-team/botica/lint/sdk/go`; no tag
`lint/sdk/go/vX.Y.Z` has been pushed yet.

The binary of the extension runs as a rule of the runtime `command`, since the runtime `wasm` is
not built:

```toml
[[implementations]]
kind = "extension"
runtime = "command"
languages = ["python"]
command = ["bin/no-blobs"]
```

```go
rules := []botica.Rule{{
	ID:        "acme/no-blobs",
	Languages: []string{"python"},
	Severity:  "error",
	Check: func(path, text string) []botica.Finding {
		return []botica.Finding{{Start: 0, End: 1, Message: "acme/no-blobs: a finding is a start, an end and a message"}}
	},
}}
if err := botica.Serve(rules); err != nil {
	log.Fatal(err)
}
```

- `Check(path, text)` returns the findings of a rule. `Severity` is `warning` when it is empty.
- `Start` and `End` are byte offsets in the UTF-8 text, which is what Go indexes in a string.
- The SDK knows the languages of the extensions `.py` (`python`), `.ts` and `.tsx` (`typescript`)
  and `.js` (`javascript`).
- `Serve` returns after `shutdown`; `ServeOn(rules, in, out)` serves any reader and writer.

`examples/no_leading_x` is a complete extension. The tests run with `go test ./...`.

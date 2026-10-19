# botica.sdk

The SDK to write a rule of botica as an extension in Python (ADR 0003). It answers the protocol of
the extension host on stdin and stdout: `initialize`, `lint/files`, `lint/project` and `shutdown`,
one JSON-RPC 2.0 message per line.

It ships in the `botica` wheel, as the module `botica.sdk`; this folder is the `python-source` of
`pyproject.toml`. A script declares `botica[sdk]`, which adds `ast-grep-py`, and runs with
Python 3.9 or later.

```python
# /// script
# dependencies = ["botica[sdk]"]
# ///
from botica.sdk import char_to_byte_offsets, rule, serve


@rule("acme/no-blobs", ["python"], severity="error")
def no_blobs(path, text, options):
    to_byte = char_to_byte_offsets(text)
    yield (to_byte(0), to_byte(1), "acme/no-blobs: a finding is a start, an end and a message")


serve()
```

- `@rule(id, languages, severity="warning", scope="file")` registers a rule. The check receives
  `(path, text, options)` and yields `(start, end, message)`. `options` holds the options that the
  configuration of the project gives the rule.
- A rule with `scope="project"` answers `lint/project`: the check receives `(root, paths, options)`,
  reads the files itself and yields `(path, start, end, message)`.
- Each request names the rules it asks for, and the SDK runs those alone.
- `start` and `end` are byte offsets in the UTF-8 text. `char_to_byte_offsets(text)` converts a
  string index into one.
- The SDK knows the languages of the extensions `.py` (`python`), `.ts` and `.tsx` (`typescript`)
  and `.js` (`javascript`).
- `serve()` returns after `shutdown` or when stdin is closed.

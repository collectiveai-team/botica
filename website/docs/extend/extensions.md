---
title: Extensions
---

# Extensions

A rule of kind `extension` is a program that `botica lint` starts, one process for each active
rule, in the root of the project. It is sent only the files where the rule applies and answers
with findings over JSON-RPC
([ADR 0003](https://github.com/collectiveai-team/botica/blob/main/docs/adr/0003-botica-lint-architecture.md)).
An extension belongs to a [catalog of a project](catalogs.md): the built-in catalog takes none.

```bash
botica rules new structure/no-blobs --kind extension --runtime python --language python \
  --catalog .botica/rules/acme   # the folder, with a script that uses botica.sdk
botica ext sync                # create the environments of the Python scripts of the extension rules
```

## The rule

Each request names the rule and carries the options that
`botica.toml` gives it. Each request names the rule and carries the options that
`botica.toml` gives it. A rule of `scope = "project"` gets the root of the project and the paths of
its files in one `lint/project`, and reads the files itself. When it starts, the program declares
the rule it runs; a program that does not declare it, or declares a rule that no `rule.toml` runs
with the same command, fails. Its implementation says what runs:

```toml
[[implementations]]
kind = "extension"
runtime = "python"          # python | node | command
languages = ["python"]
file = "no-blobs.py"        # python: `uv run --python <version> --script`; node: `node` (.js or .mjs)
# command = ["sh", ".botica/rules/acme/structure/no-blobs/check.sh"]   # runtime = "command", no file
```

## Python

A script of the runtime `python` declares its dependencies (PEP 723) and writes its rule with the
module `botica.sdk` of the wheel `botica`; the extra `botica[sdk]` brings `ast-grep-py`. The script
pins the version of `botica` that the project runs (`<version>` below is the output of
`botica --version`):

```python
# /// script
# requires-python = ">=3.10"
# dependencies = ["botica[sdk]==<version>"]
# ///
from botica.sdk import rule, serve


@rule("acme/no-blobs", ["python"], severity="error")
def no_blobs(path, text, options):
    yield (0, 1, "acme/no-blobs: a finding is a start and an end in bytes, and a message")


if __name__ == "__main__":
    serve()
```

`options` holds the options of the rule after the configuration. A rule registered with
`scope="project"` gets `(root, paths, options)` and yields `(path, start, end, message)`. `start`
and `end` are byte offsets in the UTF-8 text; `char_to_byte_offsets(text)` turns an index of a
Python string, which is what `ast-grep-py` reports, into one.

`uv` runs a script of the runtime `python` with `--python <version>`. With `project` the version
comes from `.python-version`, else from `.venv/pyvenv.cfg`, else from `requires-python` of
`pyproject.toml`, else it is 3.13. `system` runs it with a Python of the system
(`--python-preference only-system`). A free-threaded Python (`3.13t`, `3.14+freethreaded`) is
refused: the run fails and names where the version is written. `botica ext sync` runs
`uv sync --script` with that Python on the script of each active Python rule, so that the first
`botica lint`, in a hook or in CI, does not create the environments.

## Node

A script of the runtime `node` can write its rule with
[`@botica/sdk`](https://github.com/collectiveai-team/botica/blob/main/sdk/node/README.md), which is
not published to npm yet: a project copies its `src/` or points a `file:` dependency at the folder.
`botica rules new --runtime node` writes a script that speaks the protocol with no dependency.

```js
// .botica/rules/acme/structure/no-blobs/no-blobs.mjs
import { byteOffsets, rule, serve } from "@botica/sdk";

rule("acme/no-blobs", ["python"], (path, text, options) => {
  const toByte = byteOffsets(text);
  return [{ start: toByte(0), end: toByte(1), message: "acme/no-blobs: a start, an end and a message" }];
}, { severity: "error" });

serve();
```

## Go and Rust

A program in Go uses the module `github.com/collectiveai-team/botica/sdk/go`
([README](https://github.com/collectiveai-team/botica/blob/main/sdk/go/README.md)), and a program
in Rust the crate `botica-ext-sdk`
([README](https://github.com/collectiveai-team/botica/blob/main/sdk/rust/README.md)). Neither is
published yet. The binary runs as a rule of the runtime `command`:

```toml
[[implementations]]
kind = "extension"
runtime = "command"
languages = ["python"]
command = ["target/release/no-blobs"]
```

=== "Go"

    ```go
    rules := []botica.Rule{{
        ID:        "acme/no-blobs",
        Languages: []string{"python"},
        Severity:  "error",
        Check: func(path, text string, options map[string]any) []botica.Finding {
            return []botica.Finding{{Start: 0, End: 1, Message: "acme/no-blobs: a start, an end and a message"}}
        },
    }}
    if err := botica.Serve(rules); err != nil {
        log.Fatal(err)
    }
    ```

=== "Rust"

    ```rust
    use botica_ext_sdk::{serve, Finding, Rule};

    fn main() -> std::io::Result<()> {
        let rules = [Rule::new("acme/no-blobs", &["python"], |_path, _text, _options| {
            vec![Finding::new(0, 1, "acme/no-blobs: a start, an end and a message")]
        })
        .severity("error")];
        serve(&rules)
    }
    ```

## The protocol

A program in another language speaks the protocol itself: one JSON-RPC 2.0 message per line on
stdin and stdout, and nothing else on stdout. botica sends four requests:

| Method | Params | Result |
|---|---|---|
| `initialize` | `{"protocolVersion": 1, "client": "botica"}` | `{"protocolVersion": 1, "rules": [{"id": "acme/no-blobs", "languages": ["python"]}]}` |
| `lint/files` | `{"files": [{"path": "a.py", "text": "…"}], "rules": ["acme/no-blobs"], "options": {"acme/no-blobs": {}}}` | `{"diagnostics": [{"path", "rule", "severity", "start", "end", "message"}]}` |
| `lint/project` | `{"root": "/repo", "files": ["a.py"], "rules": […], "options": {…}}` | the same as `lint/files` |
| `shutdown` | none | `null`, then the program exits |

`start` and `end` are byte offsets in the text of the file. A rule of `scope = "file"` gets
`lint/files`, and one of `scope = "project"` gets `lint/project` and reads the files itself. A
method the program does not know is answered with the error `-32601`.

## Limits

A finding with another `rule` than the id of its rule is left out, with a warning on stderr. A
process that fails, that does not answer a request in time, or that is still running 5 seconds
after `shutdown`, is killed and stops the run with exit code 2; the findings of the other rules
stay. The process has 120 seconds to answer `initialize`, because a script run with `uv` may
download its dependencies first, and 60 seconds to answer `lint/files`, `lint/project` and
`shutdown`. `timeout` of `[extensions]` in `botica.toml`, a number of seconds, sets both, and the
variable `BOTICA_EXTENSION_TIMEOUT` wins over it. With `on-timeout = "warn"` a process that does not
answer in time is a warning on stderr and the run goes on without the findings of its rule.

```toml
[extensions]
python = "project"    # project (default) | system | a version for uv, as "3.12"
timeout = 30          # seconds for each request; BOTICA_EXTENSION_TIMEOUT wins
on-timeout = "fail"   # fail (default, exit 2) | warn
```

## In the editor

`botica server` runs the extension rules only when the editor asks for them: the flag
`--extensions`, or the option `{"extensions": true}` in the `initializationOptions` of
`initialize`. `botica.toml` has no key for it. It is heavy, because each rule is a process that
stays alive for the whole session, and it is a risk, because opening a repository in the editor
then runs the scripts of that repository (Python, Node, commands). A rule of `scope = "file"` has
its process started when the first document in one of its languages opens. It is sent the text of
the buffer, not the file on disk: at once when the document opens, and after the text has stayed
as it is for 250 ms when it changes. A rule whose `run-on` has `save` and not `type` is sent the
text when the document opens and at each save, and not at the changes; one whose `run-on` has
neither does not run in the editor. What it finds is published after the findings of the engine,
in a second `publishDiagnostics` with both, and leaves the buffer when its text changes, until the
rule answers again (the findings of a rule that runs at save stay until the next save). A rule of
`scope = "project"` does not run on a buffer. A process that fails, or
does not answer within its deadline, is
started again up to 3 times, with a `window/showMessage` warning each time; after that the rule
does not run until the server restarts. At `exit`, or at the end of the input, each process gets
`shutdown` and is killed after 5 seconds if it is still running.

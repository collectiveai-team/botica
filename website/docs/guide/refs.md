---
title: References
---

# References

```bash
botica refs list               # the topics (built-in and of references/), their references and who cites them
botica refs show log-no-print  # the references of a topic, or of the topics that a rule or a standard cites
botica refs check [--all]      # fails when a reference no longer resolves or a citation names no topic
botica refs export             # every topic as JSON, with the text of each file reference (the site reads it)
botica refs pin <permalink> --topic <topic> --license MIT --note "…"
                               # adds the file of a permalink of GitHub to a topic of references/
botica refs update <topic> [--write]
                               # follows the anchor of each external reference to the head of its repository
```

The [references](../references/index.md) of this site are the built-in topics.

## A topic

A topic points at code and documentation to read. It is a folder with a `topic.toml`, a
`README.md` that says what to read in each reference and what to adapt, and optionally a `code/`
folder with code to copy. The topics that the built-in rules cite are in `catalog/references/` and
are compiled into the binary, their code included; a project adds its own in `references/` at its
root, which botica reads from disk.

```toml
# catalog/references/structured-logging/topic.toml
title = "Structured logging with structlog"

[[references]]
file = "code/core/logger.py"          # a file of the topic, from its folder
lines = "1-20"                        # optional: a range or one line; without it, the whole file
anchor = "def get_logger"             # optional: text that the lines hold
note = "the function that every module imports"

[[references]]
repo = "github.com/acme/app"
rev = "3f2a9c1d5e7b4a6c8d0e1f2a3b4c5d6e7f8a9b0c"   # the full SHA of a commit
path = "src/app/core/logger.py"
license = "MIT"
note = "the same module in a project"

[[references]]
url = "https://www.structlog.org/en/stable/getting-started.html#building-a-context"
note = "binding context: `bind()` returns a new logger that carries the context"
quote = "Each logger with its context is immutable."   # optional: the passage, as the page words it
```

A reference points at a fragment. It has a `file`, a `path` or a `url`. A `file` is a file under
`code/` of the topic, written for the topic, so it shows the whole file unless it names `lines`; its
`lines` and `anchor` are checked when the topic loads, so a built-in topic whose code moved fails
the tests of botica. CI lints, type-checks and tests that code, and checks its Compose files with
`docker compose config` (`scripts/check_references.sh`). A
`url` is a page of documentation: it ends with the `#fragment` of a section, or has a `quote` of the
passage, and `note` says why. A reference to a whole page, or to a whole file through `path`, loads,
and `botica refs check` warns about it. A `path` is a file: of the project when the reference has no
`repo`, `rev` and `license`, and of another repository at a commit when it has the three. `rev` is a
full commit SHA, and `repo` is `<host>/<path>` with no scheme.

## Cite a topic

A rule cites topics with `references = ["structured-logging"]` in its `rule.toml`, and a standard
with `references: ["structured-logging"]` in its front matter. The site lists the topics at the end
of the page of each rule and standard, and has a page per topic with the lines of each file
reference, so a reader does not leave the page to understand it. The build of the site runs
`botica refs export`, which fetches the external files.

## Show

`botica refs show` reads the file of an external reference with the `git` of the user: private
repositories open with the credentials that git has, and a missing credential is an error. It runs
`git fetch --depth 1 --filter=blob:none` for the commit, then `git ls-tree` and `git show`, and each
command is killed after 60 seconds (`BOTICA_REFS_TIMEOUT` sets the seconds). The file is kept in a
cache as `<cache>/<repo>/<rev>/<path>` and read from there on the next run, with no git and no
network. The cache is `--cache-dir`, else `botica/refs` under `XDG_CACHE_HOME`, `LOCALAPPDATA`
(Windows) or `~/.cache`. For a file of GitHub, `show` prints the license and the permalink of the
lines.

## Check

`botica refs check` exits with 1 when a file of a reference cannot be read, when its `lines` are past
the end of the file, when its `anchor` is not in them, when a rule or a standard cites a topic that
does not exist, or when a `topic.toml` does not load. It reads the files of the topics of the
project only; `check --all` checks the built-in topics too, whose code is in the binary. The job
`test` of `ci.yml` runs `check --all`. A reference to a page is not fetched.

## Pin and update

`botica refs pin <permalink> --topic <topic> --license <spdx> --note <text>` adds a reference to
`references/<topic>/topic.toml`. The permalink is `https://github.com/<owner>/<repo>/blob/<commit>/<path>`
with `#L10-L40` or `#L10` for the lines. Pin reads the file at the commit, checks the lines, writes
the first line that has text, without its indentation, as the `anchor`, and appends the block to the
file. Without lines the reference points at the whole file and pin prints the advice that `check`
gives. `--title` creates the topic when it does not exist, with a `README.md` that holds the title.
Pin fails and writes nothing when the address is not a permalink of GitHub, the commit is not a full
SHA, the lines are past the end of the file or the topic is a built-in one.

`botica refs update <topic>` asks each repository for its `HEAD` (`git ls-remote`) and, for each
reference to a file of another repository, prints the commit and the lines that it proposes:
`references[0] <label>: rev -> <commit>, lines 2-3 -> 4-5`. It looks for the `anchor` in the file at
the head and keeps the block as long as it was, with the anchor on the same line of the block. A
reference with no `lines` takes the new commit only. The line ends with `, the text changed` when
the lines at the head do not show the text they showed. A reference is a problem, and the command
exits with 1, when it has `lines` and no `anchor`, when the anchor is not in the file at the head
or is in more than one place, or when the block would start before the file or end past it. Without
`--write` nothing is written. With `--write` the `rev` and the `lines` of each reference that
resolves are replaced in `references/<topic>/topic.toml` and the rest of the file stays as it was;
a file that does not write `[[references]]` as tables is not rewritten, and the command fails. A
built-in topic is proposed but not written: its `rev` is edited in `catalog/references/`. References
to a file of the project and to a page are left out.

## Replace a built-in topic

A topic of `references/` with the id of a built-in topic does not load, unless the configuration
lists the id:

```toml
[references]
replace = ["structured-logging"]
```

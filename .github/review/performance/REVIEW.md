---
role: performance
applies:
  paths: []
  tiers: [medium, high]
stages: [local, ci]
blocking: P1
owners: ["@jedzill4"]
max-findings: 6
---

## Invariants

The hot paths of botica, with the sizes that matter:

- `botica lint` on a whole project: the walk of `crates/botica-core/src/walk/`, the read of each
  file, the derived prefilter of `crates/botica-astgrep/` that skips the files a rule cannot match,
  the parse and the rules of `crates/botica-core/src/engine.rs` for each file, on several threads
  (`build_parallel`, `par_iter`). Sizes: 10,000 files, as `bench/run.sh` generates them; one file
  of 2,000 lines, as `crates/botica-core/benches/lint_source.rs` lints it; every rule of the
  catalog selected.
  The work for one file does not depend on the count of the other files, and a rule of project
  scope runs once per run, not once per file.
- The hooks of `prek.toml` and of the mirror: `botica lint --scope file` on the files of a commit
  (about 200 files), the cold start of the binary on one file, and `--commit-msg` on each commit.
- `botica server`: each change of a document lints that buffer, and the extension rules get the
  text after the debounce of 250 ms (`DEBOUNCE` of `crates/botica-lsp/src/lib.rs`). A step that
  reads the whole project on each change of a buffer is a cost that the editor shows.
- The extension rules: one process for each implementation of a rule for the whole run, which gets
  every file of its language in its requests (ADR 0003), not one process for each file. The
  deadlines are 120 s to start and 60 s for each request (`crates/botica-ext/src/lib.rs`).
- `botica lint --diff` and `botica history report`: git is run a fixed number of times for the
  run, not once for each file or each commit, and the history keeps its limits
  (`crates/botica-cli/src/history_cmd/`: 50 MB for each file, 2 GB in total, 200,000 commits).
- The result cache (`--cache`) keys each file by its path and its bytes, and is dropped whole when
  the version, the configuration or the rules change (`website/docs/guide/lint.md`).

## Check always

- A read of the disk, a run of git, a process or a parse added inside the loop over the files of a
  run, or inside the loop over the rules for each file.
- Work for each file that now depends on the count of files or of rules: a search of a list where
  a map or a set was, a sort or a clone of the whole input for each item.
- A step that ran on several threads and now runs on one, or a lock taken for each file.
- A rule of project scope that runs once for each file, and an extension that starts once for each
  file.
- A change to the prefilter, the regions or the skip of subtrees (`--no-prefilter`,
  `--no-regions` and `--no-subtree-skip` turn them off for a measure) that lets more files reach
  the parse.
- A limit, a deadline or a size of the history, of the extensions or of the reader of an artifact
  that is removed or raised.
- A new dependency of `botica-cli` or `botica-core` that the binary loads at start, or a new step
  at start that every command pays.

## Do not report

- A cost on a path that runs once per command and does not grow with the files, the rules or the
  commits.
- A micro-optimisation with no cost at the sizes above.
- The code of the topics under `catalog/references/` and of the templates under
  `catalog/recipes/files/`: botica does not run it.
- What `botica lint` already reports under the rules of the family `async`.

<!-- botica:covered -->
- The hook `botica` of `prek.toml`, run by the job `lint, type-check & test` of `tests.yml`:
  `asyncio-no-get-event-loop` on Python.
<!-- /botica:covered -->

## Evidence

Each finding gives the count of calls on the base and on the head and what it grows with, the path
from the hot path to the code with file and line, and the benchmark that a person would run to
measure it, on the base and on the head: `cargo bench -p botica-core --bench lint_source` for one
file, or the measure of `bench/run.sh` that covers the path for a project.

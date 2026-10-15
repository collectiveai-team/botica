## Directive

A Jupyter notebook (`*.ipynb`) inside the **code of a project** is an error. Keep notebooks beside
the code, in a `notebooks/` directory of their own.

## Why

A notebook is exploration, not importable code, and it ships with its saved outputs and cell
metadata, which bloat an install and its diffs. In the 129 repositories studied, none keeps a
notebook inside its package.

## What the code is

The `code-dirs` of the project: the `code-dirs` setting, or else what is detected from the
manifests. Python: the directories with an `__init__.py` inside the project or its `src/`.
JavaScript and TypeScript, and Rust: `src/`. Go: `internal/`, `pkg/` and `cmd/`. Java, Kotlin and
Scala: `src/main/java`, `kotlin` and `scala`. `botica explain no-notebooks` lists them.

A notebook under `tests`, `test`, `__tests__`, `fixtures`, `testdata`, `examples`, `docs`,
`benches`, `snapshots` or a `*_tests` / `*-tests` directory is left alone, the same directories
`no-artifacts-in-package` leaves alone: a demo notebook in `examples/` is not code that ships.

## Suppression

The finding is about a whole file and has no line, so it cannot be silenced with a line comment.
Exclude the path with `ignores` or `extend-ignores` in the rule's configuration
(`**/examples/*.ipynb`), or exclude it in the project configuration.

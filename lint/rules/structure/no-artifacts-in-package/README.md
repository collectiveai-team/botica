# no-artifacts-in-package · `BTSTRUC003`

**Standard:** CES-32 (documentation only) · **Family:** structure · **Ecosystems:** all

## Directive

Data, trained models and reports live **beside** the code of a project, not in it. Inside the code,
a file of data or of a model (`csv`, `parquet`, `pkl`, `pt`, `h5`, `npy`, `onnx`, `db`, `gz`…) is
an error. In a **Python** package, so is any file that is not code under a `data/`, `resources/`
or `reports/` directory.

## Why

Datasets, generated reports and static assets are not importable code. Bundling them into the
package bloats the wheel, leaks large or binary files into installs, and blurs the line between
"the library" and "the workspace around it". It is the layout of `cookiecutter-data-science` and
Kedro, which Collective's repositories follow.

The name of a directory is not enough: `data/` and `reports/` are often code (a `data/` package
with an `__init__.py`). That is why the rule looks at the **kind of file**, and for the three
names only at files that are not code.

## What the code is

The `code-dirs` of the project (see `no-notebooks`). Java and Kotlin resources live in
`src/main/resources`, outside `src/main/java`, so they are never flagged. Files under `tests`,
`fixtures`, `testdata`, `examples`, `docs`, `benches`, `snapshots` and `*_tests` are left alone.

## Suppression

The finding is about a whole file and has no line, so it cannot be silenced with a line comment.
Exclude the path with `ignores` or `extend-ignores` in the rule's configuration
(`**/resources/**` allows package data), or exclude it in the project configuration.

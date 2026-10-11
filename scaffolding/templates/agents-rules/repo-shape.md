# CES-32 · keep non-code out of the package

**Code:** `CES-32` &nbsp;·&nbsp; **Slug:** `repo-shape` &nbsp;·&nbsp; **Enforced by:** `botica lint`
(`no-notebooks`, `no-artifacts-in-package`), run by the `botica` prek hook &nbsp;·&nbsp; **Tracker:**
[#32](https://github.com/collectiveai-team/scaffolding/issues/32)

## Directive

The code of a project holds **code**, not artifacts. `botica lint` fails on a Jupyter notebook
(`*.ipynb`) found *inside* the code, and on a file of data or of a model (`csv`, `parquet`, `pkl`,
`pt`, `h5`, `npy`, `onnx`, `db`, `gz`…). In a Python package it also fails on any file that is not
code under a `data/`, `resources/` or `reports/` directory. Keep those beside the code, not under it.

## Why

- Notebooks, datasets, and generated reports are not importable code. Bundling them into the
  package bloats the wheel, leaks large/binary files into installs, and blurs the line between
  "the library" and "the workspace around it".
- A clean package boundary means `pip install` ships only what's importable, and the repo's
  exploratory/output material lives where humans (not the import system) expect it.

## What counts as the code

The `code-dirs` of the project: the `code-dirs` setting, or else what `botica` detects from the
manifests. There is nothing to fill in at install time. Python: the directories with an
`__init__.py` inside the project or its `src/`. JavaScript, TypeScript and Rust: `src/`. Go:
`internal/`, `pkg/` and `cmd/`. Java, Kotlin and Scala: `src/main/java`, `kotlin` and `scala`.
`botica explain no-notebooks` lists them.

The name of a directory is not enough, because a `data/` package with an `__init__.py` is code:
`no-artifacts-in-package` looks at the kind of file. It leaves alone the files under `tests`,
`fixtures`, `testdata`, `examples`, `docs`, `benches`, `snapshots` and `*_tests`.

## Suggested layout

```text
repo/
  myapp/            # import package — code only
  data/             # datasets       — outside the package
  reports/          # generated output
  resources/        # static assets
  notebooks/        # *.ipynb exploration
```

## Suppression

The findings are about a whole file and have no line, so a line comment cannot silence them. To
allow a specific path, exclude it with `ignores` or `extend-ignores` in the configuration of the
rule (`no-notebooks` or `no-artifacts-in-package`), or in the project configuration, rather than
removing the `botica` hook.

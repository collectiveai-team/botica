# repo-shape · `BTSTRUC003`

**Standard:** CES-32 (documentation only) · **Family:** structure · **Languages:** all

## Directive

A package holds **code**, not artifacts. A `resources/`, `reports/` or `data/` directory at any
depth **inside a package** is an error, one finding per file under it. Keep those directories at
the root of the repository, next to the package, not under it.

## Why

- Datasets, generated reports and static assets are not importable code. Bundling them into the
  package bloats the wheel, leaks large or binary files into installs, and blurs the line between
  "the library" and "the workspace around it".
- A clean package boundary means an install ships only what is importable, and the exploratory and
  output material lives where people, not the import system, expect it.

## What a package is

The rule does not assume a language or a `src/` directory. Packages sit at the root of the
project, as the standard layout puts them (`backend/` and `frontend/` side by side, or one
package next to `tests/`, `notebooks/`, `data/`).

1. **The configuration decides.** `packages = ["backend", "frontend"]` in the project
   configuration lists the packages, and nothing is detected.
2. **Otherwise it is detected.** A directory of the root is a package when it holds a marker of
   any language: `pyproject.toml`, `package.json`, `Cargo.toml`, `go.mod` or `__init__.py`. A
   directory named `tests`, `test` or `src` is never a package.
3. **With no package the rule does nothing**, and `botica explain repo-shape` says so.

The root itself is not a package: a `data/` next to the packages is where it belongs.

## Suppression

The finding is about a whole file and has no line, so it cannot be silenced with a line comment.
Put `botica: ignore-file[repo-shape]` in a comment of the file, exclude the path with `ignores`
or `extend-ignores` in the rule's configuration (`**/resources/**` allows package data), or
exclude it in the project configuration.

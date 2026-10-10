# no-notebooks · `BTSTRUC004`

**Standard:** CES-32 (documentation only) · **Family:** structure · **Languages:** all
**Off by default:** switch it on with `extend-select = ["no-notebooks"]`.

## Directive

A Jupyter notebook (`*.ipynb`) at any depth **inside a package** is an error. Keep notebooks at the
root of the repository, in a directory of their own next to the package.

## Why

- A notebook is exploration, not importable code, and it ships with its saved outputs and its
  cell metadata, which bloat the package and its diffs.
- Keeping notebooks outside the package keeps an install to what is importable.

## Why it is off by default

This is the part of CES-32 that projects most often have good reasons to break: documentation
examples and tutorials that ship with the package, or `nbdev`-style projects where the notebook
is the source. It is a rule of its own, apart from `repo-shape`, so a project can keep one and
leave the other. Turn it on where notebooks inside the package are a mistake.

## What a package is

The same as for `repo-shape`: the `packages` list of the project configuration, or else the
directories of the root that hold a manifest (`pyproject.toml`, `package.json`, `Cargo.toml`,
`go.mod`) or an `__init__.py`, except `tests`, `test` and `src`. With no package the rule does
nothing, and `botica explain no-notebooks` says so.

## Suppression

The finding is about a whole file and has no line, so it cannot be silenced with a line comment.
Exclude the path with `ignores` or `extend-ignores` in the rule's configuration
(`**/examples/*.ipynb` allows the examples), or exclude it in the project configuration.

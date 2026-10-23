---
id: CES-63
title: No catch-all modules
---

## Directive

No module is named `utils`, `helpers`, `aux`, `misc` or `common`, in Python, TypeScript,
JavaScript, Go and Rust: `utils.py`, `helpers.ts`, `common.go`, `misc.rs`. In TypeScript and
JavaScript a module is not named `util`, `shared` or `lib` either. Name a module for what it holds.

The directive names files. A directory of domain-named modules (`utils/text.py`, `utils/dates.ts`,
`utils/mod.rs`) is allowed in every language: a directory is a namespace and forces its contents to
be named. In Go other code imports the package, which is the directory, so the name of a Go package
is a separate check: `utils` and `common` are not package names (the rule `package-naming` of
revive).

## Why

A file named `utils.py` collects unrelated helpers. A module named for what it holds is found by
that name.

## Exceptions

A repository that already has such a file keeps it (CES-30). The standard applies to the files
that are added.

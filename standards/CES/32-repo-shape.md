---
id: CES-32
title: Keep non-code artifacts out of the package
---

## Directive

Notebooks, resources, reports and data live at the top level of the repository (`notebooks/`,
`resources/`, `reports/`, `data/`), outside the importable package. It applies to a flat layout and
to a `src` layout.

In Python, the tests live in a `tests/` directory beside the package. Go keeps a test beside its
code (`_test.go`) and Rust in the file it tests (`#[cfg(test)]`); there the tests follow the tool.

## Why

Notebooks and datasets make the wheel larger and trip the check for large files.

## Exceptions

A package that ships a small data file on purpose names it as an exception. The default is that
artifacts go to the top level.

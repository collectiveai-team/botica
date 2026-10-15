---
id: CES-32
title: Keep non-code artifacts out of the package
---

## Directive

Notebooks, resources, reports and data live at the top level of the repository (`notebooks/`,
`resources/`, `reports/`, `data/`), never inside the importable package. The standard does not
choose between a flat layout and a `src` layout: it keeps non-code artifacts out of whichever
import path the repository has.

## Why

Notebooks and datasets bloat the wheel, break reproducible imports and trip the check for large
files. The import path stays pure code.

## Exceptions

A package that ships a small data file on purpose names it as an exception. The default is that
artifacts go to the top level.

## Rules

- `no-notebooks`
- `no-artifacts-in-package`

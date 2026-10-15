---
id: CES-63
title: No catch-all modules
---

## Directive

No module file is named `utils.py`, `helpers.py`, `aux.py`, `misc.py` or `common.py`. Name a module
for what it holds. A directory of domain-named modules (`utils/text.py`, `utils/dates.py`) is
allowed: a directory is a namespace and forces its contents to be named.

## Why

A catch-all file accretes unrelated helpers with no cohesion. A name that says what the module
holds keeps it discoverable.

## Exceptions

A repository that already has such a file keeps it (CES-30). The standard applies to the files
that are added.

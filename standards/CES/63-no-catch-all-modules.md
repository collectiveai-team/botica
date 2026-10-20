---
id: CES-63
title: No catch-all modules
---

## Directive

No module file is named `utils.py`, `helpers.py`, `aux.py`, `misc.py` or `common.py`. Name a module
for what it holds. A directory of domain-named modules (`utils/text.py`, `utils/dates.py`) is
allowed: a directory is a namespace and forces its contents to be named.

## Why

A file named `utils.py` collects unrelated helpers. A module named for what it holds is found by
that name.

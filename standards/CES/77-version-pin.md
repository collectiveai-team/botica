---
id: CES-77
title: The Python version is a suggestion
---

## Directive

The `requires-python` of a project is a suggestion; no CI job or hook checks it. The default for a
new repository is 3.15. The existing pin of a repository is honoured as it is
(CES-30).

## Why

Python 3.15.0 was released on 2026-10-09 (PEP 790), so a new repository starts on the newest
release. Enforcing one minor version fights the availability of dependencies, and some packages
publish wheels for a new release months after it: in October 2026 torch had none for 3.15 on PyPI.
The pin is informational.

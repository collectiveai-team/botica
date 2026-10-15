---
id: CES-77
title: The Python version is a suggestion
---

## Directive

The `requires-python` of a project is a suggestion and never a gate of CI or of a hook. The house
default for a new repository is 3.14. The existing pin of a repository is honoured as it is
(CES-30).

## Why

Enforcing one minor version fights the availability of dependencies, and the ecosystem of 3.14
still has gaps. The pin is informational.

## Rules

None. It is judgment.

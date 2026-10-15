---
id: CES-67
title: Typed, declarative command lines
---

## Directive

A command line program uses a typed, declarative framework, Typer or Cyclopts, and renders with
Rich. It does not parse arguments by hand with `argparse`, `click` or `sys.argv`. Configuration
that arrives from the environment or flags goes through `pydantic-settings`.

## Why

A typed framework gives `--help`, coercion and validation for free and keeps an entry point
readable as its surface grows. `click` is caught on purpose: Typer is built on it, and importing it
directly bypasses the typed layer. The choice between Typer and Cyclopts is open.

## Exceptions

Reading `sys.argv[0]` for the name of the program, or handing `sys.argv` to a framework, is
legitimate. A legacy entry point is migrated when it is touched for another reason, not in a
change of its own.

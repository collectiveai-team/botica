---
id: CES-67
title: Typed, declarative command lines
---

## Directive

A command line program uses a typed, declarative framework, Typer or Cyclopts, and renders with
Rich. The framework parses the arguments, and `argparse`, `click` and `sys.argv` are not used for
that. Configuration that arrives from the environment or flags goes through `pydantic-settings`.

## Why

A typed framework provides `--help`, coercion and validation. Typer is built on `click`, and a
direct import of `click` bypasses the typed layer. Typer and Cyclopts are both accepted.

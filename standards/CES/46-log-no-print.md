---
id: CES-46
title: Libraries log, they don't print
---

## Directive

Library code logs and does not call `print` in Python or `console.log` in TypeScript. Entry points
of a command line program (`cli.py`, `__main__.py`) are exempt, because printing is their output.

## Why

`print` writes to stdout with no level and no context. A program that imports the library receives
that output.

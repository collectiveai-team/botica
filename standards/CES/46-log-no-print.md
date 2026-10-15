---
id: CES-46
title: Libraries log, they don't print
---

## Directive

Library code logs and does not call `print`. Entry points of a command line program (`cli.py`,
`__main__.py`) are exempt, because printing is their output.

## Why

`print` cannot be filtered, redirected by level or given context. It also pollutes the output of
whoever imports the library.

## Rules

- `log-no-print`

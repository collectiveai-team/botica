---
id: CES-46
title: Libraries log, they don't print
references: ["structured-logging"]
---

## Directive

Library code logs through a logger and does not write to stdout or stderr. The entry points of a
command line program and the tests may write there, because that output is their job.

| Language | Writes that are reported | Exempt |
|---|---|---|
| Python | `print()`, `sys.stdout.write()`, `sys.stderr.write()` | `__main__.py`, `cli.py`, `cli/`, an `if __name__ == "__main__":` block, `conftest.py`, `tests/` |
| TypeScript and JavaScript (`.ts`, `.tsx`, `.js`) | `console.log/info/debug/warn/error/trace/dir/table`, `process.stdout.write()`, `process.stderr.write()` | `cli.*`, `cli/`, `bin/`, an `if (import.meta.main)` or `if (require.main === module)` block, `*.test.*`, `*.spec.*`, `__tests__/`, `tests/` |

An entry point with another name goes in the `extend-ignores` of the rule.

## Why

`print` writes to stdout with no level and no context. A program that imports the library receives
that output.

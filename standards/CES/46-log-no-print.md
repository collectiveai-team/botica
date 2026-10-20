---
id: CES-46
title: Libraries log, they don't print
references: ["structured-logging"]
---

## Directive

Library code logs through a logger and does not write to stdout or stderr. The entry points of a
command line program and the tests may write there, because that output is their job.

## Why

`print` writes to stdout with no level and no context. A program that imports the library receives
that output.

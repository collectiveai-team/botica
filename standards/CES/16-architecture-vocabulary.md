---
id: CES-16
title: Architectural vocabulary
---

## Directive

Name units with this architectural vocabulary, consistently. Layers and packages are `entrypoints`,
`api`, `database`, `impl` and `core` (CES-5, CES-17, CES-18). Ports are interfaces and their
implementations are adapters. A concept that already has a name does not get a synonym such as
`manager`, `handler`, `helper`, `service_of_services` or `processor`.

## Why

With one word for each concept, `core` means the same thing in every repository, and a reader or an
agent finds a unit of an unfamiliar repository by its name. Two names for one concept are how two
modules end up doing the same job.

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

- A shared vocabulary makes the structure greppable and predictable: `core` always means the same
  thing, so a reader or an agent can navigate an unfamiliar repo by name alone.
- Synonym drift is how two modules end up doing the same job under different names. Agreeing on the
  words keeps two modules from doing the same job under different names.

## Do and don't

| Concept | Use | Avoid |
|---|---|---|
| inbound HTTP layer | `api` | `web`, `rest`, `endpoints_v2` |
| persistence unit | `database` | `persistence`, `dal`, `meta` |
| reusable domain core | `core` | `common`, `shared`, `utils` |
| port implementation | adapter | `manager`, `handler` |

## Naming a new unit

Reach for the established term first. Introduce a new word when the concept is new, and add it to
the vocabulary.

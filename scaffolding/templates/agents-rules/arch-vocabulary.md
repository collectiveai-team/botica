# CES-16 · architectural vocabulary

**Code:** `CES-16` &nbsp;·&nbsp; **Slug:** `arch-vocabulary` &nbsp;·&nbsp; **Tier:** judgment
&nbsp;·&nbsp; **Tracker:** [#16](https://github.com/collectiveai-team/botica/issues/16)

## Directive

Name units with the house architectural vocabulary, consistently. Layers and packages are
`entrypoints`, `api`, `database`, `impl`, `core` (CES-5/17/18); ports are interfaces, their
implementations are adapters. Don't invent ad-hoc synonyms (`manager`, `handler`, `helper`,
`service_of_services`, `processor`) for concepts that already have a name.

## Why

- A shared vocabulary makes the structure **greppable and predictable**: `core` always means the
  same thing, so a reader (or an agent) can navigate an unfamiliar repo by name alone.
- Synonym drift is how two modules end up doing the same job under different names. Agreeing on
  the words is the cheapest way to keep the design legible.

## Do / Don't

| Concept | Use | Avoid |
|---|---|---|
| inbound HTTP layer | `api` | `web`, `rest`, `endpoints_v2` |
| persistence unit | `database` | `persistence`, `dal`, `meta` |
| reusable domain core | `core` | `common`, `shared`, `utils` |
| port implementation | adapter | `manager`, `handler` |

## Judgment

This is a judgment-tier standard: no linter enforces it. When you name a new unit, reach for the
established term first; introduce a new word only when the concept is genuinely new, and then add
it to the vocabulary.

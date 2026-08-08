---
title: Use the exact architecture terms; avoid the substitutes
section: arch
scope: general
applies-to: all
status: direction
tags: architecture, vocabulary, language, matt-pocock
---

## Use the exact architecture terms; avoid the substitutes

Architecture discussions are precise only when everyone uses the same words. Substitutes
introduce ambiguity: "service" is overloaded across microservices, DI, and business layers;
"boundary" is claimed by DDD. Use the terms from the `arch-deep-modules` glossary in code
review, design docs, PR comments, and conversation.

| Use this | Not this | Why the substitute is weaker |
|---|---|---|
| **module** | component, service, unit | scale-agnostic; "service" implies network, "unit" implies test granularity |
| **interface** | API, signature, contract | API = HTTP endpoint to most; signature = only the type line, missing invariants |
| **implementation** | internals, body, logic | neutral; pairs precisely with module and interface |
| **depth** | complexity, thickness, size | depth is a measure of leverage, not of lines or cognitive load |
| **seam** | boundary, layer, separation | boundary is overloaded with DDD; layer implies stack order |
| **adapter** | wrapper, plugin, provider, impl | adapter states the role (fills a seam); the others describe shape or origin |
| **leverage** | reuse, DRY | leverage is about callers getting capability; reuse is about not repeating text |
| **locality** | cohesion, colocation | locality is about change and knowledge concentrating; cohesion is structural |

**Prefer (precise):**

```
# PR comment
"This module is shallow — the interface is nearly as complex as the
 implementation. Apply the deletion test: if we remove it, does the complexity
 vanish or spread to callers?"
```

**Avoid (vague):**

```
# PR comment
"This service/component seems like a thin wrapper. Maybe we don't need
 the abstraction layer?"
```

Reference: Matt Pocock `codebase-design` skill (github.com/mattpocock/skills) — "Use these
terms exactly — don't substitute 'component,' 'service,' 'API,' or 'boundary.' Consistent
language is the whole point." These terms appear consistently across `arch-deep-modules`,
`arch-deletion-test`, `arch-adapter-discipline`, and `arch-interface-is-test-surface`.

---
title: CES
---

# CES

The Collective Engineering Standard is the set of standards of Collective. Each standard is one
page, `<number>-<slug>.md`, and a rule cites it as `standard = "CES-<number>"`.

A standard has this shape:

```markdown
---
id: CES-71
title: Keep files small
---

What the standard requires.

## Why

The reason, in a few sentences.

## Examples

What follows the standard and what does not.
```

The rules that implement a standard are the ones whose `standard` field names it. A standard can
have several rules, from different families, and a rule can implement no standard.

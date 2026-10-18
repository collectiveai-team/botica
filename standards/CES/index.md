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

## Directive

What the standard requires.

## Why

The reason, in a few sentences.

## Examples

Code that follows the standard and code that breaks it.
```

`## Directive` and `## Why` are in every standard. `## Examples` and the sections that a standard
adds for itself (`## Layout`, `## Migration`, `## Suppression`) are optional. The title names the
requirement and not the tool that checks it, because the sidebar shows it.

The front matter can also carry `references`, a JSON list of the ids of the topics that show code
or documentation for the standard (ADR 0007): `references: ["structured-logging"]`. The page of the
standard lists those topics at the end.

The rules of a standard are the rules whose `standard` field names it, and the page of the standard
lists them at the end. A standard has any number of rules, none included. A rule names at most one
standard.

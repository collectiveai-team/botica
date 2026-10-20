---
id: CES-30
title: Respect the local repo
---

## Directive

A repo's existing, deliberate choices win over the defaults of a standard. An existing version
pin, lint config, dependency or layout is not overwritten because the default differs. When a
standard conflicts with a considered local decision, the change is an explicit, discussed
migration and not a silent rewrite.

## Why

The configuration of a repository often records a decision that cost something to reach.
Overwriting it to match a default loses that context and can break the build, and nobody reviews a
rewrite that the description of the change does not mention.

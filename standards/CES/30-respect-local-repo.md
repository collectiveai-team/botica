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

- A tool that bootstraps a repo adds what is missing and defers to what is already there. The same
  restraint applies to people and agents working in the repo.
- The configuration of a repository records decisions that its team made. Overwriting it to match
  a default can break the build.

## In practice

- An existing `requires-python` pin wins over the default of CES-77.
- An existing `prek.toml` or `pyproject.toml` is not replaced: new rules are merged in by hand,
  deliberately.
- A standard is adopted on the next natural touch, or as a named migration pull request, and never
  as a drive-by overwrite.

## When in doubt

Standards are defaults. Add alongside and leave the existing choice intact until a migration is
agreed.

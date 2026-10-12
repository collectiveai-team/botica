# CES-30 · respect the local repo

**Code:** `CES-30` &nbsp;·&nbsp; **Slug:** `general-respect-local-repo` &nbsp;·&nbsp; **Tier:**
judgment &nbsp;·&nbsp; **Tracker:** [#30](https://github.com/collectiveai-team/botica/issues/30)

## Directive

A repo's existing, deliberate choices win over house defaults. Don't overwrite an existing
version pin, lint config, dependency, or layout just because the standard default differs. When a
house standard conflicts with a considered local decision, treat it as an **explicit, discussed
migration** — not a silent rewrite.

## Why

- The scaffolder is **clean-adds-only** for exactly this reason: it adds what's missing and
  defers to anything already present (existing files are never edited or overwritten). The same
  restraint applies to humans and agents working in the repo.
- A repo's config often encodes hard-won, intentional decisions. Clobbering them to match a
  default destroys context and breaks builds.

## In practice

- An existing `requires-python` pin wins over the CES-77 default (which is why CES-77 ships as a
  comment).
- An existing `prek.toml` / `pyproject.toml` / `ast-grep` config is deferred, not replaced —
  merge new rules in by hand, deliberately.
- Adopt a house standard on the next natural touch, or as a named migration PR — never as a
  drive-by overwrite.

## Judgment

This is the meta-rule behind the whole standards rollout: standards are defaults, not edicts.
When in doubt, add alongside and leave the existing choice intact until a migration is agreed.

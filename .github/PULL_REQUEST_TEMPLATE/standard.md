<!-- Proposes a new standard or changes one. A pull request touches one standard. Keep prose brief. -->

## Summary

<!-- What the standard requires, in one sentence, and the smallest visual that shows it: code that
breaks it next to code that follows it, a file tree, or a diff. -->

## Standard

<!-- The file, `standards/<set>/<number>-<slug>.md`, and whether it is new or changed. -->

## Discussion

<!-- The thread in GitHub Discussions, category `standards`. -->

## Implementation

<!-- The rules that implement it (`standard = "..."` in their `rule.toml`). For each one: its kind
(native, ast-grep, extension), the languages it applies to and its default severity. If there is
none yet, say why. -->

## Reasoning

<!-- Why the standard exists: the problem it prevents and the evidence for it. Link the related
standards, rules, issues and discussions. -->

## Conflicts and risks

<!-- Where it collides with a local convention or with another standard, and the false positives
you expect. -->

## Evidence

<!-- Give reproducible evidence, not just "tests pass": `botica lint` on code that breaks the
standard and on code that follows it, with the exact commands. Mark anything not run. -->

- **Before:**
- **After:**

## Merge Danger

**Door:** <!-- one-way (hard to reverse) or two-way (cheap to roll back) -->

**Blast Radius:** <!-- who is affected and how noisy turning it on is: error or warning, which repositories and layers -->

**Migration:** <!-- how a repository that does not follow it yet adopts it: opt in, baseline, phased rollout, or new repositories only -->

## Documentation

<!-- What this change leaves out of date: the README of a rule, docs/, AGENTS.md. -->

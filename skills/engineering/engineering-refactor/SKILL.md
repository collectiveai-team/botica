---
name: engineering-refactor
description: Refactor an existing codebase in small, tested slices while keeping its local conventions, using botica for the rules and the upstream codebase-design skill for the architecture vocabulary. Use when the user asks to refactor, restructure, reorganize, clean up architecture, tidy up code, modernize a project, or adapt a repo to these practices.
---

# Engineering Refactor

Move an existing codebase toward a better structure without overriding the deliberate choices of
the repository. This skill is a workflow and states no rule of its own: the rules are in botica,
and the architecture vocabulary is the one of the upstream `codebase-design` skill.

## 1. Baseline

Read the judgment rules of `AGENTS.md` (between `<!-- botica:rules -->` and
`<!-- /botica:rules -->`), `GLOSSARY.md` and the ADRs under `docs/adr/`. Then read the local
contracts and write down, with the file that shows each:

- package manager, lockfile and build backend;
- lint, format and type-check tools, and where they run (prek, CI);
- language versions, package layout, test layout and entry points.

Keep that toolchain unless the user asked to migrate it (`botica explain respect-local-repo`). Run `botica lint` once and keep its output as the baseline.

## 2. Targets

If the user named a module or a pain point, start there. Otherwise ask the user whether to run the
upstream `improve-codebase-architecture` skill, which scans the codebase for deepening candidates;
an agent cannot invoke it on its own.

Invoke `codebase-design` and use its terms (module, interface, implementation, depth, seam,
adapter, leverage, locality) in every proposal. Add the findings of the baseline that touch the
target. For each rule involved, `botica explain <slug>` gives its text and
`botica refs show <slug>` the code it expects.

## 3. Slices

- Keep behaviour stable unless the user asked for a change.
- Before moving code, cover the behaviour with tests at the interface that callers use.
- Change one seam per slice; run the tests after each.
- Keep public imports and entry points compatible unless the task includes a breaking change.
- Leave the formatting of untouched code as it is.

## 4. Verify

Run the narrowest checks first, then broaden: the tests of the touched code, `botica lint` on the
changed files, the type checker, then the hooks of the repository (`uvx prek run --all-files` when
it uses prek). Compare the findings with the baseline. Report the checks that could not run.

## Output

Report what changed, which finding or proposal drove each change (cite the slug of the rule), the
checks that ran and their results, and the risks left: large files, shallow modules, missing tests.

Credits and sources: [`references/CREDITS.md`](references/CREDITS.md).

---
name: review-architecture
description: Review role that checks the structure a diff adds — layers and the direction of imports, ports and adapters, abstractions with one user, duplicated concepts, and functions that both order steps and carry them out. Use when reviewing a branch or PR that adds modules or changes boundaries, or when review-panel runs the architecture role.
---

# Review: architecture

This role asks whether the change leaves the code easier or harder to change next time.

## 1. Inputs

1. Pin the target and capture `git diff <target>...HEAD` and `git diff --stat <target>...HEAD`.
2. Read the role file: `git show <target>:.github/review/architecture/REVIEW.md`, or
   [`references/REVIEW.md`](references/REVIEW.md) when the repository has none.
3. Read `GLOSSARY.md`, the ADRs under `docs/adr/` that name a touched module, and run
   `botica explain` for the architecture rules of `AGENTS.md` (BTARCH00x).

## 2. Procedure

1. Draw the modules the diff touches and the imports between them, before and after. Note an
   import that goes against the layers of the ADRs or of BTARCH003.
2. For each new type, trait, interface or module: who uses it? An abstraction with one user and no
   second planned is a finding; an existing one that the change bypasses is too.
3. For each new name: does `GLOSSARY.md` or the code already have a term for the concept?
4. For the largest new function: does it order steps and also carry one of them out (BTARCH002)?
5. When the design question is larger than the diff, say so and point at the upstream
   `improve-codebase-architecture` skill instead of redesigning in the review.

## 3. Output

Write the result as described in [`references/output.md`](references/output.md), with role
`architecture`, to `.tmp/review/<short-head-sha>/architecture.json`.

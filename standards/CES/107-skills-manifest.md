---
id: CES-107
title: Track the skills manifest
---

## Directive

`skills-lock.json` is the skills manifest of the repo: the declared set of agent skills and where
each one comes from. It is committed. `.agents/skills/` is derived from it, gitignored and rebuilt
on install, and is never the source of truth.

The split is the one of `pyproject.toml` and `uv.lock`: the manifest is the resolved set and
`.agents/skills/` is the `.venv`. Installing the skills is part of setup, and it restores them from
the manifest.

## The skills CLI writes the manifest

A skill is added with `npx skills add <source> --skill <name>` and removed with
`npx skills remove`. The file is not written by hand or by another tool: its schema carries more
per entry than a narrower model keeps, and writing it again from such a model drops fields such as
`skillPath`, `computedHash` and `ref`. The upstream source (`vercel-labs/skills`,
`src/local-lock.ts`) says that the file "is meant to be checked into version control" and that
`skillPath` is "required to re-install only this skill on update".

## What the manifest records

`ref` is a branch or a tag, and a tag moves. The manifest reproduces which skills a repo has, from
where and at which ref. It is called a manifest and not a lock for that reason.

## Why

Without a tracked manifest the skill set of a repo cannot be reproduced and its drift is invisible.
A fresh clone gets whatever the installer's list says at that time. Committing the declaration makes the skill set reviewable in a diff, like any
other dependency.

## Drift

The manifest and `.agents/skills/` agree in both directions:

- A declared skill that is absent from `.agents/skills/` means that the tree is stale.
- An installed skill that is not declared exists on one machine only, because `.agents/skills/` is
  gitignored, and it disappears on a fresh clone.

A skill that the repo dropped from its manifest is a decision of the repo (CES-30), and a baseline
does not add it back.

## Suppression

A repo declares its skills or it does not. To drop a skill, run `npx skills remove` and commit the
manifest.

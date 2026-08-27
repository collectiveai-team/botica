# CES-107 · track the skills manifest

**Code:** `CES-107` &nbsp;·&nbsp; **Slug:** `skills-manifest` &nbsp;·&nbsp; **Enforced by:**
`scaffolding check` &nbsp;·&nbsp; **Tracker:**
[#107](https://github.com/collectiveai-team/scaffolding/issues/107)

## Directive

`skills-lock.json` is the repo's **skills manifest**: the declared set of agent skills and where
each comes from. **Commit it.** `.agents/skills/` is *derived* from it — gitignored, rebuilt on
install, never the source of truth.

This is the `pyproject.toml` / `uv.lock` split. The house baseline in `components.py` is the
declared intent; `skills-lock.json` is the resolved set; `.agents/skills/` is the `.venv`.
Installing skills is part of setup: `scaffolding install` restores from the manifest rather than
re-running a hardcoded list of `skills add` commands.

## The `skills` CLI owns this file. Never write it from scaffolding.

This is the rule the rest of the design hangs off, and it is not stylistic. The upstream schema
(`vercel-labs/skills`, `src/local-lock.ts`) carries more per entry than any Python model here
needs, and its own comments state the cost of dropping fields:

> `/** Path to the skill's SKILL.md within the source repo. Required to re-install only this
> skill on update — without it, an update would refetch every skill in the source repo. */`

Re-serialising the file from a narrower model silently drops `skillPath`, `computedHash` and
`ref`. Read it; plan from it; let `npx skills` write it. Adding a skill is `npx skills add`, which
records the source, ref and hash actually used.

The same file is explicit that committing it is intended:

> `/** This file is meant to be checked into version control. */`

## What it does and does not guarantee

Measured against `skills@1.5.22`, not assumed:

- `ref` records the branch or tag used at install time — **but only when the source was pinned**
  (`owner/repo#v1.2.3`). An unpinned source writes no `ref` at all. Tags are mutable, so even
  pinned this is tag-level, not commit-level.
- `computedHash` is written on `add` and compared only on the `experimental_sync`/node_modules
  path. **Restore never verifies it.** There is no integrity guarantee.
- `experimental_install` is exactly as experimental as its name. It is not documented in the
  upstream README.

So it buys reproducibility in *which skills, from where, at what ref* — not byte-reproducibility.
Say "manifest". Do not promise `uv.lock`-grade pinning you do not have.

## Why

Without a tracked manifest a repo's skill set is unreproducible and its drift is invisible. A
fresh clone gets whatever the installer's hardcoded list happens to say that week, and nobody can
tell what the repo is supposed to have. Committing the declaration makes the skill set reviewable
in diffs like any other dependency.

## Two states

| Repo state | Install behaviour |
| --- | --- |
| Manifest present | restore from it — it is the source of truth |
| No manifest | seed the house baseline, which makes the CLI write the manifest |

Plus one refusal: a manifest that exists but does not parse is **deferred**, never overwritten.

If the manifest is gitignored, install offers to un-ignore it. That is the only consent this
standard asks for, because it is the only point where scaffolding edits a file the repo owns.

**The manifest is not topped up towards the house baseline.** A skill the repo dropped is a
decision, not a gap (CES-30). The baseline seeds a repo that has none; it never holds a repo to a
set it has since changed. Adding one back is `npx skills add`.

## Skills installed, but no manifest

The common migration case, and the one with two sharp edges. Provenance is **not recoverable from
disk**: an installed skill directory holds only `SKILL.md`, and with no manifest
`skills list --json` reports `source: null`. So seeding declares the house baseline, and install
warns about both hazards *before* it runs anything:

- **Installed, outside the baseline** (a private or hand-added skill). It stays on disk,
  undeclared, and `scaffolding check` fails on it until you run
  `npx skills add <source> --skill <name>`. No manifest entry is ever fabricated for it — we would
  have to guess the source, and a wrong source is worse than an honest gap.
- **Installed, sharing a baseline name.** `skills add` overwrites the directory, so a hand-edited
  copy is lost. The derived tree is gitignored, so there is no diff to recover it from. Warned,
  not skipped: skipping would leave it installed and undeclared, which is the state this standard
  exists to remove. Copy it out first if you edited it.

The end state is deliberate rather than tidy: the baseline is declared and installed, anything
else is on disk and flagged. `scaffolding check` names exactly what is left to do.

## Seed and restore ops must deliver

`skills add` is **not** atomic, and this is the sharp edge. Measured: adding `tdd triage
no-such-skill-xyz` installs the two real skills, writes the manifest, and **exits 0**. An unknown
name — an upstream rename, a typo — is skipped silently and the run is indistinguishable from a
clean one by exit code alone.

Exit codes therefore cannot detect this class of failure. Seed and restore ops declare a
post-condition instead: the named skills must exist **in `.agents/skills/` afterwards**. Checking
the manifest would not do — it is written by the same tool whose work is being verified.

A missing `npx` remains a non-fatal skip; offline installs are legitimate, silently-empty ones are
not.

## Drift is checked in both directions

`scaffolding check` verifies the manifest exists, is not ignored, is tracked, and then compares it
against the tree **both ways**. A declared-but-absent skill means the tree is stale. An
installed-but-undeclared skill is the worse case: `.agents/skills/` is gitignored, so it exists on
one machine and evaporates on a fresh clone — precisely the failure this standard eliminates.

## Suppression

There is none, and that is deliberate — a repo either declares its skills or it does not. To not
have a house-baseline skill, remove it with `npx skills remove` and commit the resulting manifest.
Nothing will put it back.

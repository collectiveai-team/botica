<p align="center">
  <img src="assets/logo.png" alt="scaffolding — agent-driven repo bootstrap + recurring skills" style="width:600px; max-width:100%; height:auto;" />
</p>

<p align="center">
  <em>Clean-adds-only repo bootstrap CLI — plus a couple of recurring agent skills.</em>
</p>

<p align="center">
  <a href="https://github.com/collectiveai-team/scaffolding/releases"><img alt="Release" src="https://img.shields.io/github/v/release/collectiveai-team/scaffolding?logo=github" /></a>
  <a href="https://www.python.org/"><img src="https://img.shields.io/badge/Python-3.11%2B-blue?logo=python&logoColor=white" alt="Python 3.11+"></a>
  <a href="https://opencode.ai/"><img src="https://img.shields.io/badge/Agents-opencode%20%C2%B7%20claude--code%20%C2%B7%20codex-1d1d1d?logo=anthropic&logoColor=white" alt="opencode · claude-code · codex"></a>
  <a href="https://docs.astral.sh/uv/"><img src="https://img.shields.io/badge/Install-uvx%20%C2%B7%20curl-261230?logo=astral&logoColor=white" alt="Install via uvx or curl" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
</p>

---

**Source Code**: [https://github.com/collectiveai-team/scaffolding](https://github.com/collectiveai-team/scaffolding)

---

Personal repo bootstrap + agent skills for opencode / claude-code / codex.

This repo does two things:

1. **Bootstrap a repo** with my workspace defaults (gitignore, per-agent config,
   `AGENTS.md` guidance, optional prek hooks, ast-grep, CI, Varlock secrets) via a
   small Python CLI. This is a one-time-per-repo operation, run as an *agentic
   install* or directly — not an installed skill. Target one or more agents with
   `--agent` (repeatable / comma-separated; default `opencode`).
2. **Ship a couple of recurring skills** (`journalist`, `handoff`) that you
   install once and use repeatedly.

Most engineering workflow skills I use come from Matt Pocock's
[`skills`](https://github.com/mattpocock/skills). This repo intentionally does
not vendor those; it only contains my own additions.

## Bootstrap a repo

Run this from the root of the repo you want to set up.

**Existing working repo (recommended): let an agent do it.** The bootstrap needs
judgment — new-vs-existing detection, additive JSONC/`AGENTS.md` merges, Python
detection, and per-item conflict resolution. The CLI does the deterministic
clean-adds; the agent drives it and handles merges. Point your agent at the guide:

> Set up this repo by following the instructions here:
> `https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/guide.md`
> Don't summarize it — follow every step.

**New / empty repo (fast path): run the CLI directly.** It does clean adds only
and refuses to touch existing files, deferring any merge to the agent.

Straight from git via `uvx` (no PyPI):

```bash
uvx --from git+https://github.com/collectiveai-team/scaffolding scaffolding install
```

Or via the bootstrap shim (also installs `uv` if missing — preserves the classic
one-liner):

```bash
curl -fsSL https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/install.sh | bash
```

The installer is idempotent — safe to re-run. Existing files are never edited or
overwritten; they are reported as `[defer]` for the agent to merge.

### Commands

```
scaffolding install [components…]   # clean-adds (all default-on, or just the named ones)
scaffolding install --yes           # non-interactive / CI (conservative defaults)
scaffolding install --dry-run       # render the plan, write nothing
scaffolding plan --json             # machine-readable plan for the agent path
scaffolding list                    # available components (gate / default / what they add)
scaffolding check                   # verify bootstrap completeness (nonzero exit on failure)
scaffolding doctor                  # diagnose environment + tools
```

Components: `gitignore agent-config prek ast-grep pyproject ci agents standards
skills varlock` (all default-on except `ci`, which is opt-in). Scope with
positional names or `--skip a,b`. Useful flags: `--agent` (repeatable:
`opencode`/`claude-code`/`codex`), `--ci/--no-ci`, `--ci-parts`,
`--name`, `--description`, `--varlock/--no-varlock`, `--no-deps`. Legacy env vars
(`AGENT`, `SKIP_SKILLS`, `SKIP_VARLOCK`, `WITH_CI`/`SKIP_CI`, `ASSUME_YES`) are
honored.

There is no `uninstall`: the installer requires a git repo, so `git status` /
`git checkout` / `git clean` are the undo mechanism.

## Installed skills

These are real skills you install once and use repeatedly. They land in the
shared `.agents/skills` standard (read by opencode + codex); selecting
`claude-code` bridges them via the `.claude/skills` → `.agents/skills` symlink.

Which skills a repo uses is declared in **`skills-lock.json`, the tracked skills
manifest** (CES-107). `.agents/skills/` is derived from it and stays gitignored.
`scaffolding install` restores from the manifest rather than re-fetching a
hardcoded list:

```bash
npx skills experimental_install
```

Think `pyproject.toml` / `uv.lock`: the baseline below is the declared intent,
`skills-lock.json` is the resolved set, `.agents/skills/` is the `.venv`. The
analogy stops at the guarantee. Entries carry a `ref` only when the source was
pinned (`owner/repo#v1.2.3`), tags are mutable, and the `computedHash` that is
written is never verified on restore. So it reproduces *which skills, from where,
at what ref* — not bytes. It is a manifest, not a lock.

The `skills` CLI owns the file; scaffolding reads it and never writes it. Adding a
skill is `npx skills add`, which records the source, ref and hash actually used.

Two states, and one refusal. A repo **with** a manifest restores from it — it is
the source of truth, and is never topped up towards the house baseline, so a skill
you removed stays removed. A repo **without** one gets the baseline seeded, which
creates it. A manifest that exists but does not parse is deferred, never
overwritten. If the manifest is gitignored, install offers to un-ignore it — the
one consent this asks for, since it is the one file the repo owns that scaffolding
edits.

The baseline it seeds is the curated upstream set from Matt Pocock, pinned to a
release tag, plus this repo's local skills and varlock — the exact commands are
below under [seeding by hand](#seeding-by-hand). The catalog and the tag live in
`scaffolding/skills.py`; the `skills-drift` workflow reports renames, removals
and newer tags.

Skills install once into `.agents/skills`; claude-code reaches them via the
`.claude/skills` symlink created by the bootstrap, so there is no need to re-run
the installer with `--agent claude-code` / `--agent codex`.

### Working with skills

| Situation | What to run |
| --- | --- |
| New repo | `scaffolding install`, then commit `skills-lock.json` |
| Existing repo, first adoption | `scaffolding install`, accept the un-ignore prompt, commit |
| Fresh clone / CI | `scaffolding install` (or `npx skills experimental_install`) |
| Add a skill | `npx skills add <source> --agent opencode --yes --skill <name>`, commit |
| Remove a skill | delete the directory **and** the manifest entry, `scaffolding check`, commit |
| Skills installed, no manifest | `scaffolding install`, then read the warnings — see below |

`skills.py` is not part of any of these. It is the baseline for repos that have
no manifest yet; changing a repo's skill set is always `npx skills`, and the CLI
records the source, ref and hash it used.

#### Skills installed, but no manifest

The common migration case. Provenance is **not recoverable from disk**: an
installed skill directory holds only `SKILL.md`, and with no manifest
`skills list --json` reports `source: null`. So nothing is adopted or guessed —
the baseline is seeded and install warns about both hazards before it runs:

```
[warn] skills-lock.json: our-private-skill
       installed but not in the house baseline, and its source cannot be
       recovered from disk — run `npx skills add <source> --skill our-private-skill`
[warn] .agents/skills/tdd
       already on disk and will be replaced by the seed — if it was edited by
       hand, copy it out first; the derived tree is gitignored, so there is no
       diff to recover it from
```

Afterwards `scaffolding check` names exactly what is left:

```
FAIL  installed skills declared  in .agents/skills but not in skills-lock.json:
                                 our-private-skill
```

Declare it with its real source, or delete the directory. Either way the manifest
and the tree end up agreeing, which is what the check enforces.

#### Removing a skill

`npx skills remove` **does not work** for projects using `.agents/skills/`. That
directory is shared by ~15 agents, and the CLI keeps both the files and the
manifest entry while any other detected agent still references the skill
(`src/remove.ts:262-302`), so the command reports success and changes nothing.
`--agent '*'` is documented but rejected at runtime. Until that is fixed upstream,
remove by hand:

```bash
rm -rf .agents/skills/<name>
# delete the "<name>" entry from skills-lock.json
scaffolding check    # fails if you did only one of the two
git add -A && git commit
```

`scaffolding check` compares the manifest and the tree in both directions
precisely so a half-finished removal cannot pass silently. Because the baseline is
never re-applied, a removed skill stays removed.

#### Seeding by hand

Instead of running the CLI:

```bash
npx skills add 'mattpocock/skills#v1.2.3' --agent opencode --yes --skill grill-with-docs triage improve-codebase-architecture setup-matt-pocock-skills to-spec to-tickets implement wayfinder prototype diagnosing-bugs research tdd domain-modeling codebase-design code-review resolving-merge-conflicts wizard grill-me teach writing-for-agents grilling wait-what
npx skills add collectiveai-team/scaffolding --agent opencode --yes --skill ask-user journalist handoff test-smell-review
npx skills add dmno-dev/varlock --agent opencode --yes
```

The `#<tag>` fragment is the only pin the `skills` CLI accepts — `owner/repo@tag`
is silently parsed as a skill-name filter, and an unpinned install tracks the
upstream default branch. Bump the tag in `scaffolding/skills.py`, never here.

From a local checkout, install local skills with
`npx skills add . --agent opencode --yes --skill ask-user journalist handoff test-smell-review --full-depth`.

## Upstream skills from Matt Pocock

User-invoked engineering workflows:

- `grill-with-docs`, `triage`, `improve-codebase-architecture`, `setup-matt-pocock-skills`.
- `to-spec`, `to-tickets`, `implement`, `wayfinder`.

Model-invoked engineering workflows:

- `prototype`, `diagnosing-bugs`, `research`, `tdd`.
- `domain-modeling`, `codebase-design`, `code-review`, `resolving-merge-conflicts`.
- `wizard` — generates an interactive bash script for the steps only a human can
  take (provisioning, credentials, CI secrets, dashboard clicks).

Productivity workflows:

- User-invoked: `grill-me`, `teach`, `wait-what`.
- Model-invoked: `grilling`, `writing-for-agents`.

These land as editable files you own, but `npx skills add` / `skills update`
overwrite them in place without a diff or a prompt, so keep local edits in this
repo's own `skills/`, not in `.agents/skills`.

## What's in this repo

- `scaffolding/` — the bootstrap CLI (Cyclopts + Questionary + pydantic-settings
  + Rich). `cli.py`, `engine.py`/`plan.py`, `components.py`, and `templates/`
  (per-agent config — opencode.jsonc / .claude settings, prek hooks, pyproject,
  ast-grep rules, CI workflows, AGENTS.md section).
- `install.sh` — thin bootstrap shim (ensure `uv`, then run the CLI from git).
- `guide.md` — the agentic-install guide (judgment layer).
- `skills/productivity/journalist` — local daily session journals under `.journals/`.
- `skills/productivity/handoff` — compact the current session into a temp-dir handoff for another agent.
- `skills/productivity/ask-user` — route a situation to the smallest suitable installed workflow.
- `skills/quality/test-smell-review` — review tests for false-green smells (pass without
  protecting anything) using an in-context judgment protocol, adapted from
  `falsegreen-skill` without its external LLM-CLI dependency.
- `docs/engineering-standards.md` — **CES (Collective Engineering Standard)**: how house
  rules are coded, cited (`CES-<issue#>` + slug), and shipped. Single source of truth,
  referenced by `AGENTS.md`.

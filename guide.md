# Scaffolding — Agentic Install Guide

This is the judgment layer for the `scaffolding` bootstrap. The deterministic
work (clean-adds, gating, the plan) lives in the CLI; your job as the agent is to
drive that CLI, surface the decisions the **user** must own, and handle the
merges the CLI deliberately refuses to do.

Point an agent at the raw URL of this file and have it follow every step:

`https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/guide.md`

For a brand-new/empty repo you can skip the agent and run the CLI directly
(clean adds only; it refuses to touch existing files):

```bash
uvx --from git+https://github.com/collectiveai-team/scaffolding scaffolding install
# or the shim (also bootstraps uv):
curl -fsSL https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/install.sh | bash
```

The CLI is **clean-adds-only**: it never edits, merges, reorders, or overwrites
existing files. Existing targets are reported as `[defer]` and left untouched —
those are yours to merge, per this guide.

## Model A flow (how to drive the CLI)

1. **Inspect + plan.** Run `scaffolding plan --json`. You get
   `{facts, clean_adds, runs, skips, defers, decisions_needed, deferred_merges,
   notices}`. `facts` is the Tier-0 detection (git root, python?, Dockerfile?,
   visibility, available tools). `defers`/`deferred_merges` are the existing
   files you'll merge by hand.
2. **Surface decisions to the user.** For each entry in `decisions_needed`, ask
   the user — do not pick for them (see the taxonomy below). Typical keys:
   `agents`, `pyproject_name`, `pyproject_description`, `ci_parts`, `varlock`.
3. **Apply clean-adds.** Run
   `scaffolding install --yes <answers as flags>` (e.g. `--agent opencode
   --agent claude-code --name foo --ci --ci-parts tests,security`). `--yes` keeps
   it non-interactive; your gathered answers come in as flags. `--agent` is
   repeatable / comma-separated — pass every agent the user targets.
4. **Handle merges yourself.** For every `defer`, additively merge per the
   per-area rules below. This is the part that needs judgment — the CLI won't do
   it.
5. **Verify.** Run `scaffolding check` (nonzero exit on any failure) and
   `scaffolding doctor` for environment issues.

For a new/empty repo with no defers and no policy decisions, just run
`scaffolding install` and you're done.

## New vs. existing repos

**New or empty repo (clean setup).** No conflicting `.gitignore`, `opencode.jsonc`,
`prek.toml`, `AGENTS.md`, or `.env*`/`.env.schema`? Run `scaffolding install`
directly — apply every default, install skills, run `varlock init`, no
negotiation.

**Existing working repo (migration).** When any of those exist, do not silently
change an active project. Run `scaffolding plan --json` first, then present a
short installation plan and wait for the user's go-ahead, sorting every change
into three buckets:

- **Clean adds** — missing files/entries the CLI applies safely (a missing
  `.gitignore` line, a brand-new `opencode.jsonc`). Apply via `scaffolding
  install` once approved.
- **Merges** — additive edits to existing files (missing keys in
  `opencode.jsonc`, extra `prek.toml` hooks, the `AGENTS.md` section, new
  `pyproject.toml` tool sections). These show up as `defer`. Merge by hand; show
  what gets inserted and confirm no collisions.
- **Conflicts** — anything that would remove, replace, reorder, or restructure
  existing content, or where the user's value differs. Never auto-resolve; let
  the user decide per item and default to the most conservative option.

## Non-destructive edits (the merge contract)

- If a file exists, update it in place by adding only the missing entries.
- If a key already exists in `opencode.jsonc`, preserve its value and merge only
  missing nested keys or array items.
- If a change would remove, rename, replace, or restructure existing content,
  stop and ask the user first.
- Do not normalize formatting across a whole file just because it differs from
  the templates.

## Decision taxonomy (who decides what)

- **Tier 0 — Facts** (auto-detected, never asked): git root, python?,
  Dockerfile?, repo visibility, available tools.
- **Tier 1 — Clean-adds** (the CLI applies): gitignore lines, `opencode.jsonc`,
  prek hooks, ast-grep config, the `AGENTS.md` section when absent.
- **Tier 2 — Policy** (the USER decides, even on agentic install): CI opt-in and
  which parts; agent target(s) — any of `opencode`/`claude-code`/`codex`, one or
  more; `pyproject` name and description; skill set. Surface these from
  `decisions_needed`.
- **Tier 3 — Destructive** (USER, explicit confirm; default skip): `varlock init`
  when `.env.example`/`.env.schema` exists; `docker.yml` on a private/internal
  repo (GHCR billing). The CLI skips these with a notice unless told otherwise.
- **Tier 4 — Merges/conflicts** (your judgment): key-level merges into existing
  `opencode.jsonc`/`pyproject.toml`, updating an existing `AGENTS.md` section,
  clashing `.gitignore` handling. Always deferred by the CLI.

## Per-area merge rules

When a target is `deferred`, fetch the bundled template and merge additively.
Templates are bundled in the package; the raw URLs below are the fallback when
you need the content to merge by hand.

### `.gitignore`

Ensure these are present at the repo root (the CLI adds missing ones; merge by
hand only if the file conflicts). Keep `.env.schema` tracked:

```
.env
!.env.schema
.tmp/
.scratch/
.worktrees/
.journals/
```

### `prek.toml`

Additive only. Generic hooks always; Python hooks
([prek-python.toml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/prek-python.toml))
only for Python repos. Templates:

- [prek-generic.toml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/prek-generic.toml)
- [prek-python.toml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/prek-python.toml)
- [pyproject-template.toml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/pyproject-template.toml)

If a Python repo has no `pyproject.toml`, the CLI creates one from the template
with a guided `name`/`description`. If it exists, only add missing tool sections —
never replace dependencies or project metadata without explicit approval.

### `ast-grep`

The `ast-grep` hook in `prek-python.toml` runs `ast-grep scan`, which needs a
root `sgconfig.yml` pointing at a rule dir. The CLI auto-includes `ast-grep` when
`prek` is selected on a Python repo. When merging by hand, add both additively:

- [sgconfig-template.yml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/sgconfig-template.yml) → root `sgconfig.yml` (`ruleDirs: [ast-grep/rules]`)
- starter rules from [templates/ast-grep-rules/](https://github.com/collectiveai-team/scaffolding/tree/main/scaffolding/templates/ast-grep-rules) → `ast-grep/rules/` (`no-dict-call-return`, `no-dict-literal-return`, `no-dict-return-annotation`).

### GitHub Actions CI

Opt-in (Tier 2). `scaffolding install ci --ci-parts tests,security,docker,publish`
selects parts. House style derived from `aymurai-asr`: every `uses:` pinned to a
tag (never a branch) — note `astral-sh/setup-uv` must be pinned to an exact
version like `@v8.3.2`, since it stopped publishing major tags at v8.0.0 — plus a
`concurrency` group and least-privilege `permissions`. Templates:

- [zizmor.yml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/github/workflows/zizmor.yml) — workflow static analysis (any repo).
- [tests.yml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/github/workflows/tests.yml) — lint/type-check/test (Python `uv` repos).
- [osv-scanner.yml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/github/workflows/osv-scanner.yml) — dependency vuln scan via OSV.dev, reads `uv.lock` natively (Python `uv` repos; polyglot-ready). Replaces the old `pip-audit.yml`.
- [dependency-review.yml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/github/workflows/dependency-review.yml) — PR-level new-dependency/license visibility (any GitHub-hosted repo, summary-only).
- [commit-policy.yml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/github/workflows/commit-policy.yml) — CI enforcement of the no-AI-co-authorship commit policy (always-on, mirrors the commit-msg prek hook).
- [dependabot.yml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/github/dependabot.yml) — weekly updates (Python `uv` repos).
- [docker.yml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/github/workflows/docker.yml) — build/push to GHCR. Added only with a root `Dockerfile` **and** a public repo; the CLI skips it on private/internal repos with a GHCR-billing notice.
- Publish-only (opt-in, placeholders to fill): [release.yml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/github/workflows/release.yml), [pypi.yml](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/github/workflows/pypi.yml) — public Python packages with Trusted Publishing only.

Never overwrite an existing workflow; if a target exists, leave it and suggest
additive changes the user approves. Bump stale pinned action/tool versions.

### Per-agent config

The `agent-config` component writes config for each selected agent. `AGENTS.md`
(see below) is the shared brain read by **opencode** and **codex**; **claude-code**
is the exception — it reads `CLAUDE.md` + `.claude/skills`, so it is bridged with
symlinks.

- **opencode** → repo-root `opencode.jsonc` from
  [opencode-template.jsonc](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/opencode-template.jsonc).
  Merge additively: add only missing keys, plugin entries, and permission rules;
  preserve the user's values. Sets `$schema`, the `opencode-sessions-explorer` and
  `opencode-varlock@latest` plugins, and `permission` rules denying secret access
  (`.env*`, `*.pem`, `*.key`, `*credentials*`, `varlock.config`).
- **claude-code** → `.claude/settings.json` from
  [claude-settings-template.json](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/claude-settings-template.json)
  (a `permissions.deny`/`allow` mirror of the opencode secret rules), plus two
  clean-adds-only symlinks: `CLAUDE.md` → `AGENTS.md` and `.claude/skills` →
  `.agents/skills`. If either path already exists it is deferred — bridge by hand
  (never clobber). Merge `.claude/settings.json` additively into any existing file.
- **codex** → no repo-local file. Codex reads `AGENTS.md` natively and has no
  per-file deny-list; protection is varlock + AGENTS.md guidance + the leak-scan
  hook, plus user-level `~/.codex/config.toml` (`sandbox_mode="workspace-write"`,
  `approval_policy="on-request"`). See *Secret protection per agent* below.

### Secret protection per agent

`varlock init` is agent-agnostic (it only creates `.env.schema` and gitignores
`.env*`). The live context-redaction plugin (`opencode-varlock`) is **OpenCode-only**.

| Layer | opencode | claude-code | codex |
|---|---|---|---|
| `.env.schema` committed, `.env*` gitignored | ✅ | ✅ | ✅ |
| Repo-local read/bash deny-list | `opencode.jsonc` | `.claude/settings.json` | — (none) |
| Live context redaction plugin | `opencode-varlock` | — | — |
| AGENTS.md / varlock-skill guidance | ✅ | ✅ (via CLAUDE.md) | ✅ |
| Leak pre-commit hook (prek) | ✅ | ✅ | ✅ |

For codex, tell the user: never `cat .env` (use `varlock load`), rely on the
AGENTS.md rules + leak-scan hook, and harden `~/.codex/config.toml`
(`sandbox_mode`, `approval_policy`) — there is no live-redaction equivalent.

### Varlock secret management

Use [Varlock](https://varlock.dev) so secrets stay out of the repo and out of
agent context. Tier 3 — never overwrite an existing `.env.schema`/`.env` without
explicit approval. The CLI runs `varlock init` (or `npx varlock init --agent`)
only when no `.env.schema` exists; otherwise it defers. Commit `.env.schema`,
never `.env`.

### `AGENTS.md`

The core task: the managed `## Repo Workspace Defaults` section, copied verbatim
from [agents-workspace-defaults.md](https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/scaffolding/templates/agents-workspace-defaults.md).
The CLI appends it when absent and skips when the marker is present. If the
section exists but needs changes, update only the lines inside it and preserve
everything else. Do not edit `CLAUDE.md` during bootstrap unless asked.

## Skill installation

Skills install **once** into the shared `.agents/skills` standard (read by
opencode + codex). When `claude-code` is selected, the `agent-config` component's
`.claude/skills` → `.agents/skills` symlink makes the same skills visible to
Claude — do **not** re-run the installer per agent.

**`skills-lock.json` is the tracked skills manifest** (CES-107): it declares which
skills the repo uses and where they come from, and it is committed.
`.agents/skills/` is *derived* from it and stays gitignored. It is the
`pyproject.toml` / `uv.lock` split: the house baseline is the intent, this file is
the resolved set. The guarantee is weaker than `uv.lock` — entries carry a `ref`
only when the source was pinned (`owner/repo#v1.2.3`), tags are mutable, and
`computedHash` is written but never verified on restore. Call it a manifest.

**The `skills` CLI owns this file. Never write it from scaffolding, and never
hand-edit it.** Re-serialising it drops `ref`, `skillPath` and `computedHash`;
upstream notes that losing `skillPath` alone makes `update` refetch every skill in
the source repo. Adding a skill is `npx skills add`.

The `skills` component picks one of two paths:

| Repo state | What happens |
| --- | --- |
| Manifest present | restore from it — it is the source of truth |
| No manifest | seed the house baseline, which makes the CLI write the manifest |

Plus one refusal: a manifest that exists but does not parse is **deferred**, never
overwritten. If the manifest is gitignored, install offers to un-ignore it, a
decision defaulting to **yes** — the only consent this component asks for. Restore
is:

```bash
npx skills experimental_install
```

**The manifest is never topped up towards the house baseline.** A skill the repo
dropped is a decision, not a gap (CES-30). The baseline seeds a repo that has none.

Seeding uses `npx skills add <source> --agent opencode --yes --skill …`, and those
ops assert a post-condition: the named skills must exist **in `.agents/skills/`
afterwards**, or the install fails loudly. This is not belt-and-braces. `skills
add` is not atomic and does not signal partial failure — adding `tdd triage
no-such-skill-xyz` installs two skills, writes the manifest and exits **0**, so an
upstream rename is invisible to the exit code. The post-condition reads the tree
rather than the manifest, because the manifest is written by the same tool being
verified. A missing `npx` is still a non-fatal skip.

A repo with **installed skills but no manifest** is the common migration case.
Provenance is not recoverable from disk — an installed skill directory holds only
`SKILL.md`, and without a manifest `skills list --json` reports `source: null` — so
the baseline is seeded and both hazards are warned about before anything runs:

- a skill **outside the baseline** stays on disk, undeclared, and `scaffolding
  check` fails on it until `npx skills add <source> --skill <name>` declares it. No
  entry is fabricated; a guessed source is worse than an honest gap.
- a skill **sharing a baseline name** is overwritten by `skills add`. The tree is
  gitignored, so there is no diff to recover a hand-edited copy from. Copy it out
  first.

Both warnings are emitted at plan time. **Read them before accepting the plan** —
after the run there is no diff showing what the seed replaced. Afterwards
`scaffolding check` names what is still undeclared, and the fix is either
`npx skills add <source> --skill <name>` with the real source, or deleting the
directory.

### The house baseline

What a repo with no manifest gets seeded with — the curated upstream set, this
repo's recurring local skills (`ask-user`, `journalist`, `handoff`,
`test-smell-review`, `unattended-issue-driver`), and varlock:

```bash
npx skills add 'mattpocock/skills#v1.2.3' --agent opencode --yes --skill grill-with-docs triage improve-codebase-architecture setup-matt-pocock-skills to-spec to-tickets implement wayfinder prototype diagnosing-bugs research tdd domain-modeling codebase-design code-review resolving-merge-conflicts wizard grill-me teach writing-for-agents grilling wait-what
npx skills add collectiveai-team/scaffolding --agent opencode --yes --skill ask-user journalist handoff test-smell-review unattended-issue-driver
npx skills add dmno-dev/varlock --agent opencode --yes
```

The upstream set is pinned by the `#<tag>` fragment — do not rewrite it as
`mattpocock/skills@v1.2.3`, which the `skills` CLI parses as a skill-name filter
and installs from the default branch instead. Bump the tag in
`scaffolding/skills.py`, never here.

From a local checkout, install local skills with
`npx skills add . --agent opencode --yes --skill ask-user journalist handoff test-smell-review unattended-issue-driver --full-depth`.

### Changing a repo's skill set

Never by editing `skills.py` — that is only the baseline for repos with no
manifest. Always through the `skills` CLI, which records the source, ref and hash
it actually used:

| Situation | What to run |
| --- | --- |
| New repo | `scaffolding install`, then commit `skills-lock.json` |
| Existing repo, first adoption | `scaffolding install`, accept the un-ignore prompt, commit |
| Fresh clone / CI | `scaffolding install` (or `npx skills experimental_install`) |
| Add a skill | `npx skills add <source> --agent opencode --yes --skill <name>`, commit |
| Remove a skill | delete the directory **and** the manifest entry, `scaffolding check`, commit |
| Skills installed, no manifest | `scaffolding install`, then read the warnings above |

**Removing needs the manual path.** `npx skills remove` does not work for projects
using `.agents/skills/`: the directory is shared by ~15 agents and the CLI keeps
both the files and the manifest entry while any other detected agent still
references the skill (`src/remove.ts:262-302`), so it reports success and changes
nothing. `--agent '*'` is documented but rejected at runtime. Until that is fixed
upstream:

```bash
rm -rf .agents/skills/<name>
# delete the "<name>" entry from skills-lock.json
scaffolding check    # fails if you did only one of the two
git add -A && git commit
```

This is the one place a human edits the manifest, and it is a workaround, not the
intended flow. `scaffolding check` compares both directions precisely so a
half-finished removal cannot pass silently, and because the baseline is never
re-applied, a removed skill stays removed.

After installing, run the `setup-matt-pocock-skills` skill once to configure the
repo (issue tracker, triage labels, domain docs) that the other engineering
skills assume. The upstream skill defaults the issue tracker to GitHub; for this
workspace **default it to local markdown** (issues under `.scratch/<feature>/`),
since these repos are solo/agent-driven. Only set up GitHub, GitLab, or another
tracker when the user explicitly wants it.

## Verify

Run `scaffolding check` — it asserts the completeness checklist (gitignore
entries incl. `!.env.schema`; prek `betterleaks` hook; `.env.schema` tracked and
`.env` ignored; ast-grep config when the hook is present; the skills manifest
present, not gitignored, tracked, and reconciled against `.agents/skills/` in both
directions — declared-but-absent and installed-but-undeclared; the `AGENTS.md`
section, which is the only universal requirement). Agent config is validated
**only when present**: `opencode.jsonc` (schema/plugin/permission), `.claude/settings.json`
(secret `permissions.deny`), and `CLAUDE.md` → `AGENTS.md`. Then confirm any
deferred merges you applied by hand preserved all existing content, and that no
existing file, key, array item, or comment was removed without explicit user
approval.

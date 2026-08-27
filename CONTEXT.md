# Scaffolding

The bootstrap CLI that installs house standards, agent config, and agent skills into a target
repo. It is deterministic and clean-adds-only: it decides what a repo needs, then adds what is
missing without editing what is already there.

## Language

### Install engine

**Component**:
A selectable unit of scaffolding (`gitignore`, `skills`, `standards`, …) that inspects a repo and
returns ops without performing them.
_Avoid_: module, plugin, step

**Op**:
One planned operation the engine will execute. Planning and execution are strictly separated —
a component never touches the filesystem.
_Avoid_: action, task, command

**Clean-add**:
Writing something that is absent. The engine's default and, aside from the unignore op, its only
mutation. An existing target is deferred, never overwritten.
_Avoid_: create, upsert

**Defer**:
Leaving an existing target untouched and handing the merge to the agentic guide.
_Avoid_: skip, conflict

**Unignore op**:
The engine's single destructive capability: removing one literal, whitelisted line from
`.gitignore`. Bounded by a literal whitelist rather than a pattern language, so "clean-adds plus
a known set of line removals" stays checkable. See `docs/adr/0001-scoped-unignore-op.md`.

**Decision**:
A choice the engine refuses to make silently, surfaced to the user with a default. Unanswered
decisions take their default.
_Avoid_: option, setting, flag

**Post-condition**:
An assertion attached to a run op declaring what must be true **on disk** afterwards. Asserted
against the derived tree, never against a file the tool being verified just wrote. Its absence is
why an upstream rename removed a skill from every fresh install without anyone noticing.

### Skills

**Skills manifest**:
`skills-lock.json` — the tracked declaration of which skills a repo uses and where they come
from. **Owned by the third-party `skills` CLI**: scaffolding reads it and never writes it. It
records a `ref` only when the source was pinned, and its `computedHash` is never verified on
restore, so it reproduces which skills from where at what ref — not bytes.
_Avoid_: lockfile, skills lock, skill list

**Derived skills tree**:
`.agents/skills/` — the installed skill files. Rebuilt from the manifest, gitignored, and never
the source of truth.
_Avoid_: installed skills, skills directory

**House baseline**:
The skill set the scaffolder **seeds a repo with** when it has no manifest. A starting point, not
a set the repo is held to: once a manifest exists it is authoritative, and the baseline is not
reapplied.
_Avoid_: default skills, curated catalog, required skills

**Authored skill**:
A skill whose source of truth is this repo's `skills/` tree. Declared with `sourceType: local` so
restore reads the working copy instead of clobbering it with the published one.
_Avoid_: local skill, own skill

### Standards

**CES**:
Collective Engineering Standard — one house rule the scaffolder ships, tracked by exactly one
issue. Cited as `CES-<issue#>`; its machine id is a kebab-case slug. See
`docs/engineering-standards.md`.
_Avoid_: rule, convention, lint rule

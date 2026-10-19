# Botica

The house engineering standards, the `botica` linter that measures them, and the recipes that
install them, the agent config and the agent skills into a repository.

## Language

### Recipes

**Recipe**:
A named package of steps, with no condition, that `botica recipe install` applies and
`botica recipe check` compares with a repository. Built-in recipes live in `catalog/recipes/`; a
project adds its own in `recipes/`. A variant (an agent, a public or a private repository) is a
recipe chosen by name. See `docs/adr/0006-external-rules-and-recipes.md`.
_Avoid_: profile, component, preset

**Step**:
One entry of a recipe: `[[files]]`, `[[sections]]`, `[[links]]`, `[[tools]]`, `[[gitignore]]`,
`[[skills]]`, or the rules it selects. A recipe names no command: `[[tools]]` runs `uv add`,
`[[skills]]` runs `npx skills add`, and the other steps write files.
_Avoid_: op, action, task

**Template**:
A file under the folder of the recipes (`files/`, `sections/`) that a `[[files]]` or
`[[sections]]` step writes. It has no id and no page of its own.
_Avoid_: snippet, asset

**Pending**:
The outcome of a step whose target exists and differs: `install` leaves it as it is and reports
it, and exits with 1.
_Avoid_: conflict, skip, defer

**Unignore**:
The one step that deletes text from a file the repository has: it removes a literal line of
`UNIGNORE_WHITELIST` (`skills-lock.json`) from `.gitignore`, and `install` reports it as
`edited`. The whitelist is a literal list, so every deletion that `install` can make is readable
in the catalog.

### Skills

**Skills manifest**:
`skills-lock.json` — the tracked declaration of which skills a repo uses and where they come
from. **Owned by the third-party `skills` CLI**: botica reads it and never writes it. It
records a `ref` only when the source was pinned, and its `computedHash` is never verified on
restore, so it reproduces which skills from where at what ref — not bytes.
_Avoid_: lockfile, skills lock, skill list

**Derived skills tree**:
`.agents/skills/` — the installed skill files. Rebuilt from the manifest, gitignored, and never
the source of truth.
_Avoid_: installed skills, skills directory

**House baseline**:
The skill set that the `[[skills]]` steps of the recipe `base` install. A starting point, not a
set the repo is held to: the manifest records what the repo uses.
_Avoid_: default skills, curated catalog, required skills

**Authored skill**:
A skill whose source of truth is this repo's `skills/` tree. Declared with `sourceType: local` so
restore reads the working copy instead of clobbering it with the published one.
_Avoid_: local skill, own skill

### Standards

**CES**:
Collective Engineering Standard — one house rule, written in one file of `standards/CES/`. Cited
as `CES-<number>`; its machine id is a kebab-case slug. See `docs/engineering-standards.md`.
Documentation and the unit of proposals, not something a tool enforces: botica evaluates
**rules**, and a rule may cite the CES it implements. See `docs/adr/0002-ces-lifecycle-catalog-and-site.md`.
_Avoid_: rule, convention, lint rule

### Botica

**Botica**:
The Rust package published on PyPI as `botica` (wheels with the binary and the Python module `botica.sdk`), built from the Rust workspace at the root of this repository. `lint` is
its first subcommand, and `botica recipe` replaced the removed `scaffolding` package, so no
pipeline of its own is specific to `lint`. See `docs/adr/0003-botica-lint-architecture.md`.
_Avoid_: botica lint (for the package)

**Rule**:
What botica evaluates, selects, ignores and suppresses. Identified by its slug, and by a **code**.
Lives in `catalog/rules/<family>/<slug>/`, belongs to one **family** and can have one implementation
per language.
_Avoid_: CES, check, standard

**Family**:
A group of rules that share a code prefix (`STRUC`, `ARCH`, `TEST`, …). Its folder holds a
`family.toml` with the code. The linter's own rules have no family and live in `rules/botica/`.
_Avoid_: category, group

**Code**:
A rule's identifier with the `BT` header and its family, then three digits (`BTSTRUC001`; the
linter's own are `BT002`). The only identifier `# noqa` takes.
_Avoid_: rule id (the id is the slug)

**Suppression channel**:
A comment syntax that silences a rule on the line of its finding: `# botica: ignore[<id>]` in any
language, or `# noqa: <code>` in Python. A finding about a whole file takes only
`# botica: ignore-file[<id>]`, and so does a judgment rule, with a reason after `--`. A rule of
project scope takes no comment: the configuration silences it.
_Avoid_: ignore, disable

**Code dirs**:
Where a project's code lives, per ecosystem. The layout rules read it, detected from the manifest
or set with `code-dirs`.
_Avoid_: source root, package dir

**Mirror**:
`collectiveai-team/botica-pre-commit`, the repo prek installs the `botica` hook from. Its tag pins
the wheel of the same version, so a repository needs no `botica` on its PATH.
_Avoid_: hook repo

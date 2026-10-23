---
title: Recipes
---

# Recipes

```bash
botica recipe list             # the recipes of the binary and of recipes/, with what each extends and lists
botica recipe show base-python # a recipe with the rules and the exports of the recipes it extends
botica recipe check base base-python   # how the project differs from the recipes
botica recipe install base base-python # adds what the repository lacks
botica recipe install ci-node --set cooldown-days=14  # with the value of a parameter
```

## A recipe

A recipe is a list of the rules and the exports that a repository of some kind has. It is a file
`<name>.toml`, and every key is optional:

```toml
# recipes/base-python.toml
description = "A Python project"
extends = ["base"]
rules = ["no-utils", "log-no-print"]
judgment = ["test-through-interface"]
exports = ["agents-md", "abide"]
```

`extends` names the recipes whose lists come first. `rules` lists the rules that are not judgment
rules and `judgment` lists the judgment rules, each by slug, code or alias. `exports` names
`agents-md` or `abide`, the formats of `botica rules export`. `agents` names the agents that the
skills are installed for: `opencode`, `claude-code` or `codex`. A list names an entry once, and
recipes that extend one another in a circle are an error.

## Steps

A recipe also declares steps that prepare the repository:

```toml
[[skills]]                  # skills to install with `npx skills add <source> --skill <name>`
source = "mattpocock/skills#v1.2.3"
names = ["tdd", "grilling"]

[[files]]                   # a file to add from a template
path = "prek.toml"
template = "files/prek-base.toml"

[[sections]]                # a section of a file, which starts with the marker
path = "AGENTS.md"
marker = "## Botica rules"
template = "sections/botica-rules.md"

[[links]]                   # a symbolic link to add
path = "CLAUDE.md"
target = "AGENTS.md"

[[tools]]                   # packages to add to pyproject.toml with `uv add --dev`
manager = "uv"
packages = ["ruff", "pytest"]

[[gitignore]]               # lines to add to .gitignore, and lines of the whitelist to remove
lines = [".env", ".tmp/"]
unignore = ["skills-lock.json"]
```

A `template` is a file of the folder of the recipes: under `files/` for a `[[files]]` step and under
`sections/` for a `[[sections]]` step, at any depth, and a recipe does not load when the template of
one of its steps is missing. The recipes of a folder share its templates, and the templates of the
built-in recipes are in `catalog/recipes/files/` and `catalog/recipes/sections/`. A recipe of a project
finds only the templates of its own `recipes/`.

A `path` is relative to the root of the project, without `.` or `..`. A recipe names a skill once, a
file path once and a marker of a path once. The steps of the recipes it extends come first, a step
that two recipes list the same stays where it first is, and a skill that two recipes take from
different sources is taken over by the later recipe, with a warning on stderr. `list` and `show`
print the steps, `check` compares them with the project, and `install` writes the selection, then
adds the files, the sections, the links, the lines of `.gitignore`, the tools and the skills.

The selection is the `rules` and `judgment` lists of the recipes. In a project with no configuration
of botica, `install` writes them to the `select` of a new `botica.toml` (the line `selection
botica.toml: added`); `botica lint`, the hooks of `prek.toml` and the editor read that list, and the
hooks pass no `--select`. The new `botica.toml` lists the names of the recipes in `installed` of
`[recipes]`. A `botica.toml` or a `[tool.botica]` of `pyproject.toml` that exists is left as it is:
`ok` when its `select` lists every rule, `pending` with the rules it lacks otherwise; `installed` is
added to it by hand.

A `[[links]]` step has the `path` of the link and its `target`, which is the text of the link:
relative to the folder of the link, so `../.agents/skills` for `.claude/skills`. A target is not
empty, does not start with `/` and has no empty part or `\`. A recipe names a link path once.

A `[[tools]]` step has a `manager` (`uv`), the packages to add, and optionally a `group`, `dev`
when it is absent. A package is a name that starts with a letter or a digit and has letters,
digits, `.`, `_` and `-`, and may go on with extras, version specifiers and a marker:
`blockbuster>=1.5,<1.6` or `pytest-memray ; sys_platform != 'win32'`. A URL (`name @ url`) is not
accepted. A recipe names a package once, and `My_Pkg` and `my-pkg>=1` are the same package. A
group is a name. Two recipes of a chain, or two recipes named together, that ask for the same
package ask for it with the same requirement, or `show`, `check` and `install` fail with 2 before
anything is written: `ruff` and `ruff>=0.14` are different requirements. `install` passes the
requirement to `uv add` as it is written; `check` and `install` count a package as present when
`pyproject.toml` declares its name, with any versions or marker.

A `[[gitignore]]` step has `lines` and `unignore`, and needs one of them. A line is one line of text
that is not empty, and a recipe lists it once, across its steps. `unignore` names lines to remove
from `.gitignore`, and the only line it accepts is `skills-lock.json` ([ADR 0006](https://github.com/collectiveai-team/botica/blob/main/docs/adr/0006-external-rules-and-recipes.md)); `lines` does not
accept it (CES-107).

## Parameters

A recipe declares the parameters that its templates write:

```toml
[params.cooldown-days]
description = "The days that a new release of a dependency waits before a package manager or dependabot takes it"
kind = "days"
default = 7
```

A template writes the value as `{{ param.cooldown-days }}`. The name is kebab-case, `description` is
one line for `show`, and `choices`, when present, lists the values it may take, the default among
them. `kind` is `text` when absent, or one of two kinds whose views spare the template any
arithmetic or condition:

| Kind | Value | Views |
| --- | --- | --- |
| `text` | one line | none |
| `days` | a whole number | `{{ param.<name>.minutes }}`, `{{ param.<name>.seconds }}` |
| `version` | a version or a range (`1.30.1`, `5`), or empty | `{{ param.<name>.at }}`: `@<value>`, or nothing when the value is empty |

So a template that pnpm reads writes `minimumReleaseAge: {{ param.cooldown-days.minutes }}`, and
one that runs a tool with `uvx` writes `uvx zizmor{{ param.zizmor-version.at }}`: an empty version
runs the tool unpinned, as `uvx zizmor`. A version written without `.at` needs a value, and an
empty one fails. Only `{{ param.` is replaced, so the `${{ github.ref }}` of a workflow stays.

The value comes from `--set <name>=<value>` of `install`, else from the configuration, else from
the default:

```toml
[recipes.params]
cooldown-days = 14
```

Two recipes of a chain may declare a name the same, and not differently. A template that writes a
name that no recipe of its chain declares, a `--set` that none of the named recipes declares, a
name of `[recipes.params]` that no recipe at all declares, and a value that does not fit its kind
or its choices fail the command with 2 before anything is written. A name of `[recipes.params]`
that another recipe declares is left for that recipe. A parameter changes a value inside a
template, never whether a step runs.

Every chain also has `botica-version`, which botica declares and a recipe cannot: its default is
the release of the binary that runs `install`, so the `rev` of the mirror `botica-pre-commit` and
each `uvx botica@` of the templates ask for the rules of that release. Each tool that a built-in
template runs with `uvx` or `npx` reads its version from a parameter, whose default is what the
template ran before:

| Parameter | Recipe | Template | Default |
| --- | --- | --- | --- |
| `botica-version` | botica | `prek.toml`, `abide.yml`, `conventional-commits.yml`, `commit-policy.yml` | the release of the binary |
| `jscpd-version` | `base` | `prek.toml` | `5` |
| `abide-version` | `base` | `abide.yml` | `0.0.11` |
| `zizmor-version` | `ci` | `zizmor.yml` | `1.30.1` |
| `prek-version` | `ci-python` | `tests.yml` | empty (`uvx prek`) |
| `pyrefly-version` | `base-python` | `prek.toml` | empty (`uvx pyrefly`) |
| `deptry-version` | `base-python` | `prek.toml` | empty (`uvx deptry`) |

A hook with a `rev` has no parameter: `prek update` moves it.

## Built-in recipes

The built-in recipes are in `catalog/recipes/` and are compiled into the binary; `botica recipe
list` prints each with its description, and `botica recipe show <name>` its steps. `base` also adds
the section `## Repo Workspace Defaults` to `AGENTS.md`, before `## Botica rules`, whose block
`install` fills with the judgment rules (the exports `agents-md` and `abide`); a new `AGENTS.md`
starts with that section. An agent is a recipe chosen by name, beside the others
(`botica recipe install base base-python agent-claude-code`). `agent-claude-code` names no agent
for the skills: `npx skills add --agent claude-code` writes in `.claude/skills/<name>`, which the
link `.claude/skills` makes a path inside `.agents/skills`, and fails with `ENOTDIR`.

`full-python` is the whole package of a Python project: it extends `base-python` and `ci-python`
and has no step of its own, so `botica recipe install full-python` does what `botica recipe install
base-python ci-python` does. Each language that has a `base-*` and a `ci-*` has its `full-*`. The
agent and the visibility of the repository stay apart: `botica recipe install full-python
agent-claude-code repo-public`. So does `ci-publish`, which only a library that publishes to PyPI
needs: `botica recipe install full-python ci-publish`.

`secrets-varlock` installs the skill `varlock` of `dmno-dev/varlock` and has no other step. A
person or an agent runs `varlock init --agent`, which writes a draft of `.env.schema` from
`.env.example`, from the variables that the code reads, or with one example variable; the skill
asks to review the draft with the user. `varlock init` keeps an `.env.schema` that exists and then
drafts nothing, so the recipe has no template of it. `base` ignores `.env` and keeps
`.env.schema` in git.

What the recipes cannot say about themselves:

- A recipe has no condition, so `ci-docker` and `repo-public` add `docker.yml` also to a
  repository that has no `Dockerfile`: name them only when it has one, and `ci-python`, `ci-node`
  or `ci-go` only for a project of that ecosystem. The three add the same `.github/dependabot.yml`
  and `.github/workflows/osv-scanner.yml`, so a repository names one of them.
- `repo-private` leaves out the dependency review, which needs a paid plan on a private
  repository, and `docker.yml`, which pushes to GHCR and bills it. `ci-automerge-public` needs the
  setting "Allow auto-merge" and a branch protection rule with required checks; without a required
  check the merge is immediate. `ci-automerge-private` is for a repository with no branch protection
  (a private one on the Free plan).
- The template of `pyproject.toml` of `base-python` has `readme = "README.md"`, so `uv add` fails
  to build a project that has no `README.md`.

## Recipes of a project

A project adds its own recipes in `recipes/` at its root, and a recipe of the project may extend a
built-in one. A recipe of the project with the id of a built-in one does not load, unless the configuration
lists the id:

```toml
[recipes]
replace = ["base-python"]
```

The recipes that a project uses are the ones that `installed` of `[recipes]` names, the ones of
`recipes/` and the recipes that these extend. The rule `recipe-conflict` (preview) resolves them
together in `botica lint --scope project` and reports two of them that ask for one package of a
`[[tools]]` step with different requirements, before `install` fails on them:

```toml
[recipes]
installed = ["base-python", "team"]
```

## List and show

`botica recipe list` prints each recipe with what it extends and how many rules, judgment rules and
exports its file lists, and where it comes from; a recipe with steps has a line with the number of
steps of each kind that it has: skills, files, sections, links, gitignore (the lines and the
unignored lines), tools and params. `botica recipe show <name>` prints the recipe with the lists of what it extends first, each
rule with its code, its summary and the recipe that lists it, and then the sections Skills, Files,
Sections, Links, Gitignore, Unignore, Tools and Parameters for the steps it has, and a section
Agents first when the recipe lists agents. A parameter shows its value, which is the one of
`[recipes.params]` or the default.

## Check

`botica recipe check <name>...` takes one or more names, as `install` does, and checks them as one
recipe: each step once, the warnings of the resolution on stderr and the names in the last line. It
takes the options of `botica lint` that select rules (`--select`, `--ignore`, `--config`). It prints
one line for each rule, fragment, export, file, section, link, line of `.gitignore`, unignored line,
tool and skill of the recipes, in that order, rules first, then judgment rules, fragments, exports,
files, sections, links, `gitignore`, `unignore`, tools and skills, and last one `conflict` line for
each package that the recipes and the ones the project uses (`installed` and `recipes/`) ask for
with different requirements, as `recipe-conflict` reports it:

```text
rule file-size-guard: ok
rule no-notebooks: differs: not selected
rule acme/deptry: ok
fragment prek.toml `acme/deptry`: differs: prek.toml lacks the configuration of deptry that `acme/deptry` needs: `botica explain acme/deptry` shows it
export agents-md: differs: cannot read `AGENTS.md`
file prek.toml: differs: does not exist
section AGENTS.md `## Botica rules`: differs: marker not in the file
link CLAUDE.md: differs: does not exist
gitignore .tmp/: differs: not in .gitignore
unignore skills-lock.json: differs: .gitignore still has the line
tool ruff: differs: not in pyproject.toml
skill tdd: differs: not in `skills-lock.json` and not in `.agents/skills/`
conflict ruff: differs: The recipes `team` and `base` ask for the package `ruff` as `ruff>=0.15` and as `ruff`: a package has one requirement across the recipes, and `botica recipe install` stops on them
base: 13 checked, 11 differ
```

A rule differs when it is not in the catalog, is not active (the reason is the one that `botica
rules --all` gives), is a judgment rule listed under `rules` or is not one listed under
`judgment`. A fragment is a line for each configuration that an external implementation of a rule
under `rules` declares; it differs when the
settings of its tool do not exist, cannot be read or lack the fragment, as `external-config` decides, and
a rule that is not active has its fragment checked too. A rule that is not in the catalog has none.
An export differs when `botica rules export --check` fails for it with its default file:
`AGENTS.md` for `agents-md` and `.abide/rubric.json` for `abide`. A file differs when its path is
not a file of the project. A section differs when its path is not a file, cannot be read, or its
text does not contain the marker. A link differs when its path does not exist, is not a symbolic
link, or is a link whose text is not the target. A tool differs when `pyproject.toml` does not exist or does not declare the package. It declares it
in `project.dependencies`, `project.optional-dependencies`, `dependency-groups` or
`tool.uv.dev-dependencies`, under any group, and the names are compared as `uv` does. A line of `gitignore` differs when `.gitignore` does not exist, is
not a file, cannot be read, or has no line equal to it once the white space at its end is dropped. An
`unignore` line differs when `.gitignore` is not a file, cannot be read, or has a line equal to it
between white space; without a `.gitignore` it is `ok`. A skill differs when its name is not a key of `skills`
in `skills-lock.json` or `.agents/skills/<name>/SKILL.md` is not a file. The paths of files and
sections are from the root of the project, wherever the command runs. `check` does not read the
value of a parameter back from a file: a file that `install` wrote belongs to the repository, and a
value that a standard requires is the fragment of an external rule. The command exits with 1 when
something differs and with 2 when the recipe does not exist, the recipes do not load or a value of
`[recipes.params]` does not fit its parameter.

## Install

`botica recipe install <name>...` adds the files, then the sections, then the links, then the lines
of `.gitignore`, then the tools, then the skills of one or more recipes, from the root of the project, and prints
one line for each:

```text
file prek.toml: added
file other.toml: pending: exists and differs from the template `files/other.toml` of `base`
section AGENTS.md `## Botica rules`: added: appended to the file
link CLAUDE.md: added
gitignore .tmp/: added
unignore skills-lock.json: edited: removed the line from .gitignore
tool ruff: added
skill tdd: added
skill grilling: ok
base: 6 added, 1 ok, 1 pending, 1 edited
```

A file is written from its template when its path does not exist (`added`). A path that exists is
`ok` when it has the text of the template and `pending` otherwise; `install` does not change it.
The text of a template of a file or a section is the one with the values of the parameters. A
section is written to a new file, appended after a blank line to a file that has no marker (`added`),
and left alone when the file has the marker (`ok`); a path that is not a file or cannot be read is
`pending`. A path that ends in `.toml` gets the section only when the file is valid TOML before and
after: a file that is not, or a section that would define a table twice, leaves the step `pending`
and the file as it is. Running the command again changes nothing. A step that two of the named recipes share is
done once, and a name that is not a recipe fails the command with 2 before anything is written.

Each value of `--set` has a line `param <name>`. In a project with no configuration of botica, the
values go to `[recipes.params]` of the new `botica.toml` (`added`). A configuration that has the
value is `ok`, and one that lacks it or has another is `pending`: the run used the value of
`--set`, and the next run without it uses the configuration again.

A link is created, with the folders of its path, when its path does not exist (`added`), also when
its target does not exist. A path that holds a link with the text of the target is `ok`, and a path
that holds anything else is `pending`; `install` does not replace it. A folder that cannot hold the
link stops the command with 2.

The lines of `gitignore` are written to a `.gitignore` that does not exist, and the ones that a
`.gitignore` lacks are appended to it, after a line break when its last line has none (`added`). A
line that is there is `ok`, and a `.gitignore` that is not a file or cannot be read leaves each line
`pending`. An `unignore` line is removed from `.gitignore` wherever it is a whole line between white
space, and the rest of the file keeps its bytes (`edited`, counted apart in the summary when there
is one); without that line it is `ok`. `install` does not tell that a pattern such as `*.json`
ignores the manifest.

A package that `pyproject.toml` declares is `ok`. For the ones that it does not, `install` runs
`uv add --dev <package>...` (`--group <group>` for another group) in the root of the project, once
for each group, so `uv` has to be on the `PATH`. A package that is declared afterwards is `added`.
A project without `pyproject.toml` leaves each package `pending`, and `install` does not create
one. A call that fails for several packages is made again for each package, because `uv add` adds
none when one of them does not exist; the package that is still not declared is `failed`, with the
reason, as for the skills. The reason is the last line of stderr that starts with `cause:`, or its
last line, leaving out the lines that start with `hint:`.

A skill is `ok` when `.agents/skills/<name>/SKILL.md` is a file. For the skills that are not,
`install` runs `npx skills add <source> --agent <agents> --yes --json --skill <name>...` in the root of the
project, once for each source, so `npx` has to be on the `PATH`. The agents are the ones that the
named recipes list, each once and in the order they come, and `opencode` when none lists one. A skill that is there afterwards is
`added`. A skill that is not, because the call failed, could not run, or did not install it, is
`failed`, with the reason, and the summary ends with the number of failures:

```text
skill tdd: failed: `npx skills add mattpocock/skills#v1.2.3` exited with code 1: Failed to clone repository: fatal: Remote branch v1.2.3 not found in upstream origin
base: 0 added, 0 ok, 0 pending, 1 failed
```

`--json` makes `npx` write a JSON array to stdout. A skill that the array lists as `skipped` is
`failed` with the reason of the entry (`` `npx skills add mattpocock/skills` skipped it: No matching
skill found in source ``). When the call fails, the reason is the first line of the `error` of the
first `failed` entry followed by the last `fatal:` or `error:` line below it. Without such an entry,
the reason is the last line of stderr that starts with `cause:`, or its last line, without ANSI
sequences and without the lines that start with `hint:`.

The other sources still run, and the files and the sections are already written. `skills-lock.json`
is written by the CLI of skills. The command exits with 1 when a step is pending or a skill
failed, and with 2 for an error that stops it.

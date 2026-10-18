# botica lint

A deterministic linter for the house engineering standards. It never calls a model or the
network, reads your files once, and runs its rules in parallel. The design is in
[ADR 0003](../docs/adr/0003-botica-lint-architecture.md). The catalog under `rules/` is the only
documentation of the rules it measures: each rule has a README, and `botica rules` lists them. The
CES are catalogued in [`docs/engineering-standards.md`](../docs/engineering-standards.md).

## Install

- **From a wheel**, published on PyPI since 0.7.0 (see
  [`docs/botica-release.md`](../docs/botica-release.md)): `uv tool install botica` or
  `uvx botica lint`. The wheel holds only the binary, so it needs no Python at run time.
- **From source**: `cargo build --release -p botica-cli` in this directory; the binary is
  `target/release/botica`.

## Use

```bash
botica lint                    # the current directory
botica lint src tests/a.py     # directories to walk, or files (as prek passes them)
botica lint --select file-size-guard,no-utils
botica lint --output-format json   # text (default), json, sarif or github
botica lint --judged judge.sarif   # the findings of a judge, filtered by the configuration and comments
botica rules                   # which rules are active, and why (--all adds the inactive ones)
botica server                  # the Language Server Protocol on stdio: the findings for the buffers of an
                               # editor (native and pattern rules), paths from the working directory
botica server --extensions     # and the extension rules of the project: heavy, and it runs the scripts
                               # of the project (see "Rules of the project")
botica explain no-notebooks    # what a rule is and how it is configured here
botica rules export --format markdown --output site   # one page per rule of the catalog, and an index;
                               # the catalogs of .botica/rules get a folder and an index section each
botica rules export --format json --output out       # the whole catalog in out/catalog.json
botica rules export --format agents-md --output AGENTS.md [--check]
                               # the active judgment rules, between <!-- botica:rules --> and <!-- /botica:rules -->
botica rules export --format abide [--check]   # the rubric of abide, .abide/rubric.json, with the
                               # judgment rules of the block of AGENTS.md (--agents-md names another file)
botica refs list               # the topics (built-in and of references/), their references and who cites them
botica refs show log-no-print  # the references of a topic, or of the topics that a rule or a standard cites
botica refs check [--all]      # fails when a reference no longer resolves or a citation names no topic
botica refs export             # every topic as JSON, with the text of each file reference (the site reads it)
botica refs pin <permalink> --topic <topic> --license MIT --note "…"
                               # adds the file of a permalink of GitHub to a topic of references/
botica refs update <topic> [--write]
                               # follows the anchor of each external reference to the head of its repository
botica recipe list             # the recipes of the binary and of recipes/, with what each extends and lists
botica recipe show base-python # a recipe with the rules and the exports of the recipes it extends
botica recipe check base base-python # how the project differs from the recipes: rules, fragments, exports, files, sections, skills
botica recipe install base base-python # adds the files and sections of the recipes that the repository lacks
botica rules test --judge abide [test-through-interface]
                               # abide judges the fixtures of each judgment rule; compared with expected.json
botica rules conform lint/rules   # each rule over its fixtures, with and without each optimisation;
                               # .botica/rules/<name> for a catalog of a project (it has a catalog.toml)
botica rules new structure/no-foo --kind ast-grep --language python --standard CES-63
                               # the folder of a new rule (ast-grep, judgment or external) in lint/rules
botica rules new structure/no-blobs --kind extension --runtime python --language python \
  --catalog .botica/rules/acme   # a rule of a catalog of a project: id acme/no-blobs, code ACMESTRUC001
botica rules new structure/dep-hygiene --kind external --language python --tool deptry \
  --docs https://deptry.com/ --config prek.toml   # an external rule and its fragment config/prek.toml
```

Exit code: `0` clean (a warning does not fail the run), `1` a finding of severity error, `2`
the run itself failed (it wins over `1`). `--exit-zero` forgives only the findings.

## Rules

| Code | Rule | Standard | Looks at |
|---|---|---|---|
| `BTSTRUC001` | [`file-size-guard`](rules/structure/file-size-guard/README.md) | CES-71 | every file: warns at 400 lines, errors at 700 |
| `BTSTRUC002` | [`no-utils`](rules/structure/no-utils/README.md) | CES-63 | Python: `utils.py`, `helpers.py`, … |
| `BTSTRUC003` | [`no-artifacts-in-package`](rules/structure/no-artifacts-in-package/README.md) | CES-32 | data, models and reports inside the code |
| `BTSTRUC004` | [`no-notebooks`](rules/structure/no-notebooks/README.md) | CES-32 | notebooks inside the code |
| `BTSTRUC005` | [`no-tests-in-package`](rules/structure/no-tests-in-package/README.md) | none yet | Python: tests inside the package |
| `BTSTRUC006` | [`cognitive-complexity`](rules/structure/cognitive-complexity/README.md) | CES-110 | external: complexipy measures it, through its hook in `prek.toml` |
| `BTARCH001` | [`arch-database-package`](rules/architecture/arch-database-package/README.md) | CES-18 | Python: tables and engines outside a `database` package |
| `BTARCH002` | [`settings-module`](rules/architecture/settings-module/README.md) | CES-76 | Python: `os.getenv` and `os.environ` outside the settings module |
| `BTARCH003` | [`settings-base-class`](rules/architecture/settings-base-class/README.md) | none yet | Python: a settings class that extends `BaseModel` instead of `BaseSettings` |
| `BTARCH004` | [`settings-direct-instantiation`](rules/architecture/settings-direct-instantiation/README.md) | none yet | Python: a settings class built where it is used, not through `get_settings()` |
| `BTARCH005` | [`db-no-create-all`](rules/architecture/db-no-create-all/README.md) | none yet | Python: `metadata.create_all()` outside tests |
| `BTDM002` | [`no-dict-alias`](rules/data-models/no-dict-alias/README.md) | CES-79 | Python: a type alias for a raw `dict` |
| `BTDM003` | [`no-dict-call-return`](rules/data-models/no-dict-call-return/README.md) | CES-79 | Python: `return dict(...)` |
| `BTDM004` | [`no-dict-literal-return`](rules/data-models/no-dict-literal-return/README.md) | CES-79 | Python: returning a dict literal |
| `BTDM005` | [`no-dict-return-annotation`](rules/data-models/no-dict-return-annotation/README.md) | CES-79 | Python: a function annotated to return `dict` |
| `BTFAST001` | [`api-schemas-extra-forbid`](rules/fastapi/api-schemas-extra-forbid/README.md) | CES-4 | Python: API schemas without `extra="forbid"` |
| `BTFAST002` | [`api-schema-in-route-module`](rules/fastapi/api-schema-in-route-module/README.md) | none yet | Python: a Pydantic schema defined in a route module |
| `BTFAST003` | [`api-request-schema-name`](rules/fastapi/api-request-schema-name/README.md) | none yet | Python: a request schema not named `{Name}Request` |
| `BTFAST004` | [`api-response-schema-name`](rules/fastapi/api-response-schema-name/README.md) | none yet | Python: a response schema not named `{Name}Response` |
| `BTLOG001` | [`log-get-logger`](rules/logging/log-get-logger/README.md) | CES-45 | Python: `logging.getLogger` called directly |
| `BTLOG002` | [`log-no-print`](rules/logging/log-no-print/README.md) | CES-46 | Python and TypeScript: `print()` and `console.log()` in library code |
| `BTPROC001` | [`cli-typed-framework`](rules/process/cli-typed-framework/README.md) | CES-67 | Python: `argparse`, `click` and `sys.argv` |
| `BT001`–`BT003` | `io-error`, `unused-suppression`, `invalid-suppression` | | the run and the suppressions |
| `BT004` | [`external-config`](rules/botica/external-config/README.md) | | the files of configuration that the active external rules name |

`BTARCH003`–`BTARCH005`, `BTFAST002`–`BTFAST004` and `BTSTRUC006` have the status `preview` and
are off by default: they run only with `preview = true` and when you select them.

Another tool enforces an external rule (`kind = "external"`) through its configuration. The rule
names the tool, the address of its documentation and, in `config/<file>.toml` of its folder,
the fragment of configuration that goes into `<file>.toml` at the root of the project.
`external-config` reports that file, on every run, when an active external rule finds its fragment
missing.

The rules written in ast-grep's YAML run too, unless you pass `--no-astgrep`. Each rule has a
README with its directive, its reasons and how to silence it.

## References

A topic points at code and documentation to read. It is a folder with a `topic.toml` and a
`README.md` that says what to read in each reference and what to adapt. The topics that the built-in
rules cite are in `lint/references/` and are compiled into the binary; a project adds its own in
`references/` at its root, which botica reads from disk.

```toml
# lint/references/structured-logging/topic.toml
title = "Structured logging with structlog"

[[references]]
repo = "github.com/collectiveai-team/botica"
rev = "9c225ad2537550a67016e878f662fe7f93d1b3b7"   # the full SHA of a commit
path = "scaffolding/templates/snippets/core/logger.py"
lines = "72-75"                       # optional: a range or one line; without it, the whole file
anchor = "def get_logger"             # optional: text that the lines hold
license = "MIT"
note = "the function that every module imports"

[[references]]
url = "https://www.structlog.org/en/stable/getting-started.html#building-a-context"
note = "binding context: `bind()` returns a new logger that carries the context"
quote = "Each logger with its context is immutable."   # optional: the passage, as the page words it
```

A reference points at a fragment. It has a `path` or a `url`. A `url` is a page of documentation:
it ends with the `#fragment` of a section, or has a `quote` of the passage, and `note` says why. A
reference to a whole file or a whole page loads, and `botica refs check` warns about it. A `path`
is a file: of the project when the reference has no `repo`, `rev` and `license`, and of another
repository at a commit when it has the three. `rev` is a full commit SHA, and `repo`
is `<host>/<path>` with no scheme.

A rule cites topics with `references = ["structured-logging"]` in its `rule.toml`, and a standard
with `references: ["structured-logging"]` in its front matter. The site lists the topics at the end
of the page of each rule and standard, and has a page per topic with the lines of each file
reference, so a reader does not leave the page to understand it. The build of the site runs
`botica refs export`, which fetches the external files.

`botica refs show` reads the file of an external reference with the `git` of the user: private
repositories open with the credentials that git has, and a missing credential is an error. It runs
`git fetch --depth 1 --filter=blob:none` for the commit, then `git ls-tree` and `git show`, and each
command is killed after 60 seconds (`BOTICA_REFS_TIMEOUT` sets the seconds). The file is kept in a
cache as `<cache>/<repo>/<rev>/<path>` and read from there on the next run, with no git and no
network. The cache is `--cache-dir`, else `botica/refs` under `XDG_CACHE_HOME`, `LOCALAPPDATA`
(Windows) or `~/.cache`. For a file of GitHub, `show` prints the license and the permalink of the
lines.

`botica refs check` exits with 1 when a file of a reference cannot be read, when its `lines` are past
the end of the file, when its `anchor` is not in them, when a rule or a standard cites a topic that
does not exist, or when a `topic.toml` does not load. It reads the files of the topics of the
project only: the references of the built-in topics point at botica, and `check --all` fetches those
too. The job `test` of `ci.yml` runs `check --all`. A reference to a page is not fetched.

`botica refs pin <permalink> --topic <topic> --license <spdx> --note <text>` adds a reference to
`references/<topic>/topic.toml`. The permalink is `https://github.com/<owner>/<repo>/blob/<commit>/<path>`
with `#L10-L40` or `#L10` for the lines. Pin reads the file at the commit, checks the lines, writes
the first line that has text, without its indentation, as the `anchor`, and appends the block to the
file. Without lines the reference points at the whole file and pin prints the advice that `check`
gives. `--title` creates the topic when it does not exist, with a `README.md` that holds the title.
Pin fails and writes nothing when the address is not a permalink of GitHub, the commit is not a full
SHA, the lines are past the end of the file or the topic is a built-in one.

`botica refs update <topic>` asks each repository for its `HEAD` (`git ls-remote`) and, for each
reference to a file of another repository, prints the commit and the lines that it proposes:
`references[0] <label>: rev -> <commit>, lines 2-3 -> 4-5`. It looks for the `anchor` in the file at
the head and keeps the block as long as it was, with the anchor on the same line of the block. A
reference with no `lines` takes the new commit only. The line ends with `, the text changed` when
the lines at the head do not show the text they showed. A reference is a problem, and the command
exits with 1, when it has `lines` and no `anchor`, when the anchor is not in the file at the head
or is in more than one place, or when the block would start before the file or end past it. Without
`--write` nothing is written. With `--write` the `rev` and the `lines` of each reference that
resolves are replaced in `references/<topic>/topic.toml` and the rest of the file stays as it was;
a file that does not write `[[references]]` as tables is not rewritten, and the command fails. A
built-in topic is proposed but not written: its `rev` is edited in `lint/references/`. References
to a file of the project and to a page are left out.

A topic of `references/` with the id of a built-in topic does not load, unless the configuration
lists the id:

```toml
[references]
replace = ["structured-logging"]
```

## Recipes

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
marker = "## Engineering Standards"
template = "sections/standards-index.md"

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
built-in recipes are in `lint/recipes/files/` and `lint/recipes/sections/`. A recipe of a project
finds only the templates of its own `recipes/`.

A `path` is relative to the root of the project, without `.` or `..`. A recipe names a skill once, a
file path once and a marker of a path once. The steps of the recipes it extends come first, a step
that two recipes list the same stays where it first is, and a skill that two recipes take from
different sources is taken over by the later recipe, with a warning on stderr. `list` and `show`
print the steps, `check` compares them with the project, and `install` adds the files, the sections,
the links, the lines of `.gitignore`, the tools and the skills.

A `[[links]]` step has the `path` of the link and its `target`, which is the text of the link:
relative to the folder of the link, so `../.agents/skills` for `.claude/skills`. A target is not
empty, does not start with `/` and has no empty part or `\`. A recipe names a link path once.

A `[[tools]]` step has a `manager` (`uv`), the packages to add, and optionally a `group`, `dev`
when it is absent. A package is a name that starts with a letter or a digit and has letters,
digits, `.`, `_` and `-`; a recipe names it once, and `My_Pkg` and `my-pkg` are the same name. A
group has the same form.

A `[[gitignore]]` step has `lines` and `unignore`, and needs one of them. A line is one line of text
that is not empty, and a recipe lists it once, across its steps. `unignore` names lines to remove
from `.gitignore`, and the only line it accepts is `skills-lock.json` (ADR 0001); `lines` does not
accept it (CES-107).

The recipes `base`, `base-python`, `agent-opencode`, `agent-claude-code`, `agent-codex` and the
ones of CI (`ci`, `ci-python`, `ci-automerge-public`, `ci-automerge-private`,
`ci-dependency-review`, `ci-docker`, `ci-publish`, `ci-opencode`, `ci-node`, `ci-go`,
`repo-public` and `repo-private`) are in `lint/recipes/` and are compiled into the binary. `base` also adds the section `## Repo Workspace
Defaults` to `AGENTS.md`, before the standards index; a new `AGENTS.md` has no `# Agent Notes`
heading, which `scaffolding` writes. An agent is a recipe chosen by name, beside the others
(`botica recipe install base base-python agent-claude-code`). `agent-opencode` writes
`opencode.jsonc` and installs the skills for `opencode`; `agent-codex` has no step and installs
them for `codex`; `agent-claude-code` writes `.claude/settings.json` and links `CLAUDE.md` to
`AGENTS.md` and `.claude/skills` to `../.agents/skills`, so Claude Code reads the skills that are
installed once in `.agents/skills`, and it names no agent: `npx skills add --agent claude-code`
writes in `.claude/skills/<name>`, which the link makes a path inside `.agents/skills`, and fails
with `ENOTDIR`.

The recipes of CI have the workflows of `scaffolding` as `[[files]]` steps under `.github/`:
`ci` has `conventional-commits.yml`, `commit-policy.yml`, `zizmor.yml` and the config
`.github/zizmor.yml`; `ci-python` extends it with `tests.yml`, `dependabot.yml` and
`osv-scanner.yml`; `ci-dependency-review`, `ci-docker`, `ci-publish` (`release.yml` and `pypi.yml`)
and `ci-opencode` have one piece each. `repo-public` extends `ci`, `ci-dependency-review` and
`ci-docker`, and `repo-private` extends `ci`: the dependency review needs a paid plan on a private
repository and `docker.yml` pushes to GHCR, which bills a private repository. A recipe has no
condition, so `ci-docker` and `repo-public` add `docker.yml` also to a repository that has no
`Dockerfile`: name them only when it has one, and `ci-python` only for a Python project.
`ci-automerge-public` and `ci-automerge-private` extend `ci` and merge the Dependabot pull requests
that are patch updates or fix an open security alert; neither merges a minor update, and neither is
in `repo-public` or `repo-private`. `ci-automerge-public` has `dependabot-automerge.yml`, which
labels the pull request `automerge` and runs `gh pr merge --auto`: the repository needs the label,
the setting "Allow auto-merge" and a branch protection rule with required checks, or the merge is
immediate. `ci-automerge-private` is for a repository with no branch protection (a private one on
the Free plan): `dependabot-automerge-label.yml` labels, and `dependabot-automerge-run.yml` merges
on `workflow_run` once the workflows that it lists (`Tests`, `osv-scanner`, `zizmor`,
`Conventional Commits` and `commit-policy`) pass. Both run on `pull_request_target` or
`workflow_run` and never check out the code of the pull request; the `zizmor` finding
`dangerous-triggers` is ignored in the line of the trigger. The sources are the workflows of
`coro` and `collectiveai-web`. `ci-opencode` starts only for the comment of an `OWNER`, a `MEMBER`
or a `COLLABORATOR`: with `use_github_token: true` the action skips the token exchange where it
checks the permission of the commenter, and the job has write scopes and secrets.
`ci-node` extends `ci` for a Node project that uses pnpm: `dependabot.yml` (the actions in one pull
request, and npm with the patch and minor updates grouped as development and production and each
major alone) and `osv-scanner.yml` with `--lockfile=pnpm-lock.yaml`. The templates of a recipe for
another ecosystem are in `lint/recipes/files/github-<ecosystem>/`. `ci-go` extends `ci` for a Go
module: `go.yml` (`gofmt` with its diff, `go build`, `go vet` and `go test -race`, on `main` and on
every pull request whatever its base), `dependabot.yml` (the actions and `gomod`) and
`osv-scanner.yml` with `--lockfile=go.mod`. `ci-python`, `ci-node` and `ci-go` add the same
`.github/dependabot.yml` and `.github/workflows/osv-scanner.yml`, so a repository names one of them:
the second `install` leaves the files that exist and reports them as pending.
`base-python` extends `base` with a `pyproject.toml` (a `[[files]]` template with `replace-me` as
name and description, to fill in), the Python hooks of prek (three sections of `prek.toml`, one for
each repository of hooks), the configuration of ruff, pyrefly and pytest (sections of
`pyproject.toml`) and the dev packages (a `[[tools]]` step). The template of `pyproject.toml` has
`readme = "README.md"`, so `uv add` fails to build a project that has no `README.md`. A
project adds its own in `recipes/` at its root, and a recipe of the project may extend a built-in
one. A recipe of the project with the id of a built-in one does not load, unless the configuration
lists the id:

```toml
[recipes]
replace = ["base-python"]
```

`botica recipe list` prints each recipe with what it extends and how many rules, judgment rules and
exports its file lists, and where it comes from; a recipe with steps has a line with the number of
steps of each kind that it has: skills, files, sections, links, gitignore (the lines and the
unignored lines) and tools. `botica recipe show <name>` prints the recipe with the lists of what it extends first, each
rule with its code, its summary and the recipe that lists it, and then the sections Skills, Files,
Sections, Links, Gitignore, Unignore and Tools for the steps it has, and a section Agents first
when the recipe lists agents.

`botica recipe check <name>...` takes one or more names, as `install` does, and checks them as one
recipe: each step once, the warnings of the resolution on stderr and the names in the last line. It
takes the options of `botica lint` that select rules (`--select`, `--ignore`, `--config`). It prints
one line for each rule, fragment, export, file, section, link, line of `.gitignore`, unignored line,
tool and skill of the recipes, in that order, rules first, then judgment rules, fragments, exports,
files, sections, links, `gitignore`, `unignore`, tools and skills:

```text
rule file-size-guard: ok
rule no-notebooks: differs: not selected
rule cognitive-complexity: ok
fragment prek.toml `cognitive-complexity`: differs: prek.toml lacks the configuration of complexipy that `cognitive-complexity` (CES-110) needs: `botica explain cognitive-complexity` shows it
export agents-md: differs: cannot read `AGENTS.md`
file prek.toml: differs: does not exist
section AGENTS.md `## Engineering Standards`: differs: marker not in the file
link CLAUDE.md: differs: does not exist
gitignore .tmp/: differs: not in .gitignore
unignore skills-lock.json: differs: .gitignore still has the line
tool ruff: differs: not in pyproject.toml
skill tdd: differs: not in `skills-lock.json` and not in `.agents/skills/`
base: 12 checked, 10 differ
```

A rule differs when it is not in the catalog, is not active (the reason is the one that `botica
rules --all` gives), is a judgment rule listed under `rules` or is not one listed under
`judgment`. A fragment is a line for each configuration that an external implementation of a rule
under `rules` declares (the hook of prek of `cognitive-complexity`, for one); it differs when the
file it names does not exist, is not TOML or lacks the fragment, as `external-config` decides, and
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
sections are from the root of the project, wherever the command runs. The command exits with 1 when
something differs and with 2 when the recipe does not exist or the recipes do not load.

`botica recipe install <name>...` adds the files, then the sections, then the links, then the lines
of `.gitignore`, then the tools, then the skills of one or more recipes, from the root of the project, and prints
one line for each:

```text
file prek.toml: added
file other.toml: pending: exists and differs from the template `files/other.toml` of `base`
section AGENTS.md `## Engineering Standards`: added: appended to the file
link CLAUDE.md: added
gitignore .tmp/: added
unignore skills-lock.json: edited: removed the line from .gitignore
tool ruff: added
skill tdd: added
skill grilling: ok
base: 6 added, 1 ok, 1 pending, 1 edited
```

A file is written from its template when its path does not exist (`added`). A path that exists is
`ok` when it has the text of the template and `pending` otherwise; `install` does not change it. A
section is written to a new file, appended after a blank line to a file that has no marker (`added`),
and left alone when the file has the marker (`ok`); a path that is not a file or cannot be read is
`pending`. A path that ends in `.toml` gets the section only when the file is valid TOML before and
after: a file that is not, or a section that would define a table twice, leaves the step `pending`
and the file as it is. Running the command again changes nothing. A step that two of the named recipes share is
done once, and a name that is not a recipe fails the command with 2 before anything is written.

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

## Which files

The walk honours `.gitignore` and enters hidden directories (`.github/`…) except the directories
of tools, caches and agent configuration (`.git`, `.venv`, `.mypy_cache`, `.botica`, `.agents`,
`.claude`…; the list is `TOOL_DIRS` in `crates/botica-core/src/walk/mod.rs`). A path named on the
command line is linted even if it is on that list or excluded, unless you pass `--force-exclude`,
which leaves out what `extend-exclude` names and what sits in a directory of a tool (unless
`extend-include` asks for it). The `botica` hook of the mirror passes it, because prek names every
file.

- `extend-exclude = ["vendor/"]` skips more (gitignore globs); it wins over everything.
- `extend-include = ["generated/api.py"]` brings back what `.gitignore` or `TOOL_DIRS` leave out.

## Configuration

`botica.toml`, or `[tool.botica]` in `pyproject.toml`, found by walking up from the working
directory. Defining both is an error, and so is an unknown key or rule. The JSON Schema is
[`schema/botica.schema.json`](schema/botica.schema.json).

```toml
# botica.toml
extend-exclude = ["vendor/"]
ignore = ["no-tests-in-package"]

[rules.file-size-guard]
warn = 500
max = 900
```

Keys: `select`, `extend-select`, `ignore`, `preview`, `languages`, `extend-exclude`,
`extend-include`, `code-dirs`, `rules` (settings per rule), `overrides`, `external`, `replace`,
`rule-dirs`, `catalogs`, `compat-ast-grep-ignore`. Any of them can be given for one run with
`--config 'key = value'`.

`code-dirs` says where the code of the project lives (`python`, `js`, `rust`, `go`, `jvm`), for
the layout rules. Without it they are detected from `pyproject.toml`, `package.json`,
`Cargo.toml`, `go.mod` and the like; `botica explain no-notebooks` lists what was found.

## Rules of the project

A project adds rules in `.botica/rules/<catalog>/`, under the folder of `botica.toml`. A catalog has
the layout of `lint/rules/` with a `catalog.toml` on top:

```text
.botica/rules/acme/
  catalog.toml           # code = "ACME"
  structure/
    family.toml          # code = "STRUC", description = "…"
    no-blobs/
      rule.toml          # id = "acme/no-blobs", code = "ACMESTRUC001"
      README.md
      python.yml         # id: acme/no-blobs
      fixtures/
```

The id of a rule is `<catalog>/<slug>` and its code is the code of the catalog, the code of the
family and 3 digits. The code of a catalog is letters A-Z, does not start with `BT` and is not a
text prefix of the code of another catalog. The name of a catalog is not the name of a family of
the built-in catalog (`structure`, `logging`…): a catalog with such a name does not load. A catalog
takes rules of every kind but `native`. The `covered-by` of a judgment rule of a catalog names the
rules of that catalog by id (`acme/no-blobs`) and those of the built-in catalog by slug.

`rule-dirs` (`[".botica/rules"]` by default) lists the folders whose subfolders are catalogs, and
`[catalogs]` declares one catalog by the folder it is in, relative to the folder of `botica.toml`:

```toml
rule-dirs = [".botica/rules", "team-rules"]

[catalogs]
shared = { path = "../shared-rules" }   # the catalog `shared`; the folder holds its catalog.toml
```

The name of a catalog is the name of its folder in a `rule-dirs` folder, or its key in
`[catalogs]`, and it is used once. A folder of the project that holds catalogs is left out of the
walk, as `.botica` is. The rules of a declared catalog are read from its folder: an extension script
is found there.

A family of a project may have the name of a built-in family, as `structure` above. Every command
then prints a warning on stderr, unless its `family.toml` says that it extends the built-in family
on purpose:

```toml
code = "STRUC"
description = "…"
extends = "structure"   # a family of the built-in catalog
```

`extends` changes one thing: `botica rules export` puts the rules of the family in the folder and
the section of the built-in family, after its rules. The ids and the codes stay those of the
catalog. A project replaces a built-in rule with `replaces` in its `rule.toml` and `[replace]` in
`botica.toml`; `botica rules --all` shows which rule replaces which.

A rule of kind `extension` is a program that `botica lint` starts, one process for each active
rule, in the root of the project. It is sent only the files where the rule applies and answers
with findings over JSON-RPC (ADR 0003). Its implementation says what runs:

```toml
[[implementations]]
kind = "extension"
runtime = "python"          # python | node | command
languages = ["python"]
file = "no-blobs.py"        # python: `uv run --script`; node: `node` (.js or .mjs)
# command = ["sh", ".botica/rules/acme/structure/no-blobs/check.sh"]   # runtime = "command", no file
```

A script of the runtime `python` declares its dependencies (PEP 723) and writes its rule with the
module `botica.sdk` of the wheel `botica`; the extra `botica[sdk]` brings `ast-grep-py`. The script
pins the version of `botica` that the project runs (`<version>` below is the output of
`botica --version`):

```python
# /// script
# requires-python = ">=3.10"
# dependencies = ["botica[sdk]==<version>"]
# ///
from botica.sdk import rule, serve

@rule("acme/no-blobs", ["python"], severity="error")
def no_blobs(path, text):
    yield (0, 1, "acme/no-blobs: a finding is a start and an end in bytes, and a message")

if __name__ == "__main__":
    serve()
```

A script of the runtime `node` can write its rule with [`@botica/sdk`](sdk/node/README.md)
(`rule`, `byteOffsets`, `serve`), which is not published to npm yet; `botica rules new --runtime
node` writes a script that speaks the protocol with no dependency. A program in Rust uses the crate
[`botica-ext-sdk`](sdk/rust/README.md) and a program in Go uses the [Go module](sdk/go/README.md);
both run as a rule of the runtime `command`.

A finding with another `rule` than the id of its rule is left out, with a warning on stderr. A
process that fails, that does not answer a request in time, or that is still running 5 seconds
after `shutdown`, is killed and stops the run with exit code 2; the findings of the other rules
stay. The process has 120 seconds to answer `initialize`, because a script run with `uv` may
download its dependencies first, and 60 seconds to answer `lint/files` and `shutdown`. The
variable `BOTICA_EXTENSION_TIMEOUT`, a number of seconds, sets both. The built-in catalog takes no
extension: an extension is a program of the project.

`botica server` runs the extension rules only when the editor asks for them: the flag
`--extensions`, or the option `{"extensions": true}` in the `initializationOptions` of
`initialize`. `botica.toml` has no key for it. It is heavy, because each rule is a process that
stays alive for the whole session, and it is a risk, because opening a repository in the editor
then runs the scripts of that repository (Python, Node, commands). A rule of `scope = "file"` has
its process started when the first document in one of its languages opens. It is sent the text of
the buffer, not the file on disk: at once when the document opens, and after the text has stayed
as it is for 250 ms when it changes. What it finds is published after the findings of the engine,
in a second `publishDiagnostics` with both, and leaves the buffer when its text changes, until the
rule answers again. A rule of `scope = "project"` does not run on a buffer. A process that fails, or
does not answer within its deadline, is
started again up to 3 times, with a `window/showMessage` warning each time; after that the rule
does not run until the server restarts. At `exit`, or at the end of the input, each process gets
`shutdown` and is killed after 5 seconds if it is still running.

`acme/no-blobs` and `ACMESTRUC001` name a rule and `ACMESTRUC` a family, in `select`, `ignore`,
`--select` and `--ignore`; `botica explain` takes a rule. `[rules."acme/no-blobs"]` configures it,
and `# botica: ignore[acme/no-blobs]` or `# noqa: ACMESTRUC001` silences it. A catalog that does
not load stops the command with exit code 2 and names the file and the problem.

## Silence a finding

```python
x = compute()  # botica: ignore[some-rule]
# botica: ignore-file[file-size-guard]    <- anywhere in the file
y = 1  # noqa: BTSTRUC002 -- reason
```

A finding about a whole file (size, name, layout) has no line: use `ignore-file`, the rule's
`ignores`/`extend-ignores`, or `extend-exclude`. With ruff, put `lint.external = ["BT"]` in its
configuration so it does not report `RUF102` for `BT` codes (the scaffolding template does).

While a repository migrates from ast-grep, `compat-ast-grep-ignore = true` makes botica honour
`# ast-grep-ignore: <rule>` too, on the line of a finding and on the line before it. It is off by
default, and the README of a rule does not repeat it.

## prek

```toml
# prek.toml: the mirror installs the wheel of the same version from PyPI
[[repos]]
repo = "https://github.com/collectiveai-team/botica-pre-commit"
rev = "v0.11.0"

[[repos.hooks]]
id = "botica"
args = ["--select", "<the rules of prek-generic.toml>"]
```

The mirror's hook is `verbose`: prek hides the output of a passing hook, and a size warning does
not fail it ([`distribution/botica-pre-commit`](distribution/botica-pre-commit/README.md)). The
scaffolder ships this hook, with the rules it selects, in `scaffolding/templates/prek-generic.toml`.

## Result cache

`--cache` keeps per-file results in `.botica/cache` under the project root; `--cache-dir <dir>`
chooses another place. It is off by default and meant for CI and `--all-files`. The key is the file
path and bytes; the whole cache is discarded when the version, the configuration or the rules
change. A directory botica creates ignores itself in git; one that already existed is left alone,
so add it to your `.gitignore`. A cache that cannot be written (a read-only checkout) is a warning
on stderr, not a failure: the findings and the exit code are the same, only the next run is slower.

## Develop

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --no-fail-fast
```

CI (`.github/workflows/ci.yml`, which checks the whole workspace) also sets `BOTICA_REQUIRE_RUFF=1`, so the catalog test
cannot skip itself for lack of `ruff`, and `BOTICA_REQUIRE_READ_ERRORS=1`, so the permission tests
cannot skip themselves when the runner is root. Set them locally to run as CI does. `bench/` has
the parity and timing scripts (`parity.py`, `hook_parity.py`, `run.sh`).

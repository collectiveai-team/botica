# External rules and recipes: the catalog describes the tools that enforce a standard, and botica installs them

**Status:** proposed

Several standards are enforced by another tool, set up through its configuration: CES-109 by
deptry, CES-110 by complexipy, CES-118 by jscpd, CES-58 by the stack of ruff and pyrefly, and the
codes of ruff that `scaffolding/templates/pyproject-template.toml` selects (`C90` among them). What
each tool needs lives in three places: the text of the standard, the templates of the scaffolder and
`prek.toml`. `docs/standards-migration-pending.md` leaves open how a standard says what enforces it
when no rule of botica does (decision 1) and where the knowledge of each tool goes (decision 3). The
scaffolder writes the configuration of a repository from its own components, apart from the catalog.

## Decision

1. **A fifth kind of implementation, `external`.** A rule of kind `external` lives in
   `rules/<family>/<slug>/` like the others, with its slug, its code and its `standard`. Another tool
   enforces it, and the implementation names it:

   ```toml
   [[implementations]]
   kind = "external"
   tool = "complexipy"
   docs = "https://rohaquinlop.github.io/complexipy/"
   codes = []                    # the codes of the tool, e.g. ["C901"] for ruff
   config = "config/prek.toml"   # the fragment of configuration the tool needs
   ```

   Its README has the sections Directive, Why, Tool (the tool, its documentation, and that the
   configuration is what enforces the rule), Configuration, Do, Don't and Suppression, which gives
   the channel of the tool (`# noqa: C901` for ruff). It answers decisions 1 and 3 of
   `docs/standards-migration-pending.md`: a standard is enforced by its rules, `external` ones
   included, and the knowledge of a tool is the README of its rule.

   `tool` and `docs` are required; `codes` and `config` are optional, and only an external
   implementation has them. `config` is `config/<file>.toml`, a file of the folder of the rule that
   the catalog embeds, and its name is the file of the project the fragment goes into, from the root
   of the project: `config/prek.toml` goes into `prek.toml`. A fragment is TOML. A rule whose
   implementations are all external has no `scope`, `needs`, `run-on` or `prefilter`, has no
   fixtures, and the conformance runner skips it, as it skips a judgment rule. A rule may have an
   external implementation beside a deterministic one (ruff for Python, ast-grep for TypeScript), and
   then it has the fields of a deterministic rule. The README of a rule with an external
   implementation has the sections Directive, Why, Exceptions (optional), Tool, Configuration, Do,
   Don't, Options (optional) and Suppression. `botica rules new --kind external` takes `--tool`,
   `--docs` and, optionally, `--config <file>.toml`, which writes the fragment with a line to fill in.
2. **Botica checks the configuration of an external rule.** A native rule, `external-config` (`BT004`,
   enabled by default, severity error), reports a repository where the fragment that an active
   external rule declares is missing from the file it names (`pyproject.toml`, `prek.toml`). The
   tool checks the code; botica checks that the tool is set up. Its scope is `project`: every
   `botica lint` reads those files at the root of the project, whatever files it is given, and reports
   each file as a whole when it does not exist, is not TOML, or lacks the fragment. A file holds a
   fragment when it has each key of the fragment with a value that holds the value of the fragment: a
   table holds the keys it names, an array holds one item for each item of the fragment, and any
   other value is equal. `--scope` leaves the rules with no scope (judgment and external ones) as
   the selection made them.
3. **A recipe is a package of steps,** `recipes/<name>.toml`. It prepares a repository: it lists the
   rules and the exports of botica, and it declares the skills to install and the files and
   sections to add. A recipe composes with `extends`:

   ```toml
   # recipes/base.toml
   description = "Any repository"
   rules = ["file-size-guard", "no-notebooks", "no-artifacts-in-package"]
   exports = ["agents-md"]

   [[skills]]                  # npx skills add <source> --skill <name>...
   source = "mattpocock/skills#v1.2.3"
   names = ["tdd", "grilling"]

   [[skills]]
   source = "collectiveai-team/botica"
   names = ["engineering-rules", "handoff"]

   [[files]]                   # adds the file from a template when it does not exist
   path = "prek.toml"
   template = "files/prek-base.toml"

   [[sections]]                # adds a section that starts with the marker to a file
   path = "AGENTS.md"
   marker = "## Engineering Standards"
   template = "sections/standards-index.md"

   [[gitignore]]               # adds lines to .gitignore, and removes lines of the whitelist
   lines = [".env", ".tmp/"]
   unignore = ["skills-lock.json"]
   ```

   ```toml
   # recipes/base-python.toml
   description = "A Python project"
   extends = ["base"]
   rules = ["log-no-print", "cognitive-complexity", "dep-hygiene"]
   judgment = ["test-through-interface", "arch-vocabulary"]
   exports = ["agents-md", "abide"]
   ```

   Every key is optional and the file may be empty. `description` is one line for
   `botica recipe list`. `extends` names recipes whose steps come first. `rules` lists the rules
   that are not judgment rules, `judgment` lists the judgment rules, each by slug, code or alias,
   and `exports` names `agents-md` or `abide`; the rules are a step like the others. `[[skills]]`
   has `source` (a repository, with a `#tag` to pin it) and `names`. `[[files]]` has `path` and
   `template`. `[[sections]]` has `path`, `marker` and `template`. A `template` is a file under the
   folder of the recipe (`files/`, `sections/`). `[[gitignore]]` has `lines` and `unignore`, and
   needs one of them. A line is one line of text, and a recipe lists it once across its steps.
   `unignore` accepts only `skills-lock.json`, and `lines` does not accept it (ADR 0001, CES-107).
   A list names an entry once, and a recipe names a skill, a file path, or a marker of a path once.
   A recipe that extends others contributes after
   them, each recipe contributes once, and an entry that two recipes list stays where it first is.
   Each step keeps the recipe it comes from. A skill that two recipes take from different sources
   is taken over by the recipe that comes later, which keeps the place of the skill, and the
   resolution reports a warning for each such skill. Recipes that extend one another in a circle
   are an error.
   A template is a file of the recipes folder and part of the recipe package. It has no id, no page
   on the site and no command of its own. A `[[files]]` template is under `files/` and a
   `[[sections]]` template under `sections/`, and a recipe loads only when the template of each of
   its steps exists. Recipes of the same folder share templates. A standard that cites a template
   as canonical code cites it with a reference (ADR 0007).
4. **Recipes are built into the binary and read from `recipes/`.** `lint/recipes/` is compiled into
   the binary with its templates: `base` has the three rules that look at any file, and
   `base-python` extends it with the rules about Python code that `scaffolding` selects today. They
   were `generic` and `python` in the first cut. A project adds its own recipes in `recipes/` at its
   root, with its own steps and templates, and a recipe of the project may extend a built-in one. A
   recipe of the project with the id of a built-in one is an error unless the configuration names
   the id in `[recipes] replace`, as for the topics of ADR 0007.
5. **`botica recipe list`, `show` and `check` read, and `botica recipe install <name>` writes what a
   repository lacks.** `show` prints the steps of the recipe. `check` goes through them, in order,
   and prints one line for each; it exits with 1 when one differs:
   - a rule is in the catalog and active in the project (a judgment rule under `judgment`, any
     other rule under `rules`);
   - an export is current, as `botica rules export --check` decides;
   - a file exists;
   - a section's marker is in its file;
   - `.gitignore` has each line of `lines` and has no line of `unignore`;
   - a skill is in `skills-lock.json` and in `.agents/skills/`.

   The lines come in the order rules, judgment rules, exports, files, sections, `gitignore`,
   `unignore`, skills. A path is from the root of the project, wherever the command runs. A marker
   is searched for as text anywhere in the file, as the scaffolder does. A skill is in the lock when
   its name is a key of `skills`, and in `.agents/skills/` when `.agents/skills/<name>/SKILL.md` is
   a file. `.gitignore` has a line when one of its lines equals it once the white space at its end
   is dropped, and has an `unignore` line when one of its lines equals it between white space.

   `install` adds what is missing and never merges or overwrites a file that exists (CES-30). The
   one edit it makes to a file that exists is to append a section. It does the files first, then the
   sections, and prints one line for each step:
   - A `[[files]]` step writes the file from its template when the path does not exist (`added`).
     A path that exists is `ok` when it has the text of the template and `pending` otherwise, for a
     person or an agent to merge; `install` does not touch it.
   - A `[[sections]]` step writes the file with the section when the path does not exist, and
     appends the section after a blank line to a file that exists without the marker (`added`). It
     does nothing when the file has the marker (`ok`). A path that is not a file or cannot be read
     is `pending`.
   - A step that two of the named recipes share is done once, and a name that is not a recipe fails
     the command before anything is written.
   - A `[[gitignore]]` step writes `.gitignore` with its lines when it does not exist and appends
     the ones that `.gitignore` lacks, after a line break when the last line has none (`added`). A
     line that is there is `ok`; a `.gitignore` that is not a file or cannot be read leaves each
     line `pending`. `unignore` removes each line that is the given line between white space, and
     the rest of the file keeps its bytes (`edited`, which the summary counts apart); the file
     without that line, or without the file, is `ok`. `install` does not tell that a pattern such
     as `*.json` ignores the manifest.
   - The `[[skills]]` steps run last. `install` runs `npx skills add <source> --agent opencode --yes
     --skill <name>...` once for each source, for the names that are not in `.agents/skills/`. A
     name that is there is `ok`; one that is there afterwards is `added`; one that is absent
     afterwards, or whose call failed or could not run, is `failed` and the command exits with 1.
     The reason is the last line of the stderr of `npx`; the CLI of skills leaves stderr empty and
     writes its errors to stdout, so then it is the first error of stdout (a line with `■`, other
     than `Canceled`) followed by the last `fatal:` or `error:` line below it.
     The other sources still run, and the files and sections are already written.
     `skills-lock.json` is written by the CLI of skills and by nothing else (CES-107). The skills of
     `collectiveai-team/botica` have no tag and follow its default branch; the lock fixes their
     content with `computedHash`.
   - A skill that two named recipes take from different sources is taken over by the later name,
     with a warning, as for one recipe.
   - The command exits with 1 when a step is pending or a skill failed, with 0 otherwise, and with
     2 for an error that stops it.

   `install` and `check` replace the commands of `scaffolding`. Each component of `scaffolding`
   becomes a step of a recipe. The `unignore` op of ADR 0001 is the `unignore` list of
   `[[gitignore]]`, bounded by the same whitelist, and `install` reports it as `edited`.

   A variant of a repository is a recipe chosen by name: `agent-opencode`, `agent-claude-code` and
   `agent-codex` for the agent, one recipe for a public repository and another for a private one.
   `install` takes one or more names (`botica recipe install base base-python agent-claude-code`),
   and a step does not carry a condition.

## Not built yet

- The steps of `base-python`. `base` has the skills of `scaffolding`, its `prek.toml` and its
  standards index; a test in `tests/test_recipe_base.py` keeps them equal to the ones of
  `scaffolding` until it is removed.
- The steps `[[links]]` (a symbolic link, such as `CLAUDE.md` to `AGENTS.md`) and `[[tools]]` (how a tool is added, such as `uv add --dev ruff`, and
  the fragment of configuration it carries). `base-python` will add ruff, pyrefly and pytest with
  `[[tools]]`.
- The recipes of the agents and of the visibility of the repository.
- The components of `scaffolding` as steps: `agent-config`, `prek`, `pyproject`, `ci`,
  `agents`, `standards`, `skills` and `varlock`.
- In `botica recipe check`, the hooks of prek and the fragments of the external rules. `BT004`
  checks the fragments when `botica lint` runs.

## Not decided

- Whether a fragment holds the whole configuration that `botica recipe install` writes or the
  least that `external-config` checks. The fragment of `cognitive-complexity` is a hook of prek
  with the id `complexipy`.
- What happens when two steps name the same path. `base` adds `prek.toml`, and the hooks of Python
  would go in the same file, which `install` does not edit: the second step finds the file with
  the text of the first template and reports it as pending. The direction under discussion for a
  `[[files]]` step: `install` tries to merge the template into the file, and when the merge
  conflicts it leaves the file as it is and reports the conflict for a person or an agent to
  resolve. That changes the rule that `install` never merges a file that exists (CES-30, ADR 0001),
  and a merge needs a common ancestor that `install` does not have for a file that the repository
  wrote first. Until it is decided, a `[[files]]` step lists a file that exists as pending.
- The agent that `npx skills add` installs for. `install` passes `--agent opencode`, the default of
  `scaffolding`, until the recipes of the agents exist.
- When `scaffolding` is removed.

## Considered options

- **Keep the knowledge of a tool in the text of its standard.** Rejected: a standard may have several
  tools, a tool may serve several standards, and botica could not check the configuration.
- **Recipes outside the catalog, as templates of the scaffolder.** Rejected: there would be two
  sources for what a repository needs.
- **Botica runs the external tools.** Rejected in ADR 0003: prek runs each tool as its own hook.
- **The name `profile`.** The first draft used it for the file of the catalog that lists rules and
  exports; the name is `recipe`.
- **`check` compares the recipe with the selection of the project and not with its files.** The
  hooks of prek and the fragments of the external rules are files that `install` writes, so their
  check goes with install.
- **Recipes only built into the binary.** A project that wants a list of its own would have to
  patch the catalog of botica.
- **Two concepts, a recipe of rules and a package of steps.** Rejected: a repository would have two
  lists to keep, and the rules are one more step of the package.
- **`generic` and `python` stay beside `base` and `base-python`.** Rejected: the first names were
  those of the first cut, and two names for one recipe would stay in the documents.
- **Templates as objects of the catalog, with an id, a page and a command.** Rejected: only a step
  uses a template, and nothing else cites or versions it apart from the recipe.
- **A condition on a step (`when`) for the agent or the visibility of the repository.** Rejected:
  a recipe for each variant, chosen by name, needs no language of conditions.
- **A recipe names only components of the binary.** Rejected: a project could not add a step of its
  own. The steps are declared in the `.toml` of the recipe, and a recipe of the project defines
  its own.

## Consequences

- `rule.toml` accepts `kind = "external"` with `tool`, `docs`, `codes` and `config`, and the catalog
  enforces the template of its README.
- The standards marked `[prek]`, `[ci]` or `[dependency]` in `AGENTS.md` become external rules, one
  pull request per standard.
- The site shows the tool and its documentation on the page of an external rule.
- `scaffolding install` and `scaffolding check` are replaced by `botica recipe install` and
  `botica recipe check`.
- The catalog carries `lint/recipes/` and its templates, loads `recipes/` of the project, and the
  configuration has the table `[recipes]` with `replace`.
- A recipe installs skills (`npx skills add`), so `botica recipe install` needs `npx` on the
  `PATH` for a recipe with a `[[skills]]` step.

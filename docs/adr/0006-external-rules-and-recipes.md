# External rules and recipes: the catalog describes the tools that enforce a standard, and botica installs them

**Status:** accepted

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
   implementation has them. `config` is `config/<path>`, a file under the folder `config/` of the
   rule that the catalog embeds, and its path is the file of the project the fragment goes into,
   from the root of the project: `config/prek.toml` goes into `prek.toml`, and
   `config/.cargo/config.toml` into `.cargo/config.toml`. A fragment is in the format of its file:
   TOML, YAML (`.yaml`, `.yml`) or INI (`.ini`, `.cfg`, `.npmrc`; ADR 0014). `applies-when`, also
   optional and only of an external implementation, lists globs of the files that show that a
   project uses the tool (`pnpm-lock.yaml`, `**/Dockerfile`; ADR 0014). A rule whose
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
   `botica lint` reads the settings of the tool at the root of the project, whatever files it is
   given, and reports a file as a whole when no file of settings exists, when the file cannot be read
   in its format, or when the settings lack a key of the fragment. A key that the settings set to a
   weaker value is the decision of the repository, and an external rule asks nothing of a project
   that does not use its tool. For the tools it knows, botica reads the settings where the tool reads
   them and accepts what the tool accepts (ADR 0014). `--scope` leaves the rules with no scope
   (judgment and external ones) as the selection made them.
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

   [[links]]                   # adds a symbolic link when its path does not exist
   path = "CLAUDE.md"
   target = "AGENTS.md"

   [[tools]]                   # adds packages with a package manager
   manager = "uv"
   packages = ["ruff", "pytest"]

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
   `exports` names `agents-md` or `abide`, and `agents` names the agents that the skills are
   installed for (`opencode`, `claude-code` or `codex`); the rules are a step like the others. `[[skills]]`
   has `source` (a repository, with a `#tag` to pin it) and `names`. `[[files]]` has `path` and
   `template`. `[[sections]]` has `path`, `marker` and `template`. A `template` is a file under the
   folder of the recipe (`files/`, `sections/`). `[[links]]` has `path` and `target`, the text
   of the link, relative to the folder of the link. `[[tools]]` has `manager` (`uv`), `packages` and
   optionally `group` (`dev` when absent); a package is a requirement without a URL, a name with
   optional extras, versions and marker, and the name is what `check` looks for; two recipes that
   ask for a package with different requirements do not resolve. `[[gitignore]]` has `lines` and
   `unignore`, and needs one of them. A line is one line of text, and a recipe lists it once across
   its steps.
   `unignore` accepts only `skills-lock.json`, and `lines` does not accept it (CES-107).
   `unignore` is the one step that deletes text from a file that the repository has: CES-107
   tracks `skills-lock.json`, and a repository that already ignores it would keep the manifest
   out of git. The whitelist is a literal list in the catalog (`UNIGNORE_WHITELIST`), and a new
   entry is a one-line change; a pattern or a caller-supplied string is rejected. `install` removes
   a line equal to the entry between blanks and leaves the other bytes as they are (CRLF, a last
   line without a newline), and reports it as `edited`, apart from `added`. A pattern such as
   `*.json` that ignores the manifest is outside what `install` reads, and stays.
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
   A recipe declares the parameters that its templates write under `[params.<name>]`, with
   `description`, `default`, `kind` (`text` when absent, `days` or `version`) and optionally
   `choices`. A template writes `{{ param.<name> }}`, and a kind adds views, so the template does
   no arithmetic: `days` gives `{{ param.<name>.minutes }}` and `.seconds`, and `version` gives
   `{{ param.<name>.at }}`, which is `@<value>`, or nothing for an empty value (an unpinned tool).
   Only that form is replaced, so `${{ github.ref }}` stays. Two recipes of a chain may declare a
   name the same and not differently, and a template that writes a name that no recipe of its chain
   declares does not resolve. The value is the one of `--set <name>=<value>` of `install`, else the
   one of `[recipes.params]` of the configuration, else the default; `install` keeps a value of
   `--set` in `[recipes.params]` of a new `botica.toml` and reports one that an existing
   configuration lacks as `pending`, as for the selection.
4. **Recipes are built into the binary and read from `recipes/`.** `catalog/recipes/` is compiled into
   the binary with its templates: `base` has the three rules that look at any file, the skills,
   `.gitignore`, `prek.toml` and the standards index that `scaffolding` installed, and `base-python`
   extends it with the rules about Python code that `scaffolding` selected, a `pyproject.toml` template,
   the Python hooks of prek (sections of `prek.toml`, one for each repository of hooks), the
   configuration of ruff, pyrefly and pytest (sections of `pyproject.toml`) and the dev packages (a
   `[[tools]]` step). Tests in `tests/test_recipe_base.py` kept both equal to what `scaffolding`
   wrote until it was removed on 2026-10-18. A second step on a file that a template wrote is a `[[sections]]`
   step with a marker, not another `[[files]]` step. They were `generic` and `python` in the first
   cut. A project adds its own recipes in `recipes/` at its
   root, with its own steps and templates, and a recipe of the project may extend a built-in one. A
   recipe of the project with the id of a built-in one is an error unless the configuration names
   the id in `[recipes] replace`, as for the topics of ADR 0007.
5. **`botica recipe list`, `show` and `check` read, and `botica recipe install <name>` writes what a
   repository lacks.** `show` prints the steps of the recipe. `check` goes through them, in order,
   and prints one line for each; it exits with 1 when one differs:
   - a rule is in the catalog and active in the project (a judgment rule under `judgment`, any
     other rule under `rules`);
   - the file that an external implementation of a rule under `rules` names holds its fragment,
     as `external-config` decides, whether or not the rule is active (the hook of prek of
     `cognitive-complexity`, for one);
   - an export is current, as `botica rules export --check` decides;
   - a file exists;
   - a section's marker is in its file;
   - a link's path is a symbolic link whose text is its target;
   - `.gitignore` has each line of `lines` and has no line of `unignore`;
   - `pyproject.toml` declares each package of a tool;
   - a skill is in `skills-lock.json` and in `.agents/skills/`.

   `check` does not read the value of a parameter back from a file: a file that `install` wrote
   belongs to the repository (CES-30), and a value that a standard requires is the fragment of an
   external rule. A value of `[recipes.params]` that does not fit its parameter fails the command.

   `check` takes one or more names, as `install` does, and checks them as one recipe: the chain has
   each recipe once, an entry that several list stays where it is first, and the warnings of the
   resolution go to stderr. The last line names the recipes.

   The lines come in the order rules, judgment rules, fragments, exports, files, sections, links,
   `gitignore`, `unignore`, tools, skills. A path is from the root of the project, wherever the command
   runs. A marker is searched for as text anywhere in the file, as the scaffolder did. A skill is
   in the lock when its name is a key of `skills`, and in `.agents/skills/` when
   `.agents/skills/<name>/SKILL.md` is a file. `.gitignore` has a line when one of its lines equals it once the white space at its end
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
   - A `[[sections]]` step to a path that ends in `.toml` appends only when the file is valid TOML
     before and after; otherwise it is `pending`, and the file is not touched.
   - A `[[links]]` step creates the link, with the folders of its path, when the path does not exist
     (`added`). A path that holds a link with the text of the target is `ok`; anything else is
     `pending` and `install` does not replace it.
   - A `[[gitignore]]` step writes `.gitignore` with its lines when it does not exist and appends
     the ones that `.gitignore` lacks, after a line break when the last line has none (`added`). A
     line that is there is `ok`; a `.gitignore` that is not a file or cannot be read leaves each
     line `pending`. `unignore` removes each line that is the given line between white space, and
     the rest of the file keeps its bytes (`edited`, which the summary counts apart); the file
     without that line, or without the file, is `ok`. `install` does not tell that a pattern such
     as `*.json` ignores the manifest.
   - A `[[tools]]` step runs `uv add --dev <package>...` (`--group <group>` for another group) in
     the root of the project, once for each group, for the packages that `pyproject.toml` does not
     declare. A package that is declared afterwards is `added` and one that is not is `failed`.
     When a call for several packages fails, `install` makes it again for each package, because
     `uv add` adds none when one does not exist. A project without `pyproject.toml` leaves each
     package `pending`.
   - The `[[skills]]` steps run last. `install` runs `npx skills add <source> --agent <agents>
     --yes --json --skill <name>...` once for each source, for the names that are not in `.agents/skills/`. A
     name that is there is `ok`; one that is there afterwards is `added`; one that is absent
     afterwards, or whose call failed or could not run, is `failed` and the command exits with 1.
     The agents are the ones that the named recipes list, each once and in the order they come,
     and `opencode` when none lists one.
     `--json` makes `npx` write a JSON array to stdout: a skill that it lists as `skipped` is
     `failed` with the reason of the entry, and the reason of a failed call is the first line of the
     `error` of the first `failed` entry followed by the last `fatal:` or `error:` line below it,
     or else the last `cause:` line of stderr, or its last line, without the `hint:` lines.
     The other sources still run, and the files and sections are already written.
     `skills-lock.json` is written by the CLI of skills and by nothing else (CES-107). The skills of
     `collectiveai-team/botica` have no tag and follow its default branch; the lock fixes their
     content with `computedHash`.
   - A skill that two named recipes take from different sources is taken over by the later name,
     with a warning, as for one recipe.
   - The command exits with 1 when a step is pending or a skill failed, with 0 otherwise, and with
     2 for an error that stops it.

   `install` and `check` replace the commands of `scaffolding`. Each component of `scaffolding`
   becomes a step of a recipe. The `unignore` op of `scaffolding` is the `unignore` list of
   `[[gitignore]]`, bounded by the same whitelist, and `install` reports it as `edited`.

   A variant of a repository is a recipe chosen by name: `agent-opencode`, `agent-claude-code` and
   `agent-codex` for the agent, one recipe for a public repository and another for a private one.
   `agent-claude-code` links `.claude/skills` to `../.agents/skills` and names no agent, because
   `npx skills add --agent claude-code` writes in `.claude/skills/<name>` and fails through that link.
   `install` takes one or more names (`botica recipe install base base-python agent-claude-code`),
   and a step does not carry a condition. A parameter changes a value inside a template, never
   whether a step runs. A recipe `full-<language>` extends the `base-*` and the
   `ci-*` of a language and has no step of its own: one name for the whole package of a language.

   The components of `scaffolding` are recipes: `gitignore`, `agents`, `standards` and `skills` in
   `base`, `prek` and `pyproject` in `base-python`, `agent-config` in `agent-opencode`,
   `agent-claude-code` and `agent-codex`, `ci` in the recipes `ci-*`, `repo-public` and
   `repo-private`, and `varlock` in `secrets-varlock`. `secrets-varlock` installs the skill
   `varlock` of `dmno-dev/varlock`; a person or an agent runs `varlock init --agent`, which writes
   the draft of `.env.schema` from `.env.example` or from the variables that the code reads.

## Not built yet

- A recipe that lists an external rule. `botica recipe check` reports the fragment of each one
  (the hooks of prek are the fragments whose file is `prek.toml`), and `BT004` checks them when
  `botica lint` runs; no built-in recipe lists one yet, because `cognitive-complexity` is the only
  external rule and the standards of the other tools are not rules yet.

## Not decided

- Whether a fragment holds the whole configuration that `botica recipe install` writes or the
  least that `external-config` checks. The fragment of `cognitive-complexity` is a hook of prek
  with the id `complexipy`.

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
- **A parameter that the project only passes on the command line.** Rejected: a second `install`
  without the flag would write the default into a new file. The values live in `[recipes.params]`,
  as copier keeps its answers in `.copier-answers.yml`.
- **Expressions or filters in a template (Jinja).** Rejected: the conversions that the templates
  need are a few units of time and an optional `@` before a version, which the kind of a parameter
  gives; a language of expressions would also need escaping for the `${{ }}` of the workflows.
- **A recipe names only components of the binary.** Rejected: a project could not add a step of its
  own. The steps are declared in the `.toml` of the recipe, and a recipe of the project defines
  its own.
- **A template of `.env.schema` in `secrets-varlock`.** Rejected: with an `.env.schema` in place,
  `varlock init --agent` (1.23.0) keeps it and drafts nothing from `.env.example` or from the code,
  and `varlock load` fails on a schema with no variable.

## Consequences

- `rule.toml` accepts `kind = "external"` with `tool`, `docs`, `codes` and `config`, and the catalog
  enforces the template of its README.
- The standards marked `[prek]`, `[ci]` or `[dependency]` in `AGENTS.md` become external rules, one
  pull request per standard.
- The site shows the tool and its documentation on the page of an external rule.
- `botica recipe install` and `botica recipe check` replaced `scaffolding install` and
  `scaffolding check`, and `scaffolding` was removed on 2026-10-18.
- The catalog carries `catalog/recipes/` and its templates, loads `recipes/` of the project, and the
  configuration has the table `[recipes]` with `replace`, `params` and `installed`.
- A recipe installs skills (`npx skills add`), so `botica recipe install` needs `npx` on the
  `PATH` for a recipe with a `[[skills]]` step.

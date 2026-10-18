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
3. **A recipe is a file of the catalog,** `recipes/<name>.toml`. It lists rules of every kind and
   the exports to run, and it composes with `extends`:

   ```toml
   # recipes/python.toml
   extends = ["generic"]
   rules = ["file-size-guard", "log-no-print", "cognitive-complexity", "dep-hygiene"]
   judgment = ["test-through-interface", "arch-vocabulary"]
   exports = ["agents-md", "abide"]
   ```

4. **`botica recipe install <name>` writes what a repository lacks,** and `botica recipe check`
   reports how the repository differs from the recipe. Install writes the configuration of botica,
   the hooks of prek, the fragments of the external rules and the files of the exports. It adds a file
   that does not exist and never edits, merges or overwrites one that does; what it cannot add, it
   lists for a person or an agent to merge (CES-30). Both replace the commands of `scaffolding`,
   whose components and templates become recipes and fragments of the catalog.

## Not decided

- Whether a fragment holds the whole configuration that `botica recipe install` writes or the
  least that `external-config` checks. The fragment of `cognitive-complexity` is a hook of prek
  with the id `complexipy`.
- The order in which the components of `scaffolding` move to recipes, and when the package is
  removed.
- The bounded edits that `scaffolding` performs today (the `unignore` op of ADR 0001) and how
  `recipe install` keeps them.
- Where the drop-in code of a standard lives (`.agents/snippets/` today).

## Considered options

- **Keep the knowledge of a tool in the text of its standard.** Rejected: a standard may have several
  tools, a tool may serve several standards, and botica could not check the configuration.
- **Recipes outside the catalog, as templates of the scaffolder.** Rejected: there would be two
  sources for what a repository needs.
- **Botica runs the external tools.** Rejected in ADR 0003: prek runs each tool as its own hook.
- **The name `profile`.** The first draft used it for the file of the catalog that lists rules and
  exports; the name is `recipe`.

## Consequences

- `rule.toml` accepts `kind = "external"` with `tool`, `docs`, `codes` and `config`, and the catalog
  enforces the template of its README.
- The standards marked `[prek]`, `[ci]` or `[dependency]` in `AGENTS.md` become external rules, one
  pull request per standard.
- The site shows the tool and its documentation on the page of an external rule.
- `scaffolding install` and `scaffolding check` are replaced by `botica recipe install` and
  `botica recipe check`.

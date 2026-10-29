---
title: Lint
---

# Lint

`botica lint` runs the rules of its catalog and of the [catalogs of the project](../extend/catalogs.md).
The design is in
[ADR 0003](https://github.com/collectiveai-team/botica/blob/main/docs/adr/0003-botica-lint-architecture.md).
The catalog under `catalog/rules/` is the only documentation of the rules: each rule has a README,
the [rules](../rules/index.md) of this site show it, and `botica rules` lists them.

## Commands

```bash
botica lint                    # the current directory
botica lint src tests/a.py     # directories to walk, or files (as prek passes them)
botica lint --select file-size-guard,no-utils
botica lint --output-format json   # text (default), json, sarif, github or markdown; json and sarif carry the
                                   # code and the standard of the rule of each finding
botica lint --judged judge.sarif   # the findings of a judge, filtered by the configuration and comments;
                                   # the file is SARIF or the report of `abide check --json`
botica lint --diff origin/main     # the files changed since the merge base, and only the findings on
                                   # the lines the change adds (ADR 0009); no result cache
botica lint --staged               # the same for the changes staged against HEAD, read from the index
botica lint --diff-file pr.diff    # the same for the changes of a unified diff
botica lint --commit-msg "$1"      # the rules of scope commit over a commit message, and no file
botica lint --commits origin/main..HEAD   # over each commit of the range (ADR 0010)
botica lint --pr-title="$PR_TITLE" # over the title of a pull request
botica rules                   # which rules are active, and why (--all adds the inactive ones), the
                               # programs that `botica lint` runs and the tools that enforce rules
                               # outside it; --json adds the kind of each implementation
botica rules --since 0.12.0    # what the releases after 0.12.0, up to this binary, did to the built-in
                               # rules: added, status changed, renamed, code changed or removed,
                               # one line per change; --json prints them as one document
botica explain no-notebooks    # what a rule is, how it is built and how it is configured here
botica rules eval no-utils src # what one rule finds in real code, on or off in the configuration,
                               # for a review of its false positives; the findings do not fail it
botica rules export --format markdown --output site   # one page per rule of the catalog, and an index;
                               # the catalogs of .botica/rules get a folder and an index section each
botica rules export --format json --output out       # the whole catalog in out/catalog.json
botica rules export --format agents-md --output AGENTS.md [--check]
                               # the active judgment rules, between <!-- botica:rules --> and <!-- /botica:rules -->;
                               # a judgment rule whose covered-by names an active rule is left out
botica rules export --format abide [--check]   # the rubric of abide, .abide/rubric.json, with the
                               # judgment rules of the block of AGENTS.md (--agents-md names another file)
```

[Editor](editor.md) has `botica server`, [Recipes](recipes.md) has `botica recipe`,
[References](refs.md) has `botica refs`, and [Write a rule](../extend/rules.md) has `botica rules
new`, `conform` and `test`.

Exit code: `0` clean (a warning does not fail the run), `1` a finding of severity error, `2`
the run itself failed (it wins over `1`). `--fail-on warning` (or `info`, `hint`) lowers the
severity that exits with `1`. `--exit-zero` forgives only the findings.

## Rules

`botica rules --all` lists the rules of the catalog, with their code and whether each is active and
why; `botica explain <slug>` shows one, with its standard and its status. A rule with the status
`preview` is off by default: it runs only with `preview = true` and when you select it. A rule of
kind `external` is enforced by another tool through its configuration
([External rules](../extend/rules.md#external-rules)).

`botica rules --since <version>` lists what each release after `<version>`, up to the version of
the binary, did to the rules of the built-in catalog, read from `catalog/rules/history.toml`:

```text
$ botica rules --since 0.9.0
...
0.10.0  no-console-log (BTLOG003)  removed, replaced by log-no-print
...
0.13.0  database-package (BTDB001)  renamed from arch-database-package
0.13.0  database-package (BTDB001)  code BTARCH001 -> BTDB001
...
0.18.0  botica-version-aligned (BT007)  added as preview
```

Each line has the release, the id and the code that the rule had in it, and the change. Run it
after an update with the version you had before to see which names of your configuration changed.
A version newer than the binary lists nothing.

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

## Result cache

`--cache` keeps per-file results in `.botica/cache` under the project root; `--cache-dir <dir>`
chooses another place. It is off by default and meant for CI and `--all-files`. The key is the file
path and bytes; the whole cache is discarded when the version, the configuration or the rules
change. A directory botica creates ignores itself in git; one that already existed is left alone,
so add it to your `.gitignore`. A cache that cannot be written (a read-only checkout) is a warning
on stderr, not a failure: the findings and the exit code are the same, only the next run is slower.

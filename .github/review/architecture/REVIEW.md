---
role: architecture
applies:
  paths: []
  tiers: [medium, high]
stages: [local, ci]
blocking: P1
owners: ["@jedzill4"]
max-findings: 8
---

## Invariants

- The crates of the workspace depend on each other one way (ADR 0003): `botica-cli` on
  `botica-core`, `botica-config`, `botica-conformance`, `botica-lsp`, `botica-catalog`,
  `botica-astgrep` and `botica-ext`; `botica-lsp` on `botica-core`; `botica-core` on
  `botica-catalog`, `botica-astgrep`, `botica-complexity` and `botica-ext`; `botica-config` on
  `botica-catalog`. No crate depends on `botica-cli`, and only `botica-cli` depends on
  `botica-lsp`, which `botica server` runs.
- `botica-core` lints text that it is given and does no I/O unless its feature `fs` is on; only
  `botica-cli` turns it on. Reading files, walking a folder, starting a process and the cache live
  behind that feature or in `botica-cli`.
- The SDKs under `sdk/` (`python`, `node`, `go`, `rust`) speak to `botica-ext` through the
  extension protocol only, and depend on no crate of the workspace.
- A command of the command line is a module of `crates/botica-cli/src/` (`lint_cmd`,
  `rules_cmd`, `refs_cmd`, `recipes_cmd`, `review_cmd`, `history_cmd`, `server_cmd`, `ext_cmd`,
  `explain_cmd`) that `main.rs` dispatches; the logic that two commands share lives in a module of
  its own or in a crate below the front-ends, not in one command that the other imports.
- A rule is one folder `catalog/rules/<family>/<slug>/` with its `rule.toml`; a native rule adds
  its code under `crates/botica-core/src/`, and the catalog is the one list of rules (ADR 0003,
  ADR 0005).
- The terms of `GLOSSARY.md` name the concepts of the domain (recipe, step, template, rule,
  catalog, scope, family, code, topic, judgment rule, extension rule, review area, tier and the
  others it defines), and the words of its `_Avoid_` lines name none of them.

## Check always

- A dependency added to the `Cargo.toml` of a crate between crates of the workspace, or an import
  of `botica_<crate>` in a crate whose `Cargo.toml` did not have it, against the direction above.
- A read of the disk, a walk of a folder or a `Command::new` added to `botica-core` outside the
  feature `fs`, or to `botica-catalog`, `botica-config`, `botica-astgrep` or `botica-complexity`.
- A module of one command of `botica-cli` that another command imports.
- A new trait, type or module of a crate with one user, and new code that does again what a
  module of `botica-core` or `botica-cli` already does (the walk of `walk/`, the reports of
  `report/`, the git of `lint_cmd/git.rs`).
- A new name of a module, a type, a command, a flag or a key of `botica.toml` for a concept that
  `GLOSSARY.md` or the code already names, and a word that an `_Avoid_` line of `GLOSSARY.md`
  excludes.
- `separate-orchestration` and `architecture-vocabulary` on Rust and Go code: the rubric of
  `abide.yml` judges them on Python, TypeScript and JavaScript only, so this role reviews them on
  the crates, the SDKs of Go and Rust and the topics.

## Do not report

- What `botica lint` reports under the rules of the family `architecture` that `[tool.botica]`
  of `pyproject.toml` selects. `import-layers` is selected with no `layers`, so it reports nothing,
  and the direction of the crates above is reviewed here.
- A refactor that the diff does not need.
- The layout of a topic under `catalog/references/` or of a template under `catalog/recipes/files/`:
  it is the layout that the topic or the recipe teaches to another repository, not the one of
  botica.
- The prose of a page, a standard or an ADR: `plain-writing`, `minimal-comments` and
  `ubiquitous-language` in `.abide/rubric.json` cover it.

<!-- botica:covered -->
- The job `judgment rules` of `abide.yml`: `separate-orchestration` and `architecture-vocabulary`
  on the Python, TypeScript and JavaScript files of the diff.
- The hook `botica-project` of `prek.toml`, run by the job `lint, type-check & test` of
  `tests.yml`: `import-layers`, selected in `[tool.botica]` with no `layers`, so it reports
  nothing in this repository.
<!-- /botica:covered -->

## Evidence

Each finding names the two crates or modules and the dependency, the import, the type or the name,
with file and line on both sides, and the line of ADR 0003, of `GLOSSARY.md` or of this file that
it goes against.

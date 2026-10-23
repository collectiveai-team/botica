# This repository runs the botica of its checkout

**Status:** accepted

The hooks `botica`, `botica-project`, `botica-agents-md` and `botica-rubric` of `prek.toml` come
from the mirror `botica-pre-commit` at the `rev` of the last release. The hooks of commit-msg run
`cargo run --quiet --locked -p botica-cli`, and `abide.yml`, `commit-policy.yml` and
`conventional-commits.yml` run the `target/debug/botica` they build (ADR 0010). Both read the
`select` of `[tool.botica]` in `pyproject.toml`. A rule that a pull request adds is in the catalog
of the checkout and not in the botica of the mirror, which exits with 2 and
`` botica: select[35]: unknown rule `test-no-subprocess` `` when the `select` lists it, and reports
`invalid-suppression` on a comment that silences it. ADR 0010 (decision 6) and ADR 0011 (decision 9) worked around this with hooks and steps that
ran the checkout with `--select <rule>` until the next release.

## Decision

1. The four hooks of botica in `prek.toml` are local hooks, `language = "system"`, whose entry is
   `cargo run --quiet --locked -p botica-cli --` followed by the arguments of the hook of the same
   id in `distribution/botica-pre-commit/.pre-commit-hooks.yaml`. They keep its `types`,
   `pass_filenames`, `always_run` and `verbose`, and the `priority` they had.
2. `prek.toml` no longer names the mirror, and `scripts/bump_mirror_rev.py` no longer moves a
   `rev` in it. The template `catalog/recipes/files/prek-base.toml`, the docs and the README of
   the mirror keep the mirror and its `rev`.
3. A pull request that adds a rule that a recipe of this repository selects lists it in `select`,
   and fixes or silences its findings in this repository, in the same pull request. No hook with
   `--select` waits for a release. `scripts/check_rule_change.py` keeps that pull request inside
   the contract of the rule: a modified file may add a line that names the rule to the `select`
   of `[tool.botica]` in `pyproject.toml`, comments `botica: ignore[<rule>]` or
   `botica: ignore-file[<rule>]` and nothing else, and `AGENTS.md` may change its block of rules
   and `.abide/rubric.json` the rubric, which botica exports from the catalog. A fix of a finding
   in code is outside the contract and takes the label `contract-change`.
4. The job `mirror-check` of `botica-release.yml` runs after `mirror`:
   `uvx prek try-repo https://github.com/collectiveai-team/botica-pre-commit --rev vX.Y.Z
   --all-files` on the commit of the tag. When it fails, `bump` does not run and the release ends
   red. The version is already on PyPI, so the fix is a patch release.
5. `tests.yml` installs the Rust toolchain before `prek run --all-files`.

## Considered options

- Keep the mirror and add a hook with `--select <rule>` for each new rule until the release.
  Rejected: it is what ADR 0010 and ADR 0011 did, a rule waits for a release to enter the `select`,
  and each one needs a hook, a step of CI and a later change that removes them.
- A rule that the `select` names and the catalog does not know is a warning, not exit 2. Rejected
  for this repository: the published botica would not run the new rule, so the hooks would pass
  without checking it. It may still help a project that moves the `rev` of the mirror behind its
  configuration; that is a change of the configuration of botica, and is not decided here.
- `target/debug/botica` in the entry, after a hook that runs `cargo build`. Rejected: the path
  depends on `CARGO_TARGET_DIR`, and prek runs hooks of the same priority at once. `cargo run`
  builds when it has to and waits on the lock of the build directory.
- `cargo run --release`. Rejected: the profile `release` of `Cargo.toml` uses `lto = "thin"` and
  `codegen-units = 1`, which build slower than the profile `dev`, and the hooks of commit-msg and the
  workflows build `dev`. On this repository the debug binary lints every file in under half a
  second more than the wheel.

## Consequences

- The first commit after a change to `crates/` or `catalog/` compiles botica before its hooks run,
  and a change that does not compile fails the hooks of botica. A commit that should pass anyway
  uses `SKIP=botica,botica-project,botica-agents-md,botica-rubric`.
- `prek run` needs `cargo` on the `PATH`, as the hooks of commit-msg already did.
- The job of `tests.yml` builds botica, which `ci.yml` builds too.
- This repository does not run the wheel of PyPI on a pull request. `mirror-check` runs it once per
  release, on the commit of the tag.
- The `--select` of the hooks of commit-msg, of `commit-policy.yml` and of
  `conventional-commits.yml` stays: those files are copies of the templates of the recipe, where
  each hook and each job checks one rule.

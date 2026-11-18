# Develop botica

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --no-fail-fast
uv run pytest                        # the maintainer scripts and the site
scripts/build_site.sh                # the site, in website/site
```

CI (`.github/workflows/ci.yml`, which checks the whole workspace) also sets `BOTICA_REQUIRE_RUFF=1`, so the catalog test
cannot skip itself for lack of `ruff`, and `BOTICA_REQUIRE_READ_ERRORS=1`, so the permission tests
cannot skip themselves when the runner is root, and `BOTICA_REQUIRE_WORKFLOW_LINTERS=1`, so
`crates/botica-catalog/tests/recipes_workflows_lint.rs` cannot skip actionlint or zizmor, which it
runs on the workflows of the recipes. Set them locally to run as CI does. `bench/` has
the parity and timing scripts (`parity.py`, `hook_parity.py`, `run.sh`).

The hooks of botica in `prek.toml`, the workflows and `abide.yml` run the botica of the checkout
(`cargo run --quiet --locked -p botica-cli`, or `target/debug/botica` after `cargo build`), not the
mirror (ADR 0013). The first commit after a change to `crates/` or `catalog/` compiles it, and a
change that does not compile fails the hooks of botica. A change that adds a rule lists it in
`select` of `[tool.botica]` in `pyproject.toml` and fixes or silences its findings in this
repository; `prek run --all-files` shows them. `scripts/check_rule_change.py` accepts in that
change the line of the rule in `select`, the comments `botica: ignore[<rule>]` and
`botica: ignore-file[<rule>]` in any file it modifies, and the block of rules of `AGENTS.md` and
`.abide/rubric.json` that botica exports; any other edit takes the label `contract-change`.
`botica-release.yml` runs the mirror on this repository after each release.

The job `mutants` of `ci.yml` runs on pull requests only and never blocks: it has
`continue-on-error` and a `timeout-minutes` cap. It runs `cargo mutants --in-diff` on the diff of the
pull request, with cargo-mutants 27.1.0, then `botica lint --mutants` on the report, and uploads
`mutants.out/` and `.botica/history/` as the artifact `botica-mutants`. To run it locally:
`git diff origin/main...HEAD > pr.diff`, `cargo mutants --in-diff pr.diff --test-workspace=true`,
`botica lint --mutants mutants.out/outcomes.json --diff-file pr.diff --preview --select mutant-survived`.

The workflow `scheduled-review.yml` runs every Sunday on `main`, and on demand with `workflow_dispatch`.
It only reads and holds no secret, except the job `history-report` (below). Its jobs are `botica lint --record` on every file (artifact
`botica-history-scheduled-lint`), cargo-mutants on the Rust lines that `main` changed in the last 7
days (`--in-diff`, split in 4 shards, with the same version as the job `mutants`; a week with no
Rust change runs no shard), whose `report` job joins the `outcomes.json` of the shards, runs
`botica lint --mutants` on them and prints the global score, and `agnix` through the hook of
`prek.toml`. The diff and shard caps bound the scheduled work independently of workspace size.
A `schedule` runs on the default branch only, so a pull request cannot
run it: the test `crates/botica-catalog/tests/scheduled_review_workflow.rs` holds its shape. To run a
shard locally: `git diff <commit of 7 days ago> HEAD -- '*.rs' > week.diff`, then
`cargo mutants --in-diff week.diff --test-workspace=true --shard 0/4 --sharding round-robin`, with
`TMPDIR` set to a short path (the copy of the tree fails when `.tmp/` is inside the worktree).

The job `history-report` runs after the others and is the only one with `issues: write` and
`actions: read`, with the `GITHUB_TOKEN` of the run. It downloads the artifacts `botica-history-*` of
the runs of `ci.yml` and `scheduled-review.yml` on `main` of the last 28 days (`HISTORY_DAYS`) that
are a `push`, a `schedule` or a `workflow_dispatch` of this repository whose commit is in the history
of `main`, writes the `source.json` of each run from the answer of the API (`botica history fetch`
does not exist yet), runs `botica history report --format markdown` and keeps at most 20 items
(`REPORT_MAX_FINDINGS`) of each section. The report goes to the issue titled `Weekly review report`
(`REPORT_TITLE`) through `--body-file`: the job edits the open issue with that title, and opens one
only when none is open and a section that asks for a decision has an item. It merges nothing and
changes no code. A pull request cannot run it either: `scheduled_review_report.rs` runs its steps
with bash and a `gh` that answers from files, and `scheduled_review_workflow.rs` holds its
permissions.

The hooks `botica` and `botica-project` of `prek.toml` pass `--record`, and the job `lint` of
`ci.yml` runs `botica lint --record` and uploads `.botica/history/` as the artifact
`botica-history-lint` (ADR 0019). The recipes and the mirror pass no `--record`: the botica of PyPI
that they run does not know the flag.

The manual of botica is the site, under `website/docs/` (ADR 0012): a change to a command
updates its page there. The release is in [`botica-release.md`](botica-release.md).

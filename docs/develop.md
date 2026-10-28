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

The manual of botica is the site, under `website/docs/` (ADR 0012): a change to a command
updates its page there. The release is in [`botica-release.md`](botica-release.md).

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
cannot skip themselves when the runner is root. Set them locally to run as CI does. `bench/` has
the parity and timing scripts (`parity.py`, `hook_parity.py`, `run.sh`).

The manual of botica is the site, under `website/docs/` (ADR 0012): a change to a command
updates its page there. The release is in [`botica-release.md`](botica-release.md).

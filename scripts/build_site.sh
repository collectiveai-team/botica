#!/usr/bin/env bash
# Build the standards site into website/site: export the rule pages from the catalog of this
# checkout, copy the standards of standards/, then run Zensical (the version is pinned in
# website/requirements.txt) in strict mode, so a broken link fails the build. The pages of the rules
# and the standards are copies, never committed.
set -euo pipefail

cd "$(dirname "$0")/.."

rm -rf website/docs/rules website/docs/standards website/site
cargo run --quiet --locked --manifest-path lint/Cargo.toml -p botica-cli -- \
  rules export --format markdown --output website/docs/rules
cp -r standards website/docs/standards
uv run --no-project --python 3.14 --with-requirements website/requirements.txt \
  zensical build --clean --strict --config-file website/zensical.toml

#!/usr/bin/env bash
# Build website/site from the catalog, standards/ and the topics; each step is a script of
# scripts/site_*.py. Zensical runs in strict mode, so a broken link fails the build. The generated
# pages and the configuration with the navigation are not committed.
set -euo pipefail

cd "$(dirname "$0")/.."

topics="$(mktemp)"
trap 'rm -f "${topics}"' EXIT

rm -rf website/docs/rules website/docs/standards website/docs/references website/site
cargo run --quiet --locked --manifest-path Cargo.toml -p botica-cli -- \
  rules export --format markdown --output website/docs/rules
cp -r standards website/docs/standards
uv run --no-project --python 3.14 --with cyclopts python scripts/site_standards.py website/docs
cargo run --quiet --locked --manifest-path Cargo.toml -p botica-cli -- \
  refs export > "${topics}"
uv run --no-project --python 3.14 --with cyclopts python scripts/site_references.py \
  "${topics}" website/docs
uv run --no-project --python 3.14 --with cyclopts python scripts/site_filter.py \
  website/docs/rules/index.md website/docs/standards/index.md website/docs/references/index.md
uv run --no-project --python 3.14 --with cyclopts python scripts/site_language_tabs.py \
  website/docs/rules website/docs/standards
uv run --no-project --python 3.14 --with cyclopts python scripts/site_nav.py \
  website/zensical.toml website/docs website/zensical.build.toml
uv run --no-project --python 3.14 --with-requirements website/requirements.txt \
  zensical build --clean --strict --config-file website/zensical.build.toml
uv run --no-project --python 3.14 --with cyclopts python scripts/site_llms.py \
  website/zensical.toml website/docs website/site/llms.txt

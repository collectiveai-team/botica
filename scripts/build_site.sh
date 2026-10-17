#!/usr/bin/env bash
# Build the standards site into website/site: export the rule pages from the catalog of this
# checkout, copy the standards of standards/ and add to each its subtitle and the list of the rules
# that implement it (scripts/site_standards.py), write a page per topic of lint/references/ and
# list the topics on the pages of the rules and the standards that cite them
# (scripts/site_references.py),
# put the code blocks of the pages of the rules and the standards in tabs by
# language (scripts/site_language_tabs.py), write the configuration with the navigation of those
# pages (scripts/site_nav.py), then run Zensical (the version is pinned in website/requirements.txt)
# in strict mode, so a broken link fails the build, and write website/site/llms.txt with the address
# of each rule, standard and topic (scripts/site_llms.py). The pages of the rules, the standards and
# the topics and the configuration with the navigation are generated, never committed.
set -euo pipefail

cd "$(dirname "$0")/.."

rm -rf website/docs/rules website/docs/standards website/docs/references website/site
cargo run --quiet --locked --manifest-path lint/Cargo.toml -p botica-cli -- \
  rules export --format markdown --output website/docs/rules
cp -r standards website/docs/standards
uv run --no-project --python 3.14 --with cyclopts python scripts/site_standards.py website/docs
uv run --no-project --python 3.14 --with cyclopts python scripts/site_references.py \
  lint/references website/docs
uv run --no-project --python 3.14 --with cyclopts python scripts/site_language_tabs.py \
  website/docs/rules website/docs/standards
uv run --no-project --python 3.14 --with cyclopts python scripts/site_nav.py \
  website/zensical.toml website/docs website/zensical.build.toml
uv run --no-project --python 3.14 --with-requirements website/requirements.txt \
  zensical build --clean --strict --config-file website/zensical.build.toml
uv run --no-project --python 3.14 --with cyclopts python scripts/site_llms.py \
  website/zensical.toml website/docs website/site/llms.txt

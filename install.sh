#!/usr/bin/env bash
#
# botica: bootstrap shim.
#
# Ensures `uv` is available, then runs the scaffolding CLI straight from git
# (no PyPI). All real work lives in the Python package; this shim only exists to
# preserve the documented one-liner and to bootstrap uv on a cold machine:
#
#   curl -fsSL https://raw.githubusercontent.com/collectiveai-team/scaffolding/main/install.sh | bash
#
# Anything after `--` (or any args) is forwarded to `scaffolding install`, e.g.:
#
#   curl -fsSL .../install.sh | bash -s -- --yes
#   curl -fsSL .../install.sh | bash -s -- ci prek --ci-parts tests,security
#
# Override the source ref with SCAFFOLDING_REF (default: main).

set -euo pipefail

REF="${SCAFFOLDING_REF:-main}"
SRC="git+https://github.com/collectiveai-team/scaffolding@${REF}"

if ! command -v uv >/dev/null 2>&1; then
  echo "uv not found — installing via https://astral.sh/uv ..." >&2
  curl -LsSf https://astral.sh/uv/install.sh | sh
  # uv installs to ~/.local/bin or ~/.cargo/bin; make it visible for this run.
  export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
fi

exec uvx --from "$SRC" scaffolding install "$@"

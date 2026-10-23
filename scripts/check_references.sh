#!/bin/sh
# Type-check and test the code of each built-in topic, `catalog/references/<topic>/code/`, with the
# versions of the `references` group of pyproject.toml. ruff checks that code with the rest of the
# repository, and the tests of botica check the `lines` and `anchor` of each `file` reference.
#
# The code of every topic is the package `app`, a namespace package, so the code of
# `structured-logging` goes on the path of every topic: the others import its `get_logger`.
#
# The environment is its own (`.tmp/references-venv` unless UV_PROJECT_ENVIRONMENT names another),
# with a CPython 3.14 that has the GIL: the house version (CES-77), and some dependencies of prefect
# have no wheels for the free-threaded build. The tests run with the network blocked by
# pytest-socket (CES-64): a test marked `integration` or `e2e` gets it back through the hook of its
# `conftest.py`. Run by the job `references` of ci.yml.
#
# A topic with no Python file skips pyrefly and pytest. A topic may also carry a Go module in
# `code/go/`, a Node package in `code/web/` and a Cargo package in `code/rust/`. The binary embeds
# every file of `code/`, so nothing is built inside it: `go vet` and `go test` leave their output
# in the cache of Go, `npm ci`, `npm test` and `npm run typecheck` run in a copy of the package
# under `.tmp/references-node/`, and cargo writes to `.tmp/references-rust/`.
set -u

export UV_PROJECT_ENVIRONMENT="${UV_PROJECT_ENVIRONMENT:-.tmp/references-venv}"
export PYDANTIC_AI_NO_BANNER=1

logger=catalog/references/structured-logging/code

# uv downloads no interpreter for a request with a variant (`+gil`); `3.14` alone is the GIL build.
uv python install --quiet 3.14 || exit 1

run() {
    uv run --quiet --locked --group references --python 3.14+gil "$@"
}

has_python() {
    [ -n "$(find "$1" -name '*.py' -print -quit)" ]
}

status=0
for code in catalog/references/*/code; do
    topic=${code%/code}
    has_python "$topic" || continue
    echo "== ${topic#catalog/references/}"
    tests=""
    [ -d "$topic/tests" ] && tests="$topic/tests"
    # shellcheck disable=SC2086 # `tests` is empty or one path
    run pyrefly check --python-version 3.14 --search-path "$code" --search-path "$logger" \
        "$code" $tests || status=1
    run pytest -q -p no:cacheprovider --disable-socket --allow-unix-socket "$topic" \
        -o "pythonpath=$code $logger" || status=1
done
for module in catalog/references/*/code/go; do
    [ -f "$module/go.mod" ] || continue
    echo "== ${module#catalog/references/}"
    (cd "$module" && go vet ./... && go test ./...) || status=1
done
for package in catalog/references/*/code/web; do
    [ -f "$package/package.json" ] || continue
    topic=${package#catalog/references/}
    copy=.tmp/references-node/${topic%%/*}
    echo "== $topic"
    rm -rf "$copy" && mkdir -p "$copy" && cp -R "$package"/. "$copy" &&
        (cd "$copy" && npm ci --no-audit --no-fund && npm test && npm run typecheck) || status=1
done
for crate in catalog/references/*/code/rust; do
    [ -f "$crate/Cargo.toml" ] || continue
    topic=${crate#catalog/references/}
    echo "== $topic"
    target=$PWD/.tmp/references-rust/${topic%%/*}
    (
        cd "$crate" &&
            cargo fmt --check &&
            cargo clippy --locked --all-targets --target-dir "$target" -- -D warnings &&
            cargo test --locked --target-dir "$target"
    ) || status=1
done
exit "$status"

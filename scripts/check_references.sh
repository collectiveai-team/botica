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
# have no wheels for the free-threaded build. Run by the job `references` of ci.yml.
set -u

export UV_PROJECT_ENVIRONMENT="${UV_PROJECT_ENVIRONMENT:-.tmp/references-venv}"
export PYDANTIC_AI_NO_BANNER=1

logger=catalog/references/structured-logging/code

# uv downloads no interpreter for a request with a variant (`+gil`); `3.14` alone is the GIL build.
uv python install --quiet 3.14 || exit 1

run() {
    uv run --quiet --locked --group references --python 3.14+gil "$@"
}

status=0
for code in catalog/references/*/code; do
    topic=${code%/code}
    echo "== ${topic#catalog/references/}"
    tests=""
    [ -d "$topic/tests" ] && tests="$topic/tests"
    # shellcheck disable=SC2086 # `tests` is empty or one path
    run pyrefly check --python-version 3.14 --search-path "$code" --search-path "$logger" \
        "$code" $tests || status=1
    run pytest -q -p no:cacheprovider "$topic" -o "pythonpath=$code $logger" || status=1
done
exit "$status"

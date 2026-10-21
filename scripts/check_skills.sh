#!/bin/sh
# Validate every skill under skills/ with agent-ecosystem/skill-validator (no account, no network
# except the first `go install`). Run by the `skill-validator` hook of prek.toml. Needs Go.
#
# The validator exits 0 when clean, 1 on errors (or on warnings with --strict) and 2 on warnings
# only. `go run` flattens every failure to 1, so it is installed and called directly.
#
# `agents/` is allowed because Codex reads `agents/openai.yaml`; the extra frontmatter fields are
# allowed because Claude and Codex read them (`disable-model-invocation`, `argument-hint`).
set -u

version=v1.6.2
bin="$PWD/.tmp/gobin"

GOBIN="$bin" go install "github.com/agent-ecosystem/skill-validator/cmd/skill-validator@$version" || exit 1

status=0
for skill in skills/*/*/; do
    "$bin/skill-validator" check --strict -o compact --allow-dirs=agents --allow-extra-frontmatter "$skill"
    case $? in
        0 | 2) ;;
        *) status=1 ;;
    esac
done
exit "$status"

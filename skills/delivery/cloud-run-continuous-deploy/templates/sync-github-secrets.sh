#!/usr/bin/env bash
# Sync every @sensitive Varlock item into one GitHub Environment.
#
# Always invoke through varlock, so values arrive as process environment
# variables and never pass through an agent, a file, or a command line:
#
#   APP_ENV=integration varlock run --inject vars -- \
#     scripts/sync-github-secrets.sh integration
#
# bash, not sh: ${!name} indirect expansion is a bashism.
set -euo pipefail

die() {
  printf 'sync-github-secrets: %s\n' "$1" >&2
  exit 2
}

ENVIRONMENT="${1:-}"
[[ -n "$ENVIRONMENT" ]] || die 'usage: sync-github-secrets.sh <dev|prod|integration>'
case "$ENVIRONMENT" in
  dev|prod|integration) ;;
  *) die "unknown environment: $ENVIRONMENT (expected dev, prod or integration)" ;;
esac

command -v gh >/dev/null 2>&1 || die 'gh CLI is required'
command -v varlock >/dev/null 2>&1 || die 'varlock is required'
command -v python3 >/dev/null 2>&1 || die 'python3 is required'
gh auth status >/dev/null 2>&1 || die 'gh is not authenticated; run: gh auth login'

# Names only. --agent redacts values, and neither flag is --format, so this stays
# outside the agent deny-glob in opencode.jsonc and no value can reach a
# transcript. .env.schema therefore remains the only list of what exists.
KEYS="$(
  varlock load --agent --filter='@sensitive' |
    python3 -c 'import json,sys; d=json.load(sys.stdin); print("\n".join((d.get("config") or d).keys()))'
)"
[[ -n "$KEYS" ]] || die 'no @sensitive items declared in .env.schema'

# The destructive environment gets suffixed names. GitHub resolves a secret
# reference environment -> repository -> organization, so a bare DATABASE_URL
# missing from the integration Environment would silently resolve to a
# repository-scoped one -- and that DSN reaches a job whose purpose is to drop a
# schema. Suffixed, a missing secret resolves empty and the workflow fails loudly.
target_name() {
  if [[ "$ENVIRONMENT" == integration ]]; then
    printf '%s_INTEGRATION' "$1"
  else
    printf '%s' "$1"
  fi
}

count=0
while IFS= read -r key; do
  [[ -n "$key" ]] || continue
  value="${!key-}"
  [[ -n "$value" ]] || die "$key is unset or empty for environment $ENVIRONMENT"
  target="$(target_name "$key")"
  # stdin, never --body: an argv value is visible in ps.
  printf '%s' "$value" | gh secret set "$target" --env "$ENVIRONMENT" >/dev/null
  printf '  %s -> %s ok\n' "$key" "$target"
  count=$((count + 1))
done <<<"$KEYS"

printf '%d secret(s) synced to the %s environment; 0 values printed.\n' "$count" "$ENVIRONMENT"

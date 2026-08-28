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
  local _sgs_msg="$1"
  printf 'sync-github-secrets: %s\n' "$_sgs_msg" >&2
  exit 2
}

_sgs_environment="${1:-}"
[[ -n "$_sgs_environment" ]] || die 'usage: sync-github-secrets.sh <dev|prod|integration>'
case "$_sgs_environment" in
  dev|prod|integration) ;;
  *) die "unknown environment: $_sgs_environment (expected dev, prod or integration)" ;;
esac

command -v gh >/dev/null 2>&1 || die 'gh CLI is required'
command -v varlock >/dev/null 2>&1 || die 'varlock is required'
command -v python3 >/dev/null 2>&1 || die 'python3 is required'
gh auth status >/dev/null 2>&1 || die 'gh is not authenticated; run: gh auth login'

# Names only. --agent redacts values, and neither flag is --format, so this stays
# outside the agent deny-glob in opencode.jsonc and no value can reach a
# transcript. .env.schema therefore remains the only list of what exists.
_sgs_keys="$(
  varlock load --agent --filter='@sensitive' |
    python3 -c 'import json,sys; d=json.load(sys.stdin); print("\n".join((d.get("config") or d).keys()))'
)"
[[ -n "$_sgs_keys" ]] || die 'no @sensitive items declared in .env.schema'

# The destructive environment gets suffixed names. GitHub resolves a secret
# reference environment -> repository -> organization, so a bare DATABASE_URL
# missing from the integration Environment would silently resolve to a
# repository-scoped one -- and that DSN reaches a job whose purpose is to drop a
# schema. Suffixed, a missing secret resolves empty and the workflow fails loudly.
_sgs_target_name() {
  local _sgs_key="$1"
  if [[ "$_sgs_environment" == integration ]]; then
    printf '%s_INTEGRATION' "$_sgs_key"
  else
    printf '%s' "$_sgs_key"
  fi
}

_sgs_count=0
while IFS= read -r _sgs_key; do
  [[ -n "$_sgs_key" ]] || continue
  _sgs_value="${!_sgs_key-}"
  [[ -n "$_sgs_value" ]] || die "$_sgs_key is unset or empty for environment $_sgs_environment"
  _sgs_target="$(_sgs_target_name "$_sgs_key")"
  # stdin, never --body: an argv value is visible in ps.
  printf '%s' "$_sgs_value" | gh secret set "$_sgs_target" --env "$_sgs_environment" >/dev/null
  printf '  %s -> %s ok\n' "$_sgs_key" "$_sgs_target"
  _sgs_count=$((_sgs_count + 1))
done <<<"$_sgs_keys"

printf '%d secret(s) synced to the %s environment; 0 values printed.\n' "$_sgs_count" "$_sgs_environment"

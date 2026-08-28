#!/usr/bin/env bash
# Register the unattended issue driver on a repository.
#
#   ./install.sh /path/to/repo [--name N] [--trigger CRON] [--timezone TZ] [--dry-run]
#
# Labels are idempotent. The automation is NOT — `orca automations create`
# always creates a new one, and two automations on one repo fight over the same
# label lock. Check `orca automations list` before re-running.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="${1:?usage: install.sh /path/to/repo [--name N] [--trigger CRON] [--dry-run]}"; shift || true

NAME="unattended issue driver"
TRIGGER="0 9-18 * * 1-5"
TZ_NAME="$(timedatectl show -p Timezone --value 2>/dev/null || echo UTC)"
DRY=0
while [ $# -gt 0 ]; do
  case "$1" in
    --name)     NAME="$2"; shift 2 ;;
    --trigger)  TRIGGER="$2"; shift 2 ;;
    --timezone) TZ_NAME="$2"; shift 2 ;;
    --dry-run)  DRY=1; shift ;;
    *) echo "unknown flag: $1" >&2; exit 2 ;;
  esac
done

REPO="$(cd "$REPO" && pwd)"; cd "$REPO"
git rev-parse --git-dir >/dev/null 2>&1 || { echo "not a git repo: $REPO" >&2; exit 1; }

REPO_NAME="$(basename "$REPO")"

# Base branch: whatever origin/HEAD points at, else the first conventional name present.
BASE="$(git symbolic-ref --quiet --short refs/remotes/origin/HEAD 2>/dev/null | sed 's#^origin/##')"
if [ -z "$BASE" ]; then
  for c in dev develop main master; do
    git show-ref --verify --quiet "refs/remotes/origin/$c" && { BASE="$c"; break; }
  done
fi
[ -n "$BASE" ] || { echo "could not determine base branch — pass it by editing PROMPT.md" >&2; exit 1; }

# Bootstrap commands, inferred from what the repo actually contains. These are a
# starting point: check them against how the project is really built.
BOOTSTRAP=""
[ -f uv.lock ] || [ -f backend/uv.lock ] && BOOTSTRAP="${BOOTSTRAP}     uv sync --dev\n"
[ -f frontend/package-lock.json ] && BOOTSTRAP="${BOOTSTRAP}     npm ci --prefix frontend\n"
[ -f package-lock.json ] && BOOTSTRAP="${BOOTSTRAP}     npm ci\n"
[ -f poetry.lock ] && BOOTSTRAP="${BOOTSTRAP}     poetry install\n"
[ -f go.mod ] && BOOTSTRAP="${BOOTSTRAP}     go mod download\n"
[ -n "$BOOTSTRAP" ] || BOOTSTRAP="     # TODO: install this project's dependencies\n"

echo "repo      : $REPO"
echo "base      : $BASE"
echo "worktrees : ../${REPO_NAME}-worktrees/issue-N"
echo "cron      : $TRIGGER  ($TZ_NAME)"
echo "bootstrap :"; printf "$BOOTSTRAP"

PROMPT="$(sed -e "s#{{BASE_BRANCH}}#$BASE#g" -e "s#{{REPO_NAME}}#$REPO_NAME#g" "$HERE/prompt.md" \
          | awk -v b="$(printf "$BOOTSTRAP")" '{gsub(/\{\{BOOTSTRAP_COMMANDS\}\}/, b)}1')"

PRECHECK='out=$(gh issue list --state open --search "label:ready-for-agent,agent-working" --json number,title,url,state,labels --limit 1000); echo "$out"; [ "$(echo "$out" | jq length)" -gt 0 ]'

if [ "$DRY" = "1" ]; then
  echo "--- dry run: nothing created ---"
  echo "$PROMPT" | head -12
  exit 0
fi

# 1. State-machine labels. `gh label create` fails when one exists; that is fine.
gh label create "ready-for-agent" --color 0E8A16 --description "Decisions settled; an agent can pick this up and build" 2>/dev/null || true
gh label create "agent-working"   --color FBCA04 --description "The driver has a run in flight on this issue"           2>/dev/null || true
gh label create "pr-ready"        --color 6F42C1 --description "Work finished and validated; PR open awaiting review"   2>/dev/null || true
gh label create "agent-blocked"   --color B60205 --description "Inspected and not buildable as specified; needs a human decision" 2>/dev/null || true
echo "labels ensured"

orca repo add --path "$REPO" >/dev/null 2>&1 || true
echo "repo registered"

orca automations create \
  --name "$NAME" --provider claude --repo "path:$REPO" \
  --trigger "$TRIGGER" --timezone "$TZ_NAME" \
  --prompt "$PROMPT" --precheck "$PRECHECK" --precheck-timeout 120 \
  --workspace-mode existing --reuse-session --disabled \
  --json | python3 -c "
import json,sys
d=json.load(sys.stdin)
if not d.get('ok'): print('FAILED:', d.get('error')); raise SystemExit(1)
print('created:', d['result']['automation']['id'], '(disabled)')
"

cat <<EOF

Created disabled on purpose. Before enabling, in $REPO:

  1. orq-lite init, then COMMIT team.json, scripts/orq-*.sh and CONVENTIONS.md.
     A worktree carries only tracked files — uncommitted, the gates do not
     exist where the work happens.
  2. Pre-seed .gitignore with exactly the rules init writes, so a re-run is a
     no-op and no worktree picks up a stray commit. Do NOT commit
     .orquestalite/packs — init materialises it from the binary.
  3. Wire lint_argv / test_argv in team.json, and prove each gate can FAIL.
     A gate that cannot go red is worse than a missing one.
  4. If the linter auto-fixes (ruff, prettier, prek), make the wrapper run it
     twice — pass one fixes, pass two is the verdict. See REFERENCE.md.
  5. orq-lite doctor reports no failures.
  6. Confirm the workspace trust dialog is already accepted for this project,
     or every session hangs on it forever without launching anything.
  7. Label one issue ready-for-agent.

Then: orca automations edit <id> --enabled
EOF

---
name: unattended-issue-driver
description: Set up an unattended agent that picks up GitHub issues on a schedule, builds them with orq-lite, and opens PRs — using issue labels as both the state machine and the mutual-exclusion lock. Use when wiring an Orca automation to a repo, when an automation claims work it should not, loops on the same issue, burns credits on empty queues, hangs without launching, or corrupts a run by sharing one checkout.
---

# Unattended issue driver

An Orca automation runs a prompt on a cron. That prompt is a **driver**: it decides what to work on, launches `orq-lite`, and later ships the result. Labels carry the state between runs, because nothing else survives a session ending.

```
ready-for-agent  ──▶  agent-working  ──▶  pr-ready
                            │
                            └──▶  agent-blocked  ──▶  (human)  ──▶  ready-for-agent
```

## The four rules that matter

1. **The claim label is the lock.** Set `agent-working` *before* creating a worktree or launching anything. Set after, and two runs claim the same issue.
2. **`agent-blocked` is not optional.** Without a blocked state, a driver that inspects an issue and can't build it must return it to `ready-for-agent` — and re-litigates the identical blocker on every run, forever. Only a human clears it.
3. **Two checkouts, two roles.** The driver checkout never switches branch and never commits. All work happens in `../<repo>-worktrees/issue-N`. One shared checkout means a run switching branches corrupts any concurrent review.
4. **A worktree gets tracked files only.** `team.json`, the gate wrappers and the conventions file must be committed, or the gates do not exist where the work happens.

## Setup

```bash
./scripts/install.sh /path/to/repo            # labels + repo registration + automation
./scripts/install.sh /path/to/repo --dry-run  # inspect first
```

Then, in the target repo — the installer prints this list and cannot do it for you:

- `orq-lite init`, then **commit** `team.json`, `scripts/orq-*.sh` and `CONVENTIONS.md`
- Pre-seed `.gitignore` with exactly the rules `init` writes, so a re-run is a no-op
- Prove each gate can fail before trusting a green one
- `orq-lite doctor` reports no failures
- Label one issue `ready-for-agent`

## The prompt

[prompt.md](prompt.md) is the driver, with `{{BASE_BRANCH}}` and `{{REPO_NAME}}` placeholders the installer substitutes. It is written to be read literally: every phase says what to do *and* what not to do, because the failure modes it guards against all looked reasonable at the time.

Two lines in an issue comment are the entire handoff between runs:

```
orq-run: <run-id>
orq-worktree: <absolute path>
```

## Do not skip the precheck guard

The precheck must **exit non-zero when the queue is empty**, or every scheduled run launches a session that discovers there is nothing to do and bills you for finding out. Orca skips the run as `skipped_precheck`.

```bash
out=$(gh issue list --state open --search "label:ready-for-agent,agent-working" \
      --json number,title,url,state,labels --limit 1000)
echo "$out"
[ "$(echo "$out" | jq length)" -gt 0 ]
```

`--label a --label b` is **AND** and silently returns nothing. Comma inside `--search` is OR.

## When something goes wrong

[reference.md](reference.md) documents the failure modes this design exists to prevent — each one observed, with the evidence trail that found it. Read it before changing any of the four rules above; most of them look like unnecessary ceremony until you hit the failure.

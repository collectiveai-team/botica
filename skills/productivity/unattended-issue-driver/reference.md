# Failure modes

Every entry here was observed in production, not imagined. The fix is in the prompt or the setup; the symptom is what you will actually see first.

---

## The driver builds an issue nobody asked for

**Symptom.** The precheck returns issue #70. The agent builds #69, which was never labelled.

**Cause.** The prompt said *"pick the spec that is more independent and ready to develop"*. That is a judgement, not a filter. The precheck result is context on the run record; nothing binds the agent to it. So the precheck was decorative — it gated *whether* the automation ran, never *what* it worked on.

**Fix.** The prompt re-queries the label itself and says, in words that cannot be reasoned around, that the label is the sole authority: an issue that looks readier but lacks the label is out of scope, and if the agent thinks otherwise it says so and stops.

**Tell.** The issue it built has no `agent-working` in its label timeline: `gh api repos/O/R/issues/N/timeline --jq '.[] | select(.event=="labeled")'`.

---

## The same blocker is re-litigated every hour

**Symptom.** An issue cycles `ready-for-agent → agent-working → ready-for-agent` on every run, with a near-identical comment each time.

**Cause.** The driver inspected the issue, correctly found it unbuildable, and had nowhere to put it. Returning it to the queue is the only other option, and guarantees the loop.

**Fix.** `agent-blocked`, cleared only by a human. The precheck deliberately does not query it, so blocked issues leave the queue entirely.

**Related.** A failed run must also not re-comment. The prompt checks for an existing identical failure comment and ends quietly — otherwise an hourly schedule leaves ten comments a day.

---

## Two flows in one working tree

**Symptom.** A review reports findings against code that is not in the diff. The PR carries unrelated changes. Governance rejects work that is actually fine.

**Cause.** `workspaceMode: existing` means every run shares one checkout. A run doing `git switch -c agent/issue-N origin/dev` changes the branch under any concurrent process. Observed verbatim from an adversary reviewer:

> the shared checkout was switched out from under this review by another process mid-run … My first probes hit the stale pre-feature code and produced a false lead.

**Fix.** The driver checkout is read-only to the agent; work happens in a per-issue `git worktree`. Do **not** solve this with Orca's `new-per-run`: that gives each *driver run* a fresh worktree, so the run that should validate the previous run's output lands somewhere that output does not exist, and the two-phase cycle never closes.

**Consequence.** A worktree receives only tracked files. Commit `team.json`, `scripts/orq-*.sh` and the conventions file, and bootstrap each worktree with `uv sync` / `npm ci` — the gates abort instantly without `.venv` and `node_modules`.

---

## `orq-lite init` leaves a stray commit in every worktree

**Symptom.** Each fresh worktree gains a `chore: orq-lite ignore rules` commit that ends up in the PR.

**Cause.** `init` commits `.gitignore` whenever the file differs from HEAD — it does not check who changed it or whether the rules are already present.

**Fix.** Commit a `.gitignore` that already contains exactly the rules `init` writes. Verified: with the rules present, a second `init` makes no commit and leaves the tree clean.

**Do not** commit `.orquestalite/packs/`. `init` materialises the pack from the binary, verified by digest, so it always matches the installed version instead of drifting against a copy in git.

---

## The lint gate fails on code it just fixed

**Symptom.** A flow dies at `gate_failed` during governance repair. Re-running the gate by hand passes.

**Cause.** `prek` exits non-zero whenever a hook **modifies** a file, not only when a check fails — and `ruff-format`, `ruff --fix`, `end-of-file-fixer` and `trailing-whitespace` all rewrite in place. A single pass gates on *"was it already formatted"*, which no freshly written code can satisfy. Inherited unformatted code on the base branch fails every branch cut from it.

**Fix.** Two passes: the first applies fixes and its exit code is discarded, the second is the verdict.

```bash
uvx prek run --all-files || true
uvx prek run --all-files
```

**Verify the trade.** Auto-fixable violations now pass silently (the code is corrected, so this is usually right). Confirm real errors still fail — a syntax error and a lint error in a non-fixing linter should both go red. Check what your type checker actually enforces: a suppressed rule or a checker that reports `warning` and exits 0 means the gate does not cover types at all.

---

## The session launches and hangs, and the run says `completed`

**Symptom.** `orca automations runs` shows `completed`. Nothing happened — no claim, no comment, no branch. The terminal log stops a few seconds in.

**Cause.** Claude Code's workspace-trust dialog. It is skipped only in non-interactive mode; an Orca pane has a TTY, so the dialog appears and waits forever. `--dangerously-skip-permissions` does **not** cover it — that is tool permissions, a different thing.

**Fix.** Set `hasTrustDialogAccepted: true` for the project in `~/.claude.json`, with no other Claude sessions running — several concurrent sessions each hold an in-memory copy and the last to exit wins, which is how the flag gets silently reset.

**Tell.** The pane log ends on `1. Yes, I trust this folder`. Look in Orca's `terminal-history/`; the run record will not tell you.

---

## Sessions pile up

Interactive `claude` does not exit when its turn ends — it sits at the prompt. Four scheduled runs meant four live sessions, the oldest 21 hours old.

Use `--reuse-session`. And note the corollary the prompt states explicitly: **a live `claude` process means nothing.** Only an `orq-lite factory` process, or the durable run status, tells you work is in flight.

---

## Things that will waste your afternoon

- **Manual runs bypass the precheck entirely** (`precheckResult: null`). You cannot test precheck behaviour with `automations run`.
- **`--precheck-timeout` requires `--precheck`** in the same command, and passing `--precheck` alone silently resets the timeout to its default.
- **`automations create` is not idempotent.** Run it twice on one repo and two automations fight over the same label lock.
- **Editing an existing-workspace automation's repo or host is refused** with `Repo updates for existing-workspace automation require workspaceMode new_per_run`. Re-pointing `--workspace` to its own current value rewrites the host binding as a side effect, which is the way through.
- **A stale `runHostId` presents as** `skipped_unavailable` with a message about remote-server scheduling. It means the automation is bound to a runtime id that no longer exists.
- **Verify prerequisites before building.** A closed decision issue is not landed work. Check that referenced files exist and that the API surface the issue is written against is the one in the tree — an issue can be a perfect specification of a codebase that does not exist yet.

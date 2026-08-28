---
name: ask-user
description: Ask which installed skill or workflow fits the user's situation.
disable-model-invocation: true
---

# Ask User

Route the user's situation to the smallest suitable workflow. Explain the recommendation briefly, then ask whether they want to invoke it. Do not invoke another user-invoked skill without confirmation.

## Main Flow

For an idea that needs to ship:

1. Use `/grill-with-docs` to sharpen an idea in a codebase while updating `CONTEXT.md` and ADRs. Use `/grill-me` when there is no codebase or durable project documentation is not wanted.
2. If a design question needs a runnable answer, use `/handoff` to branch into a fresh `/prototype` session, then `/handoff` the findings back.
3. For work that fits one session, use `/implement` directly. For multi-session work, use `/to-spec`, then `/to-tickets`, and start a fresh `/implement` session for each unblocked ticket.

`/implement` drives `/tdd` at agreed seams and closes with `/code-review`. Use `/tdd` or `/code-review` directly when only that focused workflow is needed. `/test-smell-review` applies whenever tests are written or edited; reach for it directly to audit an existing suite for tests that pass without protecting anything.

Keep grilling, specification, and ticketing in one context window when practical, so each step builds on the same thinking. Between independently implementable tickets, clear context — see Phase Boundaries.

## On-Ramps

- Incoming bugs and requests that are not already agent-ready: `/triage`.
- A hard bug, intermittent failure, or performance regression: `/diagnosing-bugs`.
- A huge, foggy effort whose path cannot fit one session: `/wayfinder`, then `/to-spec` when the decision map is clear.

Do not triage tickets produced by `/to-tickets`; they are already agent-ready. Do not send a resolved wayfinder map directly to `/implement` unless the effort proved genuinely small.

## Codebase Health

- Survey a codebase for deepening opportunities: `/improve-codebase-architecture`.
- Design a chosen module or seam using deep-module vocabulary: `/codebase-design`.
- Resolve unclear or overloaded domain terminology: `/domain-modeling`.

## Phase Boundaries

A phase is a chunk of work inside a session: the grilling, the implementation, the review. Decide what to do with the context only at the boundary between two phases; mid-phase, continue or split the remaining work into subagents. Ask these in order and take the first yes.

1. **Continue** when the next phase needs this one as a primary source, or the remaining context window comfortably fits it. Continue costs nothing and loses nothing, so rule it out first.
2. **Clear** when everything in this session is disposable to what comes next.
3. **Handoff** (`/handoff`) when something has to travel: a different harness, a different directory or worktree, another person, or a side task forked mid-phase. What it buys is portability, so skip it when nothing travels.
4. **Subagent** when the task is scoped tightly enough to run unattended and report back.
5. **Compact** otherwise. This is the default landing spot, not the first reach; pass it an instruction so the summary keeps what the next phase needs.

Every option except Continue replaces the session with a summary of it, which is why Continue is questioned first.

## Standalone

- `/prototype`: answer one state, logic, or UI design question with throwaway code.
- `/research`: delegate primary-source research and capture cited findings in the repo.
- `/grilling`: run the interview primitive directly, with no wrapper flow around it.
- `/teach`: learn a concept over multiple sessions in a stateful workspace.
- `/writing-for-agents`: guide the writing of documents agents consume — skills, `AGENTS.md`, pointed-at docs.
- `/resolving-merge-conflicts`: resolve an in-progress merge or rebase by intent.
- `/wizard`: turn a procedure only a human can perform — provisioning, credentials, CI secrets, clicking through a third-party dashboard, a one-off migration — into an interactive script that walks them through it. Not for steps an agent can perform itself.
- `/wait-what`: re-pitch the last message when it did not land. Usable mid-conversation inside any other skill.
- `/journalist`: record or search session notes under `.journals/`.
- `/unattended-issue-driver`: stand up, or debug, a scheduled agent that builds labelled issues on its own and opens PRs. Reach for it when an automation claims the wrong work, loops on one issue, or hangs without launching.

## Precondition

Use `/setup-matt-pocock-skills` once per repository before engineering flows that depend on its issue tracker, labels, or domain documentation layout.

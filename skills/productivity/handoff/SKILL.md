---
name: handoff
description: Compact the current conversation and workspace state into a handoff document for a fresh agent session.
argument-hint: "What will the next session be used for?"
disable-model-invocation: true
---

Write a handoff document summarising the current conversation and workspace state so a fresh agent can continue the work. Save it under the workspace's `.tmp/handoff/` directory and name it with the current timestamp and a slugified topic (for example, `2024-01-01T12-00-00-fix-login-bug.md`).

Include only what the next agent needs to resume confidently:

- The objective and intended outcome.
- Decisions made, constraints, and important discoveries.
- Current workspace path, worktree path when applicable, branch, and relevant git state.
- Work completed and files changed.
- Verification already run and its results.
- Remaining work, blockers, risks, and the immediate next action.
- A "Suggested skills" section naming only skills relevant to the next action.

Do not duplicate content already captured in other artifacts such as specs, plans, ADRs, issues, commits, research notes, or diffs. Reference them by path or URL and summarize only why they matter. Prefer durable source artifacts over restating conversation history.

Redact any sensitive information, such as API keys, passwords, or personally identifiable information.

When `$ARGUMENTS` is non-empty, treat it as the next session's focus and tailor the document accordingly.

After writing the file, respond with a short restart prompt that references the handoff path, states the immediate objective, and tells the next agent to inspect the referenced artifacts and current git state before editing. Do not reproduce the full handoff in chat.

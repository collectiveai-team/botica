# CES-91 · no AI co-authorship in commits

**Code:** `CES-91` &nbsp;·&nbsp; **Slug:** `no-ai-coauthorship` &nbsp;·&nbsp; **Enforced by:** prek
commit-msg hook + CI (`commit-policy.yml`) &nbsp;·&nbsp; **Tracker:**
[#91](https://github.com/collectiveai-team/scaffolding/issues/91)

## Directive

Commit messages must not carry AI co-authorship or AI-generated-attribution trailers (`Co-authored-by: Claude <...>`, `Generated with Claude Code`, `Generated-by: ChatGPT`, `AI-generated-by: Cursor`, `Assisted-by: OpenAI Codex`, and the equivalents for Copilot, Cursor, Windsurf, Aider, Gemini, Devin, OpenCode). Commit authorship reflects the human who reviewed, accepted, and submitted the change.

This does **not** prohibit using approved AI tools — it only prohibits adding AI systems as co-authors or attribution agents in Git commit metadata.

## Why

- Misleading/inconsistent authorship metadata complicates audits, `git blame`, and changelog tooling.
- Responsibility for a merged change belongs to the accountable human, not a non-human system.
- This is a metadata policy, not code-provenance detection — it does not (and cannot) infer whether AI wrote the code, only whether the commit message says so.

## Two enforcement layers

1. **Local (`no-ai-coauthorship` prek hook, `commit-msg` stage)** — immediate developer feedback; can be bypassed with `git commit --no-verify`.
2. **CI (`commit-policy.yml`)** — the source of truth. Walks every commit in the PR/push range (`git log --format=%B <base>..<head>`) and fails if any subject/body line matches the trailer pattern.

Both share one fixed, non-interpolated Python regex — no shell injection surface, and the same pattern is never duplicated with drift between the two layers.

## Disallowed vs. allowed

```text
# Disallowed
Co-authored-by: Claude <noreply@anthropic.com>
Generated with Claude Code
Generated-by: ChatGPT
AI-generated-by: Cursor
Assisted-by: OpenAI Codex

# Allowed
feat: add invoice duplicate detection rule

Implemented the blocking logic for same carrier and close ship date.
```

## Recommended tool settings

Disable automatic AI co-author attribution at the source so the hook rarely fires:

- VS Code / GitHub Copilot: `"git.addAICoAuthor": "off"`.
- Claude Code: set `attribution.commit`/`attribution.pr` to `""` and `attribution.sessionUrl` to `false`.

## Suppression

There is no per-commit opt-out beyond fixing the message — this is a hard policy gate, matching CES-75's Conventional Commits enforcement shape. If a genuinely non-AI commit trips the pattern (a human named "Claude" co-authoring, for example), rephrase the trailer to avoid the matched tool name.

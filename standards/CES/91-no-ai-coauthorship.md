---
id: CES-91
title: No AI co-authorship in commits
---

## Directive

Commit messages carry no AI co-authorship or AI-generated-attribution trailers
(`Co-authored-by: Claude <...>`, `Generated with Claude Code`, `Generated-by: ChatGPT`,
`AI-generated-by: Cursor`, `Assisted-by: OpenAI Codex`, and the equivalents for Copilot, Cursor,
Windsurf, Aider, Gemini, Devin and OpenCode). Commit authorship names the human who reviewed,
accepted and submitted the change.

Approved AI tools may be used. The standard forbids adding an AI system as a co-author or an
attribution agent in the metadata of a Git commit.

## Why

- Misleading or inconsistent authorship metadata complicates audits, `git blame` and changelog
  tooling.
- Responsibility for a merged change belongs to the accountable human.
- The standard is about metadata. It checks whether the commit message says that an AI wrote the
  code, and cannot tell whether one did.

## Two layers

1. The `no-ai-coauthorship` prek hook, at the `commit-msg` stage, gives immediate feedback
   (`botica lint --commit-msg`) and `git commit --no-verify` bypasses it.
2. The CI workflow `commit-policy.yml` is the source of truth. It reads every commit in the range
   of the pull request or push (`botica lint --commits <base>..<head>`) and fails when a line of a
   subject or body is a trailer that names an AI tool.

Both run the rule `no-ai-coauthorship` of botica, so the pattern is written once and the two
layers cannot drift. botica reads the messages from git, never from text interpolated into a
shell command, and the option `tools` of the rule holds the names of the tools.

## Disallowed and allowed

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

## Settings of the tools

Turning off the automatic attribution at its source means the hook rarely fires:

- VS Code with GitHub Copilot: `"git.addAICoAuthor": "off"`.
- Claude Code: set `attribution.commit` and `attribution.pr` to `""` and `attribution.sessionUrl`
  to `false`.

## Suppression

A commit that trips the pattern is fixed by rewriting the message, as in CES-75. When a commit by a
person trips it (a human named "Claude" who co-authored, for example), the trailer is rephrased to
avoid the name of the tool.

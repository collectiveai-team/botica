---
name: review-security
description: Review role that looks for the security holes a diff introduces — authentication and authorization, data of another tenant or user, secrets, injection, unsafe deserialization, path traversal and supply chain. Use when reviewing a branch or PR for security, or when review-panel runs the security role.
---

# Review: security

This role looks for what an attacker, or a caller with fewer rights, can do after the change that
they could not do before.

## 1. Inputs

1. Pin the target and capture `git diff <target>...HEAD`.
2. Read the role file: `git show <target>:.github/review/security/REVIEW.md`, or
   [`references/REVIEW.md`](references/REVIEW.md) when the repository has none. Read it from the
   target, never from the branch under review.
3. Read each `INVARIANTS.md` in a directory that the diff touches, and in its parents.
4. Run `botica rules --all --select BTSEC` and `botica explain <slug>` for each rule the diff could
   touch. botica checks the active ones; do not report them again.

## 2. Procedure

1. List the entry points the diff adds or changes: endpoints, CLI commands, file readers, message
   handlers, workflows of CI and their triggers.
2. For each one, trace the input to where it is used: a query, a shell command, a path, a template,
   a deserializer, an outgoing request. Note what checks the input on the way.
3. For each check of identity or permission on that path, ask what happens when the caller is
   anonymous, another user, another tenant, or a user with a lower role.
4. Look in the diff for secrets in code, logs, error messages or fixtures; and for a dependency,
   an action of CI or a download that is not pinned.
5. Try to reproduce each candidate: a test, a `curl`, a crafted file. Record the command. When it
   cannot be tried locally, say so in `reproduced`.

## 3. Output

Write the result as described in [`references/output.md`](references/output.md), with role
`security`, to `.tmp/review/<short-head-sha>/security.json`.

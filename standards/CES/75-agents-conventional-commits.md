---
id: CES-75
title: Conventional Commits
---

## Directive

Every commit subject follows [Conventional Commits](https://www.conventionalcommits.org):

```
<type>(<optional scope>)<optional !>: <description>
```

A local `commit-msg` prek hook validates the message at commit time, and a CI workflow validates
the title of a pull request. Both embed `CES-75`.

## Allowed types

| Type | For |
|---|---|
| `feat` | a new feature |
| `fix` | a bug fix |
| `docs` | documentation only |
| `style` | formatting, no change in what the code means |
| `refactor` | a code change that neither fixes a bug nor adds a feature |
| `perf` | a performance improvement |
| `test` | adding or fixing tests |
| `build` | the build system or dependencies |
| `ci` | CI configuration |
| `chore` | maintenance, no change to source or tests |
| `revert` | reverts a previous commit |

- The scope is optional, lowercase, in parentheses: `feat(api): …`.
- A breaking change appends `!` before the colon (`feat!: …`), a `BREAKING CHANGE:` footer, or both.
- `Merge …`, `Revert …` and `fixup! …` subjects are exempt from the hook.

## Why

- A machine-readable history makes changelogs, release automation (semver bumps) and `git log`
  triage deterministic.
- The type and scope prefix forces a one-line statement of what kind of change a commit is, which
  keeps commits small and single-purpose.

## Examples

```text
feat(api): add the check for duplicate invoices
fix(parser): keep the trailing newline of a block
docs: explain the slug and the code of a standard
```

## Suppression

The hook reads the subject line only and exempts merge, revert and fixup commits. Any other
subject is fixed and not exempted.

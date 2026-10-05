# CES-75 · Conventional Commits

**Code:** `CES-75` &nbsp;·&nbsp; **Slug:** `agents-conventional-commits` &nbsp;·&nbsp; **Enforced
by:** prek commit-msg hook + CI PR-title check &nbsp;·&nbsp; **Tracker:**
[#75](https://github.com/collectiveai-team/botica/issues/75)

## Directive

Every commit subject follows [Conventional Commits](https://www.conventionalcommits.org):

```
<type>(<optional scope>)<optional !>: <description>
```

A local `commit-msg` prek hook validates the message at commit time; a CI workflow validates the
**PR title** on pull requests. Both embed `CES-75`.

## Allowed types

| Type | For |
|---|---|
| `feat` | a new feature |
| `fix` | a bug fix |
| `docs` | documentation only |
| `style` | formatting, no code-meaning change |
| `refactor` | code change that neither fixes a bug nor adds a feature |
| `perf` | a performance improvement |
| `test` | adding or fixing tests |
| `build` | build system or dependencies |
| `ci` | CI configuration |
| `chore` | maintenance, no src/test change |
| `revert` | reverts a previous commit |

- **Scope** (optional) is lowercase in parentheses: `feat(api): …`.
- **Breaking change**: append `!` before the colon (`feat!: …`) and/or a `BREAKING CHANGE:`
  footer.
- `Merge …`, `Revert …`, and `fixup! …` subjects are exempt from the hook.

## Why

- A machine-readable history makes changelogs, release automation (semver bumps), and `git log`
  triage deterministic instead of guesswork.
- The type/scope prefix forces a one-line statement of *what kind of change* this is, which
  keeps commits small and single-purpose.

## Examples

```text
feat(standards): ship the logging family (#81)
fix(engine): defer existing AGENTS.md instead of overwriting
docs: clarify the CES slug-vs-code convention
```

## Suppression

The hook keys on the subject line only and exempts merge/revert/fixup commits. There is no
per-commit opt-out beyond those — fix the subject instead.

## Directive

The subject of each commit message, its first line, is `<type>(<scope>)!: <description>`: a type of
the option `types`, a scope in lowercase letters, digits, `.`, `_` and `-` between parentheses, a
`!` for a breaking change, `: ` and the description. The scope and the `!` are optional. A subject
that starts with an entry of `exempt` is not checked. An error by default.

The rule reads messages, never files: `botica lint --commit-msg <file>` in a `commit-msg` hook,
`botica lint --commits <range>` in CI, and `botica lint --pr-title="$PR_TITLE"` for the title of a
pull request, which a squash merge takes as the subject.

## Why

A history whose subjects say the kind of each change lets a release tool pick the next version and
write the changelog, and lets a reader find the fixes and the breaking changes in `git log`.

## Do

```text
feat(api): add the endpoint of orders
fix!: drop the flag --legacy
```

## Don't

```text
Add the endpoint of orders
feature(API): add the endpoint of orders
```

## Options

```toml
[rules.conventional-commits]
types = ["feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore", "revert"]
exempt = ["Merge ", "Revert ", "fixup! "]
```

## Suppression

A message has no comments that botica reads, so only the configuration silences the rule:
`ignore = ["conventional-commits"]`, or a subject start added to `exempt`.

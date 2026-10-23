## Directive

uv resolves only the files that were uploaded to the index at least 7 days ago. `pyproject.toml`
sets `exclude-newer = "7 days"` in `[tool.uv]`, and `uv add`, `uv lock` and `uv sync` then treat a
newer file as if it did not exist, for the direct and the transitive dependencies.

## Why

A malicious release has no advisory during its first days, and `osv-scanner` reports it only once
the advisory exists. The cooldown of Dependabot holds back the dependency that its pull request
updates, but `uv lock` moves the transitive ones to their newest versions, and nothing holds back
`uv add` run by a person or an agent.

## Exceptions

One package that needs a newer release, such as the fix of an advisory, gets an upload date of its
own in `exclude-newer-package`, with the reason and the date from which it can go:

```toml
[tool.uv]
exclude-newer = "7 days"
# GHSA-xxxx-xxxx-xxxx: requests 2.33.1 fixes it. Remove on 2026-10-28.
exclude-newer-package = { requests = "2026-10-21T00:00:00Z" }
```

A date lets in only what was uploaded until then; `false` would let in every later release of the
package. Removing the entry before the release is 7 days old makes the next `uv lock` go back to an
older version.

## Tool

[uv](https://docs.astral.sh/uv/concepts/resolution/#dependency-cooldowns) enforces the rule: a
duration in `exclude-newer` (since uv 0.9.17) is a cutoff relative to the time of the resolution,
and `uv.lock` records it as `exclude-newer-span`. Botica checks that the settings of uv have it, in
`uv.toml` or in `[tool.uv]` of `pyproject.toml`, the file that uv reads.

## Configuration

```toml
[tool.uv]
exclude-newer = "7 days"
```

Any span that uv reads holds the rule when it is 7 days or longer: `"1 week"`, `"P7D"` or
`"14 days"`. A shorter span, a date or `false` is a value the project wrote on purpose, and
botica does not report it. botica asks for the setting only in a project with `uv.lock`.

## Do

```toml
[tool.uv]
exclude-newer = "7 days"
```

## Don't

```toml
[tool.uv]
package = false
```

## Suppression

The finding is about `pyproject.toml` as a whole. Switch the rule off with
`ignore = ["uv-cooldown"]` in the configuration of botica.

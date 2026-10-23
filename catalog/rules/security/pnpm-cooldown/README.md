## Directive

pnpm resolves only the versions that were published at least 7 days ago. `pnpm-workspace.yaml`
sets `minimumReleaseAge: 10080` (minutes), and `pnpm add`, `pnpm install` and `pnpm update` then
skip a newer version, for the direct and the transitive dependencies.

## Why

A malicious release has no advisory during its first days, and `osv-scanner` reports it only once
the advisory exists. The cooldown of Dependabot holds back the dependency that its pull request
updates, but not the transitive ones, and nothing holds back a package added by a person or an
agent.

## Exceptions

One package that needs a newer release, such as the fix of an advisory, gets an exception with the
reason and the date from which it can go:

```yaml
minimumReleaseAge: 10080
# GHSA-xxxx-xxxx-xxxx: left-pad 1.3.1 fixes it. Remove on 2026-10-28.
minimumReleaseAgeExclude:
  - left-pad@1.3.1
```

## Tool

[pnpm](https://pnpm.io/settings/dependency-resolution#minimumreleaseage) enforces the rule (since
pnpm 10.16; pnpm 11 sets 1440 minutes by default). pnpm reads it from `pnpm-workspace.yaml` and
not from `.npmrc`. Botica checks that `pnpm-workspace.yaml` has it.

## Configuration

```yaml
minimumReleaseAge: 10080
```

Any number of minutes of 10080 or more holds the rule. A smaller number, `0` too, is a value the
project wrote on purpose, and botica does not report it. botica asks for the setting only in a
project with `pnpm-lock.yaml`.

## Do

```yaml
minimumReleaseAge: 10080
```

## Don't

```yaml
minimumReleaseAge: 1440
```

## Suppression

The finding is about the file as a whole. Switch the rule off with
`ignore = ["pnpm-cooldown"]` in the configuration of botica.

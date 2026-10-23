## Directive

Yarn installs only the versions that were published at least 7 days ago. `.yarnrc.yml` sets
`npmMinimalAgeGate: 7d`, and `yarn add`, `yarn install` and `yarn up` then skip a newer version.

## Why

A malicious release has no advisory during its first days, and `osv-scanner` reports it only once
the advisory exists. The cooldown of Dependabot holds back the dependency that its pull request
updates, but not the transitive ones, and nothing holds back a package added by a person or an
agent.

## Exceptions

One package that needs a newer release, such as the fix of an advisory, gets an exception with the
reason and the date from which it can go:

```yaml
npmMinimalAgeGate: 7d
# GHSA-xxxx-xxxx-xxxx: left-pad 1.3.1 fixes it. Remove on 2026-10-28.
npmPreapprovedPackages:
  - left-pad@1.3.1
```

## Tool

[Yarn](https://yarnpkg.com/configuration/yarnrc#npmMinimalAgeGate) enforces the rule (since Yarn
4.10). Botica checks that `.yarnrc.yml` has it.

## Configuration

```yaml
npmMinimalAgeGate: 7d
```

Any span of 7 days or more holds the rule: `1w`, `7d`, or `10080`, a number of minutes. A shorter
span, `0` too, is a value the project wrote on purpose, and botica does not report it. botica asks
for the setting only in a project with `yarn.lock`.

## Do

```yaml
npmMinimalAgeGate: 7d
```

## Don't

```yaml
npmMinimalAgeGate: 1d
```

## Suppression

The finding is about the file as a whole. Switch the rule off with
`ignore = ["yarn-cooldown"]` in the configuration of botica.

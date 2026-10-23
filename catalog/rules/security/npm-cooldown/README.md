## Directive

npm installs only the versions that were published at least 7 days ago. The `.npmrc` of the
project sets `min-release-age=7` (days), and `npm install` and `npm update` then build the tree from
older versions only.

## Why

A malicious release has no advisory during its first days, and `osv-scanner` reports it only once
the advisory exists. The cooldown of Dependabot holds back the dependency that its pull request
updates, but not the transitive ones, and nothing holds back a package added by a person or an
agent.

## Exceptions

One package that needs a newer release, such as the fix of an advisory, gets an exception with the
reason and the date from which it can go:

```ini
min-release-age=7
; GHSA-xxxx-xxxx-xxxx: left-pad 1.3.1 fixes it. Remove on 2026-10-28.
min-release-age-exclude[]=left-pad
```

`min-release-age-exclude` (npm 11.17) takes a name or a glob and no version, so it lets in every
release of the package until it is removed.

## Tool

[npm](https://docs.npmjs.com/cli/v11/using-npm/config#min-release-age) enforces the rule (since npm
11.10). Botica reads the `.npmrc` at the root of the project; the `.npmrc` of the user and the
global one are not seen.

## Configuration

```ini
min-release-age=7
```

Any number of days of 7 or more holds the rule. A smaller number, `0` too, is a value the project
wrote on purpose, and botica does not report it. botica asks for the setting only in a project
with `package-lock.json`.

## Do

```ini
min-release-age=7
```

## Don't

```ini
registry=https://registry.npmjs.org/
```

## Suppression

The finding is about the file as a whole. Switch the rule off with
`ignore = ["npm-cooldown"]` in the configuration of botica.

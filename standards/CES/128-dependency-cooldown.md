---
id: CES-128
title: Dependencies wait seven days after their release
---

## Directive

A project resolves a version of a dependency from a registry only once that version has been
published for 7 days. The package manager enforces it through a setting in a file that the
repository commits, so that it holds for `add`, `lock` and `update`, for a person, an agent and CI
alike, and for the transitive dependencies too. Each entry of `updates` in `.github/dependabot.yml`
has `cooldown` with `default-days: 7` or more.

| Manager | File | Setting | Since |
| --- | --- | --- | --- |
| uv | `pyproject.toml` (or `uv.toml`) | `[tool.uv]` `exclude-newer = "7 days"` | uv 0.9.17 |
| pnpm | `pnpm-workspace.yaml` | `minimumReleaseAge: 10080` (minutes) | pnpm 10.16 |
| npm | `.npmrc` | `min-release-age=7` (days) | npm 11.10 |
| Yarn | `.yarnrc.yml` | `npmMinimalAgeGate: "1w"` | Yarn 4.10 |
| Bun | `bunfig.toml` | `[install]` `minimumReleaseAge = 604800` (seconds) | Bun 1.3 |
| Cargo | `.cargo/config.toml` | `[registry]` `global-min-publish-age = "7 days"` | Rust 1.100 |
| Go | none | see below | |

Go has no such setting: the proposal for one, golang/go#76485, is on hold. Go selects the minimum
version that satisfies `go.mod`, so it takes a new release only when it is asked for one. A Go
project adds or raises a dependency with an explicit version that is 7 days old
(`go get example.com/mod@v1.4.2`), not with `@latest` or `go get -u`, and Dependabot raises the
rest with its cooldown.

A Rust project whose toolchain is older than 1.100 relies on the cooldown of Dependabot until it
moves to 1.100.

## Why

`osv-scanner` (CES-119) and the dependency review (CES-113) react to an advisory that exists. A
malicious release has none during its first days, and a resolver takes it as soon as it is the
newest version that fits. The window of 7 days gives the registry and the community that time
before the project can resolve the release. It detects nothing by itself.

The cooldown of Dependabot alone is not enough. It holds back the dependency that a pull request
updates, but `uv lock` moves the transitive dependencies to their newest versions with no age
limit (dependabot-core#13816), and nothing holds back `uv add` or `pnpm add` on a laptop. The
setting of the package manager covers every resolution.

`osv-scanner` already reports the `MAL-` advisories of the OpenSSF feed of malicious packages, so a
project adds no second scanner of malicious packages.

## Exceptions

An exception names one package, carries a comment with the reason (the advisory, or the release
that the project needs) and the date from which it can go, and is removed on that date. The
exception for uv is an upload date and not `false`: `false` lets in every later release of the
package, and a date lets in only what was published until then.

| Manager | Exception for one package |
| --- | --- |
| uv | `exclude-newer-package = { requests = "2026-10-21T00:00:00Z" }` |
| pnpm | `minimumReleaseAgeExclude: ["requests@2.33.1"]` (an exact version) |
| npm | `min-release-age-exclude[]=requests` (a name or a glob; npm 11.17) |
| Yarn | `npmPreapprovedPackages: ["requests@2.33.1"]` |
| Bun | `minimumReleaseAgeExcludes = ["requests@2.33.1"]` |
| Cargo | none in the configuration: `CARGO_RESOLVER_INCOMPATIBLE_PUBLISH_AGE=allow cargo update -p requests --precise 2.33.1` |

Cargo keeps a version that `Cargo.lock` already has, so the lockfile is the exception. uv does not:
removing the exception before the version is 7 days old makes the next `uv lock` go back to an
older version.

## Security updates

The cooldown of Dependabot does not apply to its security updates. For npm, pnpm and Yarn,
Dependabot also turns off the setting of the package manager on a security update. For uv it does
not, so a security update whose fix is younger than 7 days fails to resolve and Dependabot opens
no pull request. A person then opens it:

1. Add the exception for the package, with the upload time of the fixed version and the advisory
   in the comment.
2. Run `uv lock --upgrade-package <package>` and open the pull request.
3. Remove the exception once the fixed version is 7 days old, and run `uv lock`.

## Migration (existing repos)

Add the setting. A lockfile that already holds a version younger than the window fails
`uv lock --check` once; `uv lock` fixes it. A dependency pinned to a release younger than the window
does not resolve until the release is old enough, so it gets an exception, with the date from which
it can go, in the same pull request.

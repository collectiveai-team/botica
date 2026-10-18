---
id: CES-114
title: Dockerfile lint
---

## Directive

A `Dockerfile` at the root of a repo passes [`hadolint`](https://github.com/hadolint/hadolint). It
reports unpinned base images, a missing `--no-install-recommends`, `ADD` where `COPY` belongs,
running as root, and similar problems.

## Why

Scanners such as `trivy` and `grype` work on the built image. `hadolint` checks the text of the
Dockerfile itself: it is deterministic, it runs as a prek hook, and it needs no build.

## Scope

The hook is scoped with `files = "^Dockerfile"`. It runs only when a Dockerfile exists at the root
of the repo and does nothing in a repo without one, which is the placement-scoping of `repo-shape`
(CES-32).

## License

`hadolint` is GPL-3.0 and runs as an external command through prek, the boundary that every other
third-party tool of this stack has (ruff, betterleaks), so it puts no copyleft obligation on the
source of the repo.

## Migration (existing repos)

Run the hook once on an existing Dockerfile. Fix the findings, or ignore them explicitly with
`# hadolint ignore=DLxxxx`. The posture is the one of `file-size-guard` (CES-71): flag, and do not
exempt silently. Some default rules, such as the one that requires a specific `HEALTHCHECK`, are
opinionated enough that the first rollout may need an ignore list.

## Suppression

Inline, with `# hadolint ignore=DLxxxx` on the line, or in a `.hadolint.yaml` ignore list for the
project. The hook is not disabled globally to silence one rule.

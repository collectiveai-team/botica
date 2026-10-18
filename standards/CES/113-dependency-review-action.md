---
id: CES-113
title: Dependency review on pull requests
---

## Directive

Every pull request that changes the dependency graph gets an automatic summary, visible in the
pull request, of what was added, removed or changed. The summary comes from
[`actions/dependency-review-action`](https://github.com/actions/dependency-review-action) and runs
as the workflow `dependency-review.yml`.

## Why

`pip-audit` and `osv-scanner` scan the resolved dependency set for known CVEs. Neither shows what
changed in a pull request against the base branch, so a reviewer finds a new dependency by reading
the diff of `uv.lock` line by line. `dependency-review-action` shows it in the pull request. It
addresses dependency creep: an agent that adds a library for one HTTP call where the standard
library or an existing dependency would do.

## Posture

The workflow is summary-only. It sets `comment-summary-in-pr: always`, sets neither
`fail-on-severity` nor `deny-licenses`, and sets `continue-on-error: true`, so it cannot block a
merge. Making it a gate is a separate decision, taken after a trial period.

## Public repositories

The documentation of GitHub says that dependency review is "available for all public repositories,
as well as private repositories that have GitHub Code Security or GitHub Advanced Security
enabled". A private or internal repository adds the workflow by hand when its plan includes one of
the two.

## Requirements

- The repo runs on GitHub-hosted runners and has the dependency graph enabled (Settings, Security,
  Advanced Security, Dependency Graph). The setting is free and is only available in the web UI:
  neither the REST API nor GraphQL exposes it, so `gh` cannot set it.
- When the dependency graph is not available, for example in an organization that turned it off
  for a public repo, the action does not skip quietly. It fails the step with "Dependency review
  is not supported on this repository." That is why the workflow sets `continue-on-error: true`:
  without it, a gap in the infrastructure would block merges, against the posture above.

## Suppression

The summary-only workflow only comments and fails no check, so there is nothing to suppress. Once a
repo makes it a gate, the `allow-ghsas` and `allow-licenses` inputs of the action allow a single
advisory or license.

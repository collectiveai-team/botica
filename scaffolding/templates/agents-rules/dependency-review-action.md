# CES-113 · dependency-review-action

**Code:** `CES-113` &nbsp;·&nbsp; **Slug:** `dependency-review-action` &nbsp;·&nbsp; **Enforced
by:** CI workflow (`dependency-review.yml`) &nbsp;·&nbsp; **Tracker:**
[#113](https://github.com/collectiveai-team/scaffolding/issues/113)

## Directive

Every PR that changes the dependency graph gets an automatic, PR-visible summary of what was
added/removed/changed, via
[`actions/dependency-review-action`](https://github.com/actions/dependency-review-action).

## Why

`pip-audit`/`osv-scanner` scan the *final resolved* dependency set for known CVEs — neither
highlights *what changed in this PR* versus the base branch, so a reviewer has to notice a new
dependency by reading `uv.lock`'s diff line by line. `dependency-review-action` is a direct,
GitHub-native fix for the "dependency creep" failure mode: an agent reaching for a new library to
make one HTTP call where the stdlib or an existing transitive dep would do.

## Enforcement posture

Ships **summary-only**: `comment-summary-in-pr: always`, no `fail-on-severity` or
`deny-licenses`, **plus `continue-on-error: true`**. It cannot block a merge in this
configuration — promote to a hard gate only after a burn-in period, as a separate, explicit
decision.

## Tier-gated: public repos only by default

`dependency-review-action` is free for **public** repos but requires **GitHub Advanced Security /
Code Security (a paid tier)** on private/internal repos — confirmed directly from GitHub's own
docs: *"available for all public repositories, as well as private repositories that have GitHub
Code Security or GitHub Advanced Security enabled."* This tool is tier-agnostic (works on free
public **and** free private repos without assuming a paid upgrade), so the `ci` component only
ships `dependency-review.yml` when `facts.visibility` is `public` — private/internal repos get a
`[skip]` notice instead, the same posture `docker.yml` already uses for GHCR billing. Add it by
hand if your org's plan includes Advanced Security/Code Security.

## Requirements (public repos)

GitHub-hosted, public, and the repo (or org) needs "Dependency graph" enabled (Settings >
Security > Advanced Security > Dependency Graph — free, but a **web-UI-only toggle**: there is no
REST or GraphQL field for it, confirmed via the current `PATCH /repos/{owner}/{repo}` schema and
a GraphQL `Repository` type introspection — `gh` CLI can't set it either, since it only wraps
those APIs). Unlike most GitHub security features, the action does **not** no-op quietly when
Dependency graph is unavailable (e.g. an org that has it switched off even on a public repo) — it
hard-fails the step with "Dependency review is not supported on this repository." That is exactly
why `continue-on-error: true` is part of the shipped default, not optional polish: without it, an
infra gap (not a real dependency finding) would silently violate this standard's own "can't block
merges" promise. Verified directly against a real repo where the org has the feature disabled.

## Suppression

There is nothing to suppress in the summary-only default — it only comments, never fails a check.
Once (and if) a repo promotes this to a hard gate, per-advisory allow-listing is the
`allow-ghsas`/`allow-licenses` inputs on the action itself, not a house-side mechanism.

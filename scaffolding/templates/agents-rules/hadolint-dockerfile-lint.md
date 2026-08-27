# CES-114 · Dockerfile lint (hadolint)

**Code:** `CES-114` &nbsp;·&nbsp; **Slug:** `hadolint-dockerfile-lint` &nbsp;·&nbsp; **Enforced
by:** prek hook (`hadolint-docker`, file-pattern-scoped) &nbsp;·&nbsp; **Tracker:**
[#114](https://github.com/collectiveai-team/scaffolding/issues/114)

## Directive

Any `Dockerfile`/`Dockerfile.*` in a repo with the `docker` CI component enabled must pass
[`hadolint`](https://github.com/hadolint/hadolint) (unpinned base images, missing
`--no-install-recommends`, `ADD` vs `COPY` misuse, running as root, etc).

## Why

Scaffolding already ships a `docker.yml` CI workflow template (opt-in) but previously zero
Dockerfile-source linting. `trivy`/`grype`-style scanners operate on the *built image*;
`hadolint` is the lightweight, deterministic check on the Dockerfile *text itself*, and is
prek-native (fast, no build step).

## Scope

`files = "^Dockerfile"` — the hook only ever runs when a Dockerfile exists at the repo root, same
placement-scoping posture as `repo-shape` (CES-32). It is inert (never invoked) in a repo without
one, so it ships unconditionally in `prek-generic.toml` rather than needing component-level
gating.

## License note

`hadolint` is GPL-3.0, invoked purely as an external CLI/subprocess via prek — the same boundary
already used for every other third-party tool in this stack (ruff, ast-grep, betterleaks) — so
this creates no copyleft obligation on a repo's own (typically MIT) source.

## Migration (existing repos)

New repos with `docker` enabled get it by default. Existing repos with a Dockerfile: run once, fix
or explicitly ignore (`# hadolint ignore=DLxxxx`) findings — the same "flag, don't silently
exempt" posture as `file-size-guard` (CES-71). Some default hadolint rules (e.g. requiring a
specific `HEALTHCHECK`) are opinionated enough that a house ignore-list may be needed on first
rollout.

## Suppression

Inline `# hadolint ignore=DLxxxx` on the offending line, or a project-level `.hadolint.yaml`
ignore list — never disable the hook globally to silence one rule.

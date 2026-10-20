## Engineering Standards

> **Retrieval-led, not training-led.** Prefer this repo's standards and references over
> training-default patterns. Before writing code that touches a rule below, run
> `uvx botica refs show <rule or CES-number>` and follow the house pattern rather than your default
> one.

Each rule is a **CES — Collective Engineering Standard** (`CES-<number>` is the citable code;
the kebab-case slug names the file of the standard). `[prek]` rules are enforced automatically by
`prek`; `[script]` rules are kept by a command of botica or of a tool (`npx skills`); `[ci]` rules
are enforced by a GitHub Actions workflow (no local hook); `[dependency]` rules ship as a
dev-dependency that activates automatically, no separate check; `[judgment]` rules are
reviewer/agent judgment; `[reference]` points at canonical code that `botica refs show` prints. Full convention:
`docs/engineering-standards.md`.

The rules that `botica lint` measures are not listed here and have no detail file: the catalog
of botica is their only documentation. `uvx botica rules` lists them, `uvx botica explain <slug>`
describes one, and the hook reports each finding with its slug and its message.

### Standards

- **CES-107 · track the skills manifest** `[script]` — commit `skills-lock.json`; `.agents/skills/`
  is derived from it and stays gitignored. Install restores from the manifest. It pins no version
  and verifies no hash, so call it a manifest, not a lock. Slug: `skills-manifest`.
- **CES-74 · the house logger** `[reference]` — `uvx botica refs show structured-logging` prints the
  canonical structlog setup (JSON in prod, colored console in dev, level from `LOG_LEVEL`). Copy it
  to `<pkg>/core/logger.py`. Slug: `core-logger`.
- **CES-75 · Conventional Commits** `[prek]` — commit subjects follow `type(scope): description`;
  a commit-msg hook checks every commit and a CI workflow checks the PR title. Slug:
  `agents-conventional-commits`.
- **CES-77 · version pin** `[judgment]` — `requires-python` ships as a non-enforced comment
  (house default 3.14); an existing repo's pin wins. → see `pyproject.toml` comment.
- **CES-91 · no AI co-authorship in commits** `[prek]` `[ci]` — commit messages must not carry
  `Co-authored-by:`/`Generated with`/etc AI-attribution trailers; a commit-msg hook gives local
  feedback, `commit-policy.yml` is the CI source of truth. Slug: `no-ai-coauthorship`.
- **CES-111 · randomize test order** `[dependency]` — `pytest-randomly` runs every test suite in
  a random, reproducibly-seeded order to catch state-leaking/order-dependent tests; activates
  automatically, no separate hook. Slug: `test-order-randomization-pytest-randomly`.
- **CES-113 · dependency review on PRs** `[ci]` — `dependency-review-action` comments a
  new-dependency/license/vulnerability summary on every PR that changes the dependency graph;
  summary-only, not a merge gate. Slug: `dependency-review-action`.
- **CES-114 · Dockerfile lint** `[prek]` — `hadolint`, scoped to `^Dockerfile`; inert unless a
  Dockerfile is present. Slug: `hadolint-dockerfile-lint`.
- **CES-118 · no copy-paste duplication** `[prek]` — near-verbatim duplicated code blocks above a
  token threshold fail the build, via `jscpd` (polyglot, respects `.gitignore`). Slug:
  `code-duplication-jscpd`.
- **CES-119 · dependency vulnerability scanning** `[ci]` — `osv-scanner` scans `uv.lock` (and any
  other ecosystem lockfile) against the OSV.dev database on every push/PR/weekly schedule;
  replaces `pip-audit.yml`. Slug: `osv-scanner-replace-pip-audit`.

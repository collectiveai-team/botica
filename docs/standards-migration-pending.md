# Pending: normalising the migrated CES

The 20 standards that `3cba995` moved from the details of the scaffolder to `standards/CES/` still
have the format and the voice of the scaffolder. Only what needs no new definition was fixed:
`PR` is now `pull request` (the literal names of a tool, such as `comment-summary-in-pr` or
`attribution.pr`, stay), and the hand-written `## Rules` lists of the other 11 are gone, because the
site now builds that list from the `standard` of each rule (`scripts/site_standards.py`). The rest
waits for the decisions below. Nothing here has been deleted.

## Decisions that are missing

1. **How a standard says what enforces it when no botica rule does.** Today it is a loose first
   line (`**Enforced by:**`, `**Tier:**`, `**Ships as:**`), and the index of `AGENTS.md` marks the
   same fact by hand (`[prek]`, `[judgment]`, `[snippet]`). `standards/CES/index.md` defines a
   standard as text, `## Why` and `## Examples`, and says nothing about this. A field of the front
   matter, a section or nothing are all possible, and ADR 0004 fixes `id` and `title` only.
2. **Where the drop-in code lives.** `.agents/snippets/` is a payload of the scaffolder and does not
   exist in this repository. CES-64 and CES-74 point at it.
3. **Where the knowledge of a tool goes.** The sections *Migration*, *Suppression*,
   *Configuration* and *Requirements* explain how to adopt `deptry`, `complexipy`, `jscpd`,
   `osv-scanner`, `hadolint` and `dependency-review-action`. They are not the standard, and with
   the scaffolder gone they are the only place that knowledge survives.
4. **Whether CES-107, CES-113 and CES-114 survive the scaffolder.** Each describes behaviour of the
   scaffolder (what `scaffolding check` verifies, which component ships a workflow, when a hook is
   shipped). What remains once that is removed is a shorter standard, and CES-107 may be left with
   only its first paragraph.
5. **The title convention.** Some titles carry the tool in parentheses (`Dependency hygiene
   (deptry)`, `The house logger (core/logger.py)`), others do not. The sidebar now shows the
   title, so the convention shows.

## The smells

| Code | Smell |
|---|---|
| S1 | Loose first line, `Enforced by` / `Tier` / `Ships as` (decision 1) |
| S2 | Voice of the scaffolder: names `scaffolding`, its components, `install` / `check`, "this tool", "shipped", "seed", "clean-adds-only" |
| S3 | Reference to a file that is not in this repository: `.agents/snippets/…`, `prek-generic.toml`, `components.py`, `guide.md`, `pyproject-template.toml` (decision 2) |
| S4 | Example taken from the history of the scaffolder, not from this project |
| S5 | Sections outside the shape of `CES/index.md`: `Directive`, `Judgment`, `Migration`, `Suppression`, `Enforcement status`, and the like |
| S6 | Lines longer than 100 columns, which the rest of the standards do not have |
| S7 | A measurement stated with no source and no date ("Verified directly", "Measured against", "confirmed directly") |
| S8 | "house" as an unexplained qualifier: house stack, house baseline, house layers |

## Case by case

| CES | Smells and where |
|---|---|
| 5 import-linter | S1 (`AGENTS directive now; a commented skeleton`); S2 "This slice ships the directive + a commented contract skeleton"; S5 `Enforcement status`, `The skeleton`; S6 one line; S8 "house layers" |
| 8 spaghetti-mixed-orchestration | S1 `Tier: judgment`; S5 `Judgment` |
| 16 arch-vocabulary | S1 `Tier: judgment`; S5 `Judgment`; S8 "house architectural vocabulary" |
| 17 api-boundary-layout | S1 (long line naming a skeleton in `pyproject.toml`); S2 "ships as a commented"; S5 `Enforcement status`, `Related`; S6 one line |
| 30 respect-local-repo | S1 `Tier: judgment`; S2 "The scaffolder is clean-adds-only for exactly this reason", and "which is why CES-77 ships as a comment"; S5 `Judgment`, `In practice`; S8 "house standard", "standards rollout" |
| 58 py-legacy-lint-stack | S1 `Tier: judgment`; S5 `Judgment`; S6 one line; S8 "house Python toolchain", "house stack" |
| 64 test-in-memory-adapters | S1 `Tier: judgment`; S3 `.agents/snippets/tests/in_memory_repository.py`; S5 `Pattern`, `Judgment`; S6 one line |
| 65 test-through-interface | S1 `Tier: judgment`; S5 `Judgment` |
| 66 test-coverage-gap | S1 `Tier: judgment`; S5 `Judgment` |
| 74 core-logger | S1 `Ships as: snippet`; S2 "the scaffolder never writes into your package path"; S3 `.agents/snippets/core/logger.py` twice; S5 `Behaviour`, `Usage`, `Adapting`; S6 one line |
| 75 agents-conventional-commits | S1 `Enforced by: prek commit-msg hook + CI check`; S4 `feat(standards): ship the logging family (#81)` and `fix(engine): defer existing AGENTS.md instead of overwriting` are commits of the scaffolder; S5 `Allowed types`, `Suppression` |
| 91 no-ai-coauthorship | S1 `Enforced by`; S5 `Two enforcement layers`, `Recommended tool settings`, `Suppression`; S6 nine lines |
| 107 skills-manifest | S1 `Enforced by: scaffolding check`; S2 throughout: `scaffolding install`, `scaffolding check`, "seed", "restore ops", "the baseline in `components.py`", "never write it from scaffolding"; S3 `components.py`; S5 nine sections; S7 "Measured against `skills@1.5.22`", "Measured: adding `tdd triage no-such-skill-xyz`"; S8 "house baseline". It is a description of the scaffolder more than a standard (decision 4) |
| 109 dep-hygiene-deptry | S1 `Enforced by: prek hook`; S2 "This tool's own `pyproject.toml`", `extend_exclude = ["scaffolding/templates", "skills"]` and the paragraph that explains it; S5 `Scope and false positives`, `Migration`, `Suppression` |
| 110 cognitive-complexity-complexipy | S1; S3 `pyproject-template.toml`; S5 `Threshold`, `When you hit a violation`, `Migration`, `Suppression`; S6 one line; S8 "this house's", "the shipped hook" |
| 111 test-order-randomization | S1 `Enforced by: dev dependency (pytest plugin)`; S5 `Migration`, `Suppression`, `When a test fails under randomization` |
| 113 dependency-review-action | S1; S2 "the `ci` component only ships `dependency-review.yml` when `facts.visibility` is `public`", `[skip]` notice, "the same posture `docker.yml` already uses"; S5 `Enforcement posture`, `Tier-gated`, `Requirements`, `Suppression`; S7 "confirmed directly from GitHub's own docs", "confirmed via the current `PATCH /repos/{owner}/{repo}` schema", "Verified directly against a real repo" |
| 114 hadolint-dockerfile-lint | S1 `Enforced by: prek hook (hadolint-docker, …)`; S2 "Scaffolding already ships a `docker.yml`", "the `docker` CI component", "component-level gating"; S3 `prek-generic.toml`; S5 `Scope`, `License note`, `Migration`, `Suppression`. This repository excludes the standard (no `Dockerfile`) |
| 118 code-duplication-jscpd | S1; S2 "a scaffolded repo", "in the generic template"; S3 `guide.md`, `README.md` as the example of a mirrored file; S5 `Threshold and scope`, `When you hit a violation`, `Migration`, `Suppression`; S7 "Verified directly by running both flag combinations against this repo"; S8 "house stack" |
| 119 osv-scanner-replace-pip-audit | S1; S2 "a scaffolded repo's roadmap (a Next.js frontend, a Go service)"; S4 the file name and the directive speak of replacing `pip-audit.yml`, which is the history of the scaffolder and not of the requirement; S5 `Configuration`, `Migration`, `Suppression`; S6 one line |

## Also

- The 11 standards written from their issues (4, 18, 32, 45, 46, 63, 67, 71, 76, 77, 79) use the
  same `## Directive` heading (S5) that `standards/CES/index.md` does not describe. Either the
  index or the 31 files must change.
- `standards/CES/index.md` and ADR 0004 say that a standard can have several rules "from different
  families". No standard does today: CES-79 and CES-32 have several, all of one family.
- The rule page says `Standard: CES-71` as plain text. Linking it to the page of the standard needs
  botica to know where the standards live, which ADR 0004 rules out; the list at the end of the
  page of the standard is the link in the other direction.

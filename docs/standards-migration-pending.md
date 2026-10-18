# Pending: normalising the migrated CES

The 20 standards that `3cba995` moved from the details of the scaffolder to `standards/CES/` had
the format and the voice of the scaffolder. They were rewritten in one pass, and this page lists
what that pass settled and what is left.

## Settled

1. **How a standard says what enforces it.** It does not. A standard is documentation (ADR 0002):
   the rules that enforce it are the rules whose `standard` field names it, and the page of the
   standard lists them. The loose first line (`Enforced by`, `Tier`, `Ships as`) is gone from every
   standard. The index of `AGENTS.md` keeps its tags (`[prek]`, `[judgment]`, `[reference]`) until
   the standards that a tool enforces have an `external` rule (ADR 0006).
2. **Where the drop-in code lives.** In the topics of `references/`, as ADR 0007 says.
   `.agents/snippets/` and `scaffolding/templates/snippets/` are removed, and CES-64 and CES-74
   point at `botica refs show`.
3. **The shape of a standard.** `## Directive` and `## Why` are in every standard, and the other
   sections are optional. `standards/CES/index.md` describes it, so the 31 files and the index
   agree.
4. **CES-107, CES-113 and CES-114.** They state the requirement. What described the behaviour of
   the scaffolder (what `scaffolding check` verifies, which component ships a workflow, the seed
   and restore operations) is removed.
5. **The title convention.** A title names the requirement and not the tool that checks it, so
   `Dependency hygiene (deptry)` is `Dependency hygiene`.
6. **The smells of the old text.** The voice of the scaffolder, examples taken from its history,
   measurements with no source, "house" without an explanation, bold for emphasis, and lines longer
   than 100 columns are removed from the 20 standards.

## Left

- **Where the knowledge of a tool goes.** The sections `Migration`, `Suppression`, `Configuration`,
  `Scope`, `Threshold` and `Requirements` of CES-109, CES-110, CES-111, CES-113, CES-114, CES-118
  and CES-119 explain how to adopt `deptry`, `complexipy`, `pytest-randomly`,
  `dependency-review-action`, `hadolint`, `jscpd` and `osv-scanner`. ADR 0006 puts that text in the
  README of an `external` rule, one pull request per standard. Until a rule exists the sections
  stay in the standard.
- **The name of the file of CES-119.** `119-osv-scanner-replace-pip-audit.md` speaks of replacing
  `pip-audit.yml`, which is history. The slug is cited in `AGENTS.md` and in the index of the
  scaffolder, so renaming it is a change of its own.
- **CES-75 and CES-91** have sections of their own (`Allowed types`, `Two layers`, `Settings of the
  tools`) that describe the hook and the workflow that enforce them. They go with their rules when
  those exist.

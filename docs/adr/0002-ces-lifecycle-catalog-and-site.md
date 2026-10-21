# CES lifecycle: proposals are pull requests, the catalog documents what botica measures, the site is generated

**Status:** accepted

Amended on 2026-10-19: the message of a finding starts with `CES-<number> (<slug>)` and `botica
explain` prints the `standard` of the rule, so item 1 no longer holds that the CES is absent from
the output of botica.

Amended on 2026-10-20: the message of a finding no longer starts with the CES. The JSON format
carries the `standard` of the rule in each diagnostic, SARIF carries it in the `properties` of each
rule, and `botica explain` prints it. The text output names the rule only. Botica still selects, ignores and suppresses by slug or code, never by CES.

Amended on 2026-10-20, again: item 1 of the Migration is done in another form: the rules that no
tool measures are judgment rules of the catalog (ADR 0005), and ADR 0008 removed the
`engineering-rules` skill. Item 2 is done: the site is `website/`, built by `site.yml`, with giscus
in `website/overrides/partials/comments.html`, and the Discussions categories exist. Item 3 is
open.

A CES was born as a GitHub issue. The issue number is the CES number and the issue body is the
standard's text. Its review state used to be a set of labels that a bot rewrote from slash commands
in the comments:

- **The number belongs to the tracker, not the standard.** Gaps are unavoidable and a CES can only
  exist if an issue exists, so as-built rules need retroactive issues.
- **The text of a standard that no tool measures has no single home.** #164 measured the overlap
  between `.agents/rules/` and the `engineering-rules` files; four direct contradictions were found
  while planning botica lint (#188, #189, #190, #191).
- **Review is fragile.** Votes are comment commands parsed by a bot, editing the body dismisses
  approvals, labels drift, and debate and record-keeping share one thread.

The botica rule catalog (`rules/<family>/<slug>/rule.toml` + README + fixtures) already has to
carry each rule's id, code, status, options and docs. A second description of a rule it measures
would recreate the problem.

## Decision

1. **A CES is documentation; a rule is what botica evaluates.** A CES is the unit of proposals.
   A rule's id is its slug, its code carries a family prefix (`BTSTRUC001`), and it may cite the
   standard it implements (a CES, or one that a user writes) in an informational `standard` field. Botica never selects, ignores or suppresses
   by CES, carries no text or number of one, and the CES is not a field of its output. One CES can
   bring several rules, from different families, and a rule can exist without a CES (the linter's
   own rules, and those that cite none yet).
2. **The catalog is the only documentation of the rules botica measures.** The README of a rule
   says what it enforces, why, and how to silence it; `botica rules` lists the rules and
   `botica explain <slug>` describes one. `AGENTS.md`, `.agents/rules/`, the `engineering-rules`
   skill and `docs/engineering-standards.md` do not repeat a rule the catalog measures, and
   `tests/test_catalog_is_the_only_doc.py` fails if they do.
3. **A rule has its own lifecycle:** `preview` → `stable` → `deprecated` → `removed`, modelled on
   Ruff. Renamed ids keep `aliases` so configs and suppressions never break. Preview rules run only
   when explicitly opted in.
4. **Proposals are pull requests, and they carry the implementation.** A proposal adds the rule
   folder (`rule.toml`, README, fixtures and what runs), not only its text, so the review judges
   what would be enforced. A standard with nothing to run yet is a `judgment` rule. The decision
   is the review of the pull request: approval rules live in `CODEOWNERS` and branch protection,
   not in a comment-parsing bot.
5. **Discussion happens in GitHub Discussions**, one category per area, linked from the pull
   request and embedded on each page of the site via giscus. Reactions are the advisory signal.
6. **The standards website is generated from the catalog with Zensical**, in the manner of the
   documentation of Ruff, flake8 or the PEPs: it documents the rules, the catalogs and the CES. It
   is in English, published from CI to GitHub Pages. A pre-build step
   (`botica rules export --format markdown --output <dir>`) renders one page per rule and an
   index, so the site does not depend on Zensical's module/plugin system. Each page has an "edit this page" link (opens a pull request) and a giscus thread.

## Not decided

Where the text of a standard lives and how its number is assigned are decided in ADR 0004. The
lifecycle of a CES (its statuses and transitions) is still open.

## Considered options

- **Keep issues and the voting bot.** Rejected: the text and the review state would still live
  outside the repo.
- **Discourse or another forum.** Rejected for now: a separate account and host for a community
  that already lives on GitHub. Discussions can be exported later if the community outgrows it.
- **Astro Starlight.** It has schema-validated content, built-in search and i18n. Rejected
  only to keep the toolchain in the Python ecosystem. **Material for MkDocs** is in maintenance
  mode with end of life in May 2027, so it is not an option for a new site.

## Consequences

- The voting bot, its script and tests, the proposal issue template and the review chapter of
  `docs/engineering-standards.md` are removed, with no compatibility layer. Nothing replaces them
  until the site and Discussions exist, so a proposal has no process in the meantime. The
  `state:*` / `vote:*` / `approved:lazy` labels and the open proposal issues are still on the
  tracker; close the issues with a link to where their standard lives once that is decided.
- Zensical's stable line starts on 2026-11-05. Until then the site is built with a pinned
  version, and anything it cannot render is produced by the export step, not by plugins.
  Verify that giscus can be embedded through a template override before relying on it.

## Migration

What remains, in order:

1. Catalogue the rules that no tool measures (`kind = "judgment"`, ADR 0003) and generate the
   `engineering-rules` skill from them, resolving first the overlaps measured in #164 and the
   contradiction issues (#188–#191).
2. Stand up the Zensical site and Discussions categories; enable giscus.
3. Close and link the legacy issues; delete the labels.

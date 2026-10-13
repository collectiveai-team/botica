# CES lifecycle: proposals are pull requests, the catalog documents what botica measures, the site is generated

**Status:** proposed

Today a CES is born as a GitHub issue. The issue number is the CES number, the issue body is the
standard's text, and its review state is a set of labels that a bot (`proposal-vote.yml`,
`proposal-update.yml`) rewrites from slash commands in the comments. In practice this has been
confusing for contributors and expensive to maintain:

- **The number belongs to the tracker, not the standard.** Gaps are unavoidable and a CES can only
  exist if an issue exists, so as-built rules need retroactive issues.
- **The text of a standard that no tool measures has no single home.** #164 measured the overlap
  between `.agents/rules/` and the `engineering-rules` files; four direct contradictions were found
  while planning botica lint (#188, #189, #190, #191).
- **Review is fragile.** Votes are comment commands parsed by a bot, editing the body dismisses
  approvals, labels drift, and debate and record-keeping share one thread. Keeping it working
  took ~20 KB of workflow.

The botica rule catalog (`rules/<family>/<slug>/rule.toml` + README + fixtures) already has to
carry each rule's id, code, status, options and docs. A second description of a rule it measures
would recreate the problem.

## Decision

1. **A CES is documentation; a rule is what botica evaluates.** A CES is the unit of proposals.
   A rule's id is its slug, its code carries a family prefix (`BTSTRUC001`), and it may cite the
   CES it implements in an informational `ces` field. Botica never selects, ignores or suppresses
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
4. **Proposals are pull requests.** The decision is the review of the pull request: approval rules
   live in `CODEOWNERS` and branch protection, not in a comment-parsing bot.
5. **Discussion happens in GitHub Discussions**, one category per area, linked from the pull
   request and embedded on each page of the site via giscus. Reactions are the advisory signal
   that `/upvote` was trying to be.
6. **The standards website is generated from the catalog with Zensical**, in English, published
   from CI to GitHub Pages. A pre-build step (`botica rules export --format markdown`, not built
   yet) renders one page per rule, so the site does not depend on Zensical's module/plugin
   system. Pages offer "edit this page" (opens a pull request) and a giscus thread.

## Not decided

Where the text of a CES and its number live. It is planned for a later stage of the botica lint
work, and until then the convention of `AGENTS.md` stands (`CES-<issue#>`, one tracker issue per
CES). Everything that depends on it is open: how numbers are assigned, the lifecycle of a CES
(its statuses and transitions), what a proposal adds in its pull request, and whether the site has
a page per CES. The change that decides it also states the new invariant in `AGENTS.md`, for the
reason ADR 0001 gives.

## Considered options

- **Keep issues, fix the bot.** Rejected: the text and the review state would still live outside
  the repo.
- **Discourse or another forum.** Rejected for now: a separate account and host for a community
  that already lives on GitHub. Discussions can be exported later if the community outgrows it.
- **Astro Starlight.** Strong option (schema-validated content, built-in search, i18n), rejected
  only to keep the toolchain in the Python ecosystem. **Material for MkDocs** is in maintenance
  mode with end of life in May 2027, so it is not an option for a new site.

## Consequences

- `proposal-vote.yml`, `proposal-update.yml`, the proposal issue template, and the
  `state:*` / `vote:*` / `approved:lazy` labels are retired after migration. Existing proposal
  issues are closed with a link to where their standard lives by then.
- Zensical's stable line starts on 2026-11-05. Until then the site is built with a pinned
  version, and anything it cannot render is produced by the export step, not by plugins.
  Verify that giscus can be embedded through a template override before relying on it.

## Migration

What remains, in order:

1. Freeze new proposal issues; point the issue template at Discussions.
2. Catalogue the rules that no tool measures (`kind = "judgment"`, ADR 0003) and generate the
   `engineering-rules` skill from them, resolving first the overlaps measured in #164 and the
   contradiction issues (#188–#191).
3. Stand up the Zensical site and Discussions categories; enable giscus.
4. Close and link the legacy issues; retire the bot and the labels.

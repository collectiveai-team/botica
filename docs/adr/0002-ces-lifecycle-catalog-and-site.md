# CES lifecycle: the rule catalog is the source of truth, proposals are PRs, the site is generated

**Status:** proposed

> **Amended 2026-10-09** (user decision D15 of the botica lint spec). A CES and a rule are two
> different things. A **CES** is documentation and the unit of proposals: botica never selects,
> ignores or suppresses by CES, and the CES is not a field of its output. A **rule** is what botica
> evaluates: its id is its slug, its code carries a family prefix (`BTSTRUC001`), and it may cite
> the CES it implements in an informational `ces` field. One CES can bring several rules, from
> different families, and a rule can exist without a CES (the linter's own rules, and
> `no-tests-in-package` until its CES exists).
>
> This changes decision 1 (the rule catalog is no longer the source of truth for a CES: a rule
> only points at one) and the first Consequence ("every CES has a catalog entry"), and it leaves
> decision 5 open (numbers are no longer assigned "by the catalog"). **Where the text of a CES and
> its number live instead is not decided**, so those points stay below as first written until it
> is. The rule lifecycle, proposals as pull requests, Discussions and the generated site are
> unaffected.
>
> **Amended 2026-10-13** (user decision). A CES lives **only as documentation**: botica carries no
> text and no number of a CES, only the informational `ces` reference of a rule. The new definition
> of a CES (where its text and its number are kept) is planned for a later stage of the botica lint
> spec, so decisions 1 and 5 stay below as first written until then.

Today a CES is born as a GitHub issue. The issue number is the CES number, the issue body is the
standard's text, and its review state is a set of labels that a bot (`proposal-vote.yml`,
`proposal-update.yml`) rewrites from slash commands in the comments. The text is then copied into
`.agents/rules/<slug>.md`, `docs/engineering-standards.md`, the `engineering-rules` skill, and
ast-grep YAML. In practice this has been confusing for contributors and expensive to maintain:

- **The number belongs to the tracker, not the standard.** Gaps are unavoidable and a CES can only
  exist if an issue exists, so as-built rules need retroactive issues.
- **The text has no single home.** #164 already measured the overlap between `.agents/rules/`
  and the 70 `engineering-rules` files; four direct contradictions were found while planning
  botica lint (#188, #189, #190, #191).
- **Review is fragile.** Votes are comment commands parsed by a bot, editing the body dismisses
  approvals, labels drift, and debate and record-keeping share one thread. Keeping it working
  required a dedicated TUI (`review/`) and ~20 KB of workflow.

botica lint introduces a rule catalog (`rules/<catalog>/<slug>/rule.toml` + README + fixtures)
that already has to carry each rule's id, CES code, status, options, and docs. Keeping a second
source of truth for the same standard would recreate the problem.

## Decision

1. **The botica rule catalog is the single source of truth for every standard.** A CES is a
   catalog entry. `.agents/rules/`, the `engineering-rules` skill, `docs/engineering-standards.md`
   tables, `botica explain`, and the website are generated from it. (This implements #164.)
2. **Two lifecycles, linked.**
   - *Standard (CES):* `draft` → `accepted` → `active` → `deprecated` (with `superseded-by`);
     terminal `rejected` / `withdrawn`. Terminal entries stay published as a record.
   - *Detector (rule):* `preview` → `stable` → `deprecated` → `removed`, modelled on Ruff.
     Renamed ids keep `aliases` so configs and suppressions never break. Preview rules run only
     when explicitly opted in.
   - A CES page shows its **coverage**: which detectors enforce it (`covered-by`) and what remains
     judgment.
3. **Proposals are pull requests.** A proposal is a PR adding the standard's catalog entry with
   `status = "draft"` (or a `standards/proposals/<slug>.md` when no detector exists yet). The
   decision is the PR review: approval rules live in `CODEOWNERS` and branch protection, not in a
   comment-parsing bot. Merging with `status = "accepted"` is acceptance.
4. **Discussion happens in GitHub Discussions**, one category per area, linked from the PR and
   embedded on each standard's page via giscus. Reactions are the advisory signal that
   `/upvote` was trying to be.
5. **CES numbers are assigned at acceptance, sequentially, by the catalog** — not by the issue
   tracker. Existing numbers are frozen; new numbers continue after the highest existing one, so
   no citation changes meaning. The slug remains the stable machine and suppression key.
6. **The standards website is generated from the catalog with Zensical**, in English, published
   from CI to GitHub Pages. A pre-build step (`botica rules export --format markdown`) renders one
   page per standard and per rule, so the site does not depend on Zensical's (not yet released)
   module/plugin system. Pages offer "edit this page" (opens a PR) and a giscus thread.

## Considered options

- **Keep issues, fix the bot.** Rejected: the number, the text, and the review state would still
  live outside the repo, and the catalog would still need a second copy.
- **Discourse or another forum.** Rejected for now: a separate account and host for a community
  that already lives on GitHub. Discussions can be exported later if the community outgrows it.
- **Astro Starlight.** Strong option (schema-validated content, built-in search, i18n), rejected
  only to keep the toolchain in the Python ecosystem. **Material for MkDocs** is in maintenance
  mode with end of life in May 2027, so it is not an option for a new site.
- **Number at proposal time (PEP style).** Rejected: it reproduces today's gaps; numbering at
  acceptance keeps the sequence meaningful.

## Consequences

- `docs/engineering-standards.md` changes its invariants: "every CES has a tracker issue" becomes
  "every CES has a catalog entry"; "number source = the issue id" becomes "assigned at
  acceptance". `AGENTS.md` must state the new invariant in the same change, for the same reason
  ADR 0001 gives.
- `review/`, `proposal-vote.yml`, `proposal-update.yml`, the proposal issue template, and the
  `state:*` / `vote:*` / `approved:lazy` labels are retired after migration. Existing proposal
  issues are closed with a link to their catalog page; declined ones are imported as `rejected`.
- Zensical's stable line starts on 2026-11-05. Until then the site is built with a pinned
  version, and anything it cannot render is produced by the export step, not by plugins.
  Verify that giscus can be embedded through a template override before relying on it.

## Migration

1. Freeze new proposal issues; point the issue template at Discussions.
2. Import the 25+ coded CES and the 70 `engineering-rules` files into the catalog, resolving the
   overlaps measured in #164 and the four contradiction issues (#188–#191) first.
3. Generate `.agents/rules/` and the skill from the catalog; add a drift test.
4. Stand up the Zensical site and Discussions categories; enable giscus.
5. Close and link the legacy issues; retire the bot, the TUI, and the labels.

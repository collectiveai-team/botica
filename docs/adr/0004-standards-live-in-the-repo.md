# Standards live in the repo, apart from the rules that implement them

**Status:** proposed

ADR 0002 left open where the text of a standard lives. A standard is a level above a rule: it says
what is required, a rule is one way to check it. One standard can have several rules, from
different families, and a rule can implement none. Botica evaluates rules and knows nothing about
standards. Collective writes the CES, and another team can write standards of its own.

## Decision

1. **A standard is a file written by hand**, `standards/<set>/<number>-<slug>.md`, at the root of
   the repository. A set is the folder of the standards of one author (`ces`). The file carries
   `id` and `title` in its front matter and prose after it.
2. **A rule names the standard it implements** in the `standard` field of its `rule.toml`, as
   `<SET>-<number>` (`CES-71`, `ACME-3`). Botica only checks the format. That the file exists is
   checked outside botica.
3. **A change touches one standard.** A pull request that proposes or modifies a standard carries
   the implementation (the folder of each rule that implements it) and updates the documentation
   that the change affects. `scripts/check_rule_change.py` fails a pull request that touches two
   standards, and no label accepts it.
4. **The discussion happens in GitHub Discussions**, shown at the end of the page of the standard.
5. **The site copies `standards/`** into its documents (`scripts/build_site.sh`); the copy is never
   committed, and "edit this page" opens the file at the root.

## Not decided

How the number of a standard is assigned, and the lifecycle of a standard (its statuses and
transitions). Until then the convention of `AGENTS.md` stands: `CES-<issue#>`. The existing CES
move to `standards/ces/` one pull request each, as the rule above asks.

## Considered options

- **`rules export` writes the page of each standard.** Rejected: botica would have to know the
  standards, and a rule is not bound to one.
- **Keep the text in the tracker issue.** Rejected for the reasons of ADR 0002: the text and the
  review state would live outside the repository.
- **`standards/` under `website/docs/`.** Rejected: the standards belong to the repository, and the
  site is one way to read them.

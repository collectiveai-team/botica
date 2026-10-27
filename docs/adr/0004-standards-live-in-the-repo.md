# Standards live in the repo, apart from the rules that implement them

**Status:** accepted

**Amended 2026-10-20:** a standard is written and kept by people, at a higher level than its
rules, and it may repeat what the README of a rule says: its examples, configuration and migration
stay in the standard. `docs/standards-migration-pending.md` is removed, and so is
`docs/engineering-standards.md`: how a standard is numbered and cited is in `standards/CES/index.md`.

**Amended 2026-10-26:** a standard gets its number at the merge, not when it is proposed (decision
6). A proposal is a draft pull request whose standard is `0-<slug>.md`, `id: <SET>-0`, and whose
rules name `<SET>-0`; `scripts/number_standard.py` numbers it before the merge.

ADR 0002 left open where the text of a standard lives. A standard states what is required; a rule
is one way to check it. A standard has any number of rules, none included, and a rule names at most
one standard. Botica evaluates rules. Collective writes the CES, and another team can write
standards of its own.

## Decision

1. **A standard is a file written by hand**, `standards/<set>/<number>-<slug>.md`, at the root of
   the repository. A set is the folder of the standards of one author, named like the prefix of
   their ids (`CES` for `CES-71`). The file carries `id` and `title` in its front matter and prose
   after it. A standard that cites topics (ADR 0007) adds `references`, a JSON list of their ids.
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
6. **The number of a standard is the next free one of its set, `max + 1`, taken at the merge.**
   A proposal has no number: its standard is `standards/<set>/0-<slug>.md`, with `id: <SET>-0`,
   its rules name `<SET>-0`, and its commits name the standard by its title. Before the merge, on
   the branch rebased onto `main`, `scripts/number_standard.py <SET>` renames the file to the next
   free number and writes it in the `id` and in the `standard` of each rule. The rule contract of
   CI (`scripts/check_rule_change.py`) accepts a standard numbered 0 in a draft and fails a pull
   request ready for review that still has one. The standards that existed as issues keep their
   number (up to `CES-119`). A number is never reused or changed, and a standard that is rejected
   takes none. `scripts/check_rule_change.py` fails a change that leaves two standards of a set
   with the same number.

## Not decided

The lifecycle of a standard (its statuses and transitions). Every CES now has its file.

## Considered options

- **`rules export` writes the page of each standard.** Rejected: botica would have to know the
  standards, and a rule is not bound to one.
- **Keep the text in the tracker issue.** Rejected for the reasons of ADR 0002: the text and the
  review state would live outside the repository.
- **The number of a standard is the number of its pull request,** as in the RFCs of Rust. It never
  repeats, and it does not clash with the issues, since GitHub shares one counter between issues
  and pull requests. Rejected: the author does not know the number until the pull request is open,
  and the rules that cite the standard, in `standard` and in the message of each violation, would
  have to be renamed afterwards.
- **`standards/` under `website/docs/`.** Rejected: the standards belong to the repository, and the
  site is one way to read them.

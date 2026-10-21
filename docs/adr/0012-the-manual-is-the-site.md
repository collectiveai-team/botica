# The manual of botica is the site

**Status:** accepted

The manual of botica is `docs/botica.md`, one file of 805 lines that GitHub renders. The site
shows the standards, the rules and the references from the catalog, and two pages written by hand:
`index.md` and `use.md`, which sends the reader to the manual on GitHub. The site has no guide of
use and no page on how to extend botica, and the README does not say what botica is.

## Decision

1. **The manual is pages of the site, written by hand under `website/docs/`.** `docs/botica.md` is
   split by its sections; no page is generated from it, and no other file copies it.

   | Section | Page |
   |---|---|
   | (new) what botica is, a first run; Install | `start.md` (replaces `use.md`) |
   | Use, Rules, Which files, Result cache | `guide/lint.md` |
   | Configuration | `guide/configuration.md` |
   | Silence a finding, Known findings | `guide/findings.md` |
   | Recipes | `guide/recipes.md` |
   | References | `guide/refs.md` |
   | Editor | `guide/editor.md` |
   | prek, the workflow of abide, and (new) a job of CI | `guide/prek-and-ci.md` |
   | Catalogs of a project, up to the extensions | `extend/catalogs.md` |
   | Catalogs of a project, the extensions; (new, from the `README.md` of each SDK) the protocol and the SDKs of Python, Go, Node and Rust | `extend/extensions.md` |
   | the external rules of Rules, abide and `rules test`; (new) a pattern rule, its fixtures, a judgment rule | `extend/rules.md` |
   | Develop | `docs/develop.md` (for the maintainers, not on the site) |

   The `README.md` of each SDK keeps what a package registry shows: install and a minimal example,
   with a link to `extend/extensions.md`. A page links a file of the repository outside
   `website/docs/` by its address on GitHub at `main`, since Zensical in strict mode fails a
   relative link that leaves the folder of the documents.

2. **`docs/botica.md` is removed.** What named it names the page that holds its section now:
   `scripts/bump_mirror_rev.py` rewrites the `rev` of the mirror in `guide/prek-and-ci.md`,
   `scripts/check_rule_change.py` lists `website/docs/guide/` and `website/docs/extend/`, the tests of both,
   `tests/test_catalog_is_the_only_doc.py`, `docs/botica-release.md`, ADR 0005, `AGENTS.md` and
   the README.

3. **`scripts/site_nav.py` adds the sections "Start", "Guide" and "Extend"** before the standards,
   in the order of the table, and each page has a `title` in its front matter. `llms.txt` lists the
   pages of the manual first (`scripts/site_llms.py`).

4. **The README says what botica is, how to install it, how to run it and how to extend it,** and
   links to the site for the rest. A section of the README that covers a page of the site says it
   in fewer words and links to the page, so the hook `jscpd` of `prek.toml` needs no new
   exclusion.

## Considered options

- **`scripts/build_site.sh` copies `docs/botica.md` split by section,** as it copies
  `standards/`. Rejected: the split needs markers in the file, and a link between two sections
  would work on GitHub and break on the site.
- **New pages written for the site, and the manual kept.** Rejected: two texts of the same
  commands diverge, and jscpd reports what they repeat.
- **The manual stays in `docs/`, and the site links to it.** Rejected: it is what the site does
  today.

## Consequences

- A page of the site opens on "edit this page" at the file that holds the text.
- A pull request that changes a command updates its page under `website/docs/`.
- A preview of the site on each pull request is not decided.

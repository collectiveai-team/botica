---
title: Standards
---

# Standards

A standard is a requirement and its reason. `botica lint` runs [rules](../rules/index.md), and the
`rule.toml` of a rule names the standard it implements:

```toml
standard = "CES-71"
```

A standard belongs to a set, and a set is the folder of the standards of one author, named like the
prefix of their ids: the standards of `CES-71` live in `standards/CES/`. A rule names a standard of
any set the same way.

## Sets

- [CES](CES/index.md), the Collective Engineering Standard.

## Propose or change a standard

A standard is one file, `standards/<set>/<number>-<slug>.md`, written by hand. Its number is the
next free one of the set, and it never changes.

1. Open a pull request that adds or edits the file of **one** standard, with the template for
   standards (add `?template=standard.md` to the address of the new pull request). A pull request
   touches one standard; the check of the repository fails a pull request that touches two.
2. The same pull request carries the implementation: the folder of each rule that implements the
   standard.
3. The same pull request updates the documentation that the change affects.
4. The discussion happens in [GitHub Discussions](https://github.com/collectiveai-team/botica/discussions),
   linked from the pull request and shown at the end of the page of each rule and each standard.

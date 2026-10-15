---
title: Standards
---

# Standards

A standard says how engineering work is done. Botica does not evaluate standards: it evaluates
[rules](../rules/index.md), and a rule may name the standard it implements in its `rule.toml`:

```toml
standard = "CES-71"
```

A standard belongs to a set, and a set is the folder of the standards of one author, named like the
prefix of their ids: the standards of `CES-71` live in `standards/CES/`. A rule that implements a
standard of any set names it the same way.

## Sets

- [CES](CES/index.md), the Collective Engineering Standard.

## Propose or change a standard

A standard is one file, `standards/<set>/<number>-<slug>.md`, written by hand. Its number is the
next free one of the set, and it never changes.

1. Open a pull request that adds or edits the file of **one** standard, with the template for
   standards (add `?template=standard.md` to the address of the new pull request). A pull request
   never touches two standards, and the check of the repository fails it if it does.
2. The same pull request carries the implementation: the folder of each rule that implements the
   standard.
3. The same pull request updates the documentation that the change affects.
4. The discussion happens in [GitHub Discussions](https://github.com/collectiveai-team/botica/discussions),
   linked from the pull request and shown at the end of the page of each rule and each standard.

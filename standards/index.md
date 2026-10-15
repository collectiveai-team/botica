---
title: Standards
---

# Standards

A standard says how engineering work is done. Botica does not evaluate standards: it evaluates
[rules](../rules/index.md), and a rule may name the standard it implements in its `rule.toml`:

```toml
standard = "CES-71"
```

A standard belongs to a set, and a set is the folder of the standards of one author. A rule that
implements a standard of any set names it the same way.

## Sets

- [CES](ces/index.md), the Collective Engineering Standard.

## Propose or change a standard

A standard is one file, `standards/<set>/<number>-<slug>.md`, written by hand.

1. Open a pull request that adds or edits the file of **one** standard. A pull request never
   touches two standards, and the check of the repository fails it if it does.
2. The same pull request carries the implementation: the folder of each rule that implements the
   standard, so the review judges what would be enforced and not only the text.
3. The same pull request updates the documentation that the change affects.
4. The discussion happens in [GitHub Discussions](https://github.com/collectiveai-team/botica/discussions),
   linked from the pull request and shown at the end of each page.

---
title: botica
---

# ![botica](assets/wordlogo-light.svg#only-light){ width="320" }![botica](assets/wordlogo-dark.svg#only-dark){ width="320" }

botica checks, deterministically, the engineering standards that can be measured in one file. This
site has one page per rule of its catalog: what the rule asks, why, and how to silence it.

Start with the [list of rules](rules/index.md).

```bash
uvx botica lint            # run the rules active in the current directory
uvx botica rules --all     # every rule, active or not, and why
uvx botica explain <rule>  # one rule, as it is configured here
```

A rule with the status `preview` is off by default: it runs only with `preview = true` in the
configuration, and when it is selected.

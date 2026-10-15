---
title: Botica
---

<div class="botica-hero">
  <img class="botica-icon" src="assets/logo-dark.svg#only-light" alt="">
  <img class="botica-icon" src="assets/logo-light.svg#only-dark" alt="">
  <h1 id="botica">
    <img src="assets/wordlogo-light.svg#only-light" alt="Botica">
    <img src="assets/wordlogo-dark.svg#only-dark" alt="Botica">
  </h1>
  <p class="botica-tagline">
    A deterministic linter for the engineering standards that can be measured in one file.
    It never calls a model or the network.
  </p>
</div>

## Get started

```bash
uvx botica lint            # run the rules active in the current directory
uvx botica rules --all     # every rule, active or not, and why
uvx botica explain <rule>  # one rule, as it is configured here
```

[How to install, configure and silence it](use.md){ .md-button .md-button--primary }
[The rules](rules/index.md){ .md-button }

## Standards and rules

A **standard** says what is required and why. A **rule** is one way to check it: botica evaluates
rules and knows nothing about standards, but a rule may name the standard it implements, and a
standard can have several rules, from different families.

- [Standards](standards/index.md) are written by hand, one file each, in sets. The
  [CES](standards/CES/index.md), the Collective Engineering Standard, is the set of Collective.
- [Rules](rules/index.md) are listed by family, one page each: what the rule asks, why, and how to
  silence it.

## Propose a change

A standard or a rule changes through a pull request that carries the implementation, so the review
judges what would be enforced and not only the text. The discussion happens in
[GitHub Discussions](https://github.com/collectiveai-team/botica/discussions) and is shown at the
end of every page. [How to propose a standard](standards/index.md#propose-or-change-a-standard).

A rule with the status `preview` is off by default: it runs only with `preview = true` in the
configuration, and when it is selected.

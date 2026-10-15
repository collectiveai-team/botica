<div class="botica-hero">
  <div class="botica-brand">
    <img class="botica-icon" src="assets/logo-light.svg#only-light" alt="">
    <img class="botica-icon" src="assets/logo-dark.svg#only-dark" alt="">
    <h1 id="botica">
      <img class="botica-wordlogo" src="assets/wordlogo-light.svg#only-light" alt="Botica">
      <img class="botica-wordlogo" src="assets/wordlogo-dark.svg#only-dark" alt="Botica">
    </h1>
  </div>
  <p class="botica-tagline"><strong><em>Botica: recetas, alquimia y artilugios</em></strong></p>
  <p>Recipes and standards for projects: deterministic rules, templates, and design decisions.</p>
</div>

## Get started

```bash
uvx botica lint            # run the rules active in the current directory
uvx botica rules --all     # every rule, active or not, and why
uvx botica explain <rule>  # one rule, as it is configured here
```

- [Use](use.md): install, run, configure, silence a finding, run on every commit.
- [Standards](standards/index.md): the requirements, in sets.
- [Rules](rules/index.md): the checks of `botica lint`, by family.

## Standards and rules

A standard is a requirement and its reason, written in `standards/<set>/<number>-<slug>.md`. A rule
is a check that `botica lint` runs. The `rule.toml` of a rule names the standard it implements
(`standard = "CES-71"`), and the page of a standard lists its rules at the end. The
[CES](standards/CES/index.md) is the set of standards of Collective.

## Propose a change

A change to a standard is a pull request with the file of the standard and the folders of its
rules. The discussion is in
[GitHub Discussions](https://github.com/collectiveai-team/botica/discussions), at the end of the
page of each rule and each standard. The steps are in
[Propose or change a standard](standards/index.md#propose-or-change-a-standard).

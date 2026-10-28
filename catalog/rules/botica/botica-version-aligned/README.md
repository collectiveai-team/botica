## Directive

A project names one version of botica: the hooks, the workflows and the parameter `botica-version`
of its recipes follow the `rev` of the mirror `collectiveai-team/botica-pre-commit`. The recipe
`base` writes that `rev`, the `uvx botica@<version>` lines of the commit-msg hooks and of
`abide.yml`, `commit-policy.yml` and `conventional-commits.yml` with one version, and `prek update`
moves only the `rev`.

```toml
# prek.toml
[[repos]]
repo = "local"

[[repos.hooks]]
id = "agents-conventional-commits"
entry = "uvx botica@0.16.1 lint --select conventional-commits --commit-msg"  # reported here

[[repos]]
repo = "https://github.com/collectiveai-team/botica-pre-commit"
rev = "v0.17.1"
```

The reference is the first of:

1. the `rev` of the mirror in `prek.toml`;
2. the `rev` of the mirror in `.pre-commit-config.yaml`;
3. the newest version that the `botica@<version>` of the project name.

A `rev` that is not a version, such as a commit, is skipped. botica reports, each on its line:

- a `botica@<version>` of `prek.toml`, of `.pre-commit-config.yaml` or of a `.yml` or `.yaml` file
  of `.github/workflows/`, at the root of the project, with another version. botica leaves out the
  lines that start with `#`, a version that does not start with a digit (`botica@latest`) and an
  action of GitHub (`owner/botica@v1`);
- the `rev` of the mirror in `.pre-commit-config.yaml`, when `prek.toml` has the mirror at another
  version;
- the parameter `botica-version` in `[recipes.params]` of `botica.toml`, or in
  `[tool.botica.recipes.params]` of `pyproject.toml`, with another version. `botica recipe install`
  writes `botica@` and the `rev` with that value; without it, `install` writes the version of the
  botica that runs it.

botica does not read a dependency on botica, such as `botica==0.16.1` in the `dev` group of
`pyproject.toml`, or its version in `uv.lock`: a project that runs botica as a dependency of uv
moves it with `uv lock --upgrade-package botica`, and the rule does not compare that version.

## Why

The mirror is the version that prek installs and that `prek update` moves. After `prek update`, the
hooks of the mirror run the new release and the commit-msg hooks and the workflows run the old one:
a rule that the new release added or changed is checked on one side and not on the other. A stale
`botica-version` writes the old version again on the next `install`.

## Suppression

The finding is about the versions of the project, so only the configuration silences it:
`ignore = ["botica-version-aligned"]`, or the path in `extend-ignores` of
`[rules.botica-version-aligned]`. To fix it, write the version of the reference where the finding
says, or remove `botica-version` from the parameters.

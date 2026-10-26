# botica-pre-commit

Mirror of [botica](https://github.com/collectiveai-team/botica) for prek and pre-commit: it holds
the hook definitions and nothing else. The source is `distribution/botica-pre-commit/` in the
botica repository, copied here at each release (see "Releasing").

```toml
# prek.toml
[[repos]]
repo = "https://github.com/collectiveai-team/botica-pre-commit"
rev = "v0.15.1"

[[repos.hooks]]
id = "botica"

[[repos.hooks]]
id = "botica-project"

[[repos.hooks]]
id = "botica-agents-md"

[[repos.hooks]]
id = "botica-rubric"
```

Hooks: `botica` (the rules that look at one file; gets the file names) and `botica-project` (the
rules that need the whole project; gets none). Both read the selection from the configuration of
botica (`botica.toml` or `[tool.botica]`), the one that `botica recipe install` writes.
`botica-agents-md` and `botica-rubric` fail when the block of `AGENTS.md` or `.abide/rubric.json`
differs from what `botica rules export` writes; run the export to update them.

## Releasing

The job `mirror` of `.github/workflows/botica-release.yml` in the botica repository updates and
tags this repository. `prek update` in a project moves `rev` to the new tag.

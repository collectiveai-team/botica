# botica-pre-commit

Mirror of [botica](https://github.com/collectiveai-team/botica) for prek and pre-commit: it holds
the hook definitions and nothing else. The source is `distribution/botica-pre-commit/` in the
botica repository, copied here at each release (see "Releasing").

```toml
# prek.toml
[[repos]]
repo = "https://github.com/collectiveai-team/botica-pre-commit"
rev = "v0.13.0"

[[repos.hooks]]
id = "botica"

[[repos.hooks]]
id = "botica-project"
```

Hooks: `botica` (the rules that look at one file; gets the file names) and `botica-project` (the
rules that need the whole project; gets none). Both read the selection from the configuration of
botica (`botica.toml` or `[tool.botica]`), the one that `botica recipe install` writes.

## Releasing

1. A `vX.Y.Z` tag in the botica repository builds the wheels and publishes them (see
   `.github/workflows/botica-release.yml`).
2. Here, set `version` and the `botica==` pin in `pyproject.toml` to `X.Y.Z`, commit, and tag
   `vX.Y.Z`. Both versions must match, or prek installs a different binary than the tag names.
3. `prek autoupdate` in a project moves `rev` to the new tag.

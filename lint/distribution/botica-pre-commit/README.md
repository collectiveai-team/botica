# botica-pre-commit

Mirror of [botica](https://github.com/collectiveai-team/botica) for prek and pre-commit: it holds
the hook definitions and nothing else. The source is `lint/distribution/botica-pre-commit/` in the
botica repository, copied here at each release (see "Releasing").

```toml
# prek.toml
[[repos]]
repo = "https://github.com/collectiveai-team/botica-pre-commit"
rev = "v0.8.0"

[[repos.hooks]]
id = "botica"
args = ["--select", "<the rules of prek-generic.toml>"]
```

Hooks: `botica` (the rules that look at one file; gets the file names) and `botica-project` (the
rules that need the whole project; gets none).

## Releasing

1. A `vX.Y.Z` tag in the botica repository builds the wheels and publishes them (see
   `.github/workflows/botica-release.yml`).
2. Here, set `version` and the `botica==` pin in `pyproject.toml` to `X.Y.Z`, commit, and tag
   `vX.Y.Z`. Both versions must match, or prek installs a different binary than the tag names.
3. `prek autoupdate` in a project moves `rev` to the new tag.

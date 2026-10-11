# botica-pre-commit

Mirror of [botica](https://github.com/collectiveai-team/botica) for prek and pre-commit. This
directory is the content of the `collectiveai-team/botica-pre-commit` repository; it lives here
until that repository exists.

```toml
# prek.toml
[[repos]]
repo = "https://github.com/collectiveai-team/botica-pre-commit"
rev = "v0.1.0"

[[repos.hooks]]
id = "botica"
args = ["--no-astgrep", "--select", "file-size-guard,no-utils,no-notebooks,no-artifacts-in-package,no-tests-in-package"]
```

Hooks: `botica` (the rules that look at one file; gets the file names) and `botica-project` (the
rules that need the whole project; gets none).

## Releasing

1. A `botica-vX.Y.Z` tag in the botica repository builds the wheels and publishes them (see
   `.github/workflows/botica-release.yml`).
2. Here, set `version` and the `botica==` pin in `pyproject.toml` to `X.Y.Z`, commit, and tag
   `vX.Y.Z`. Both versions must match, or prek installs a different binary than the tag names.
3. `prek autoupdate` in a project moves `rev` to the new tag.

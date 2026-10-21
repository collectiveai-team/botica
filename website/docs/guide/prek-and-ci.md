---
title: prek and CI
---

# prek and CI

## prek

```toml
# prek.toml: the mirror installs the wheel of the same version from PyPI
[[repos]]
repo = "https://github.com/collectiveai-team/botica-pre-commit"
rev = "v0.13.0"

[[repos.hooks]]
id = "botica"

[[repos.hooks]]
id = "botica-project"

[[repos.hooks]]
id = "botica-agents-md"

[[repos.hooks]]
id = "botica-rubric"
```

The hooks read the selection from the configuration, as `botica lint` and the editor do.
`botica-agents-md` and `botica-rubric` run `botica rules export --check` for the block of
`AGENTS.md` and for `.abide/rubric.json`, on every commit, since a new file changes the rubric. The
mirror's hooks are `verbose`: prek hides the output of a passing hook, and a size warning does
not fail it
([`distribution/botica-pre-commit`](https://github.com/collectiveai-team/botica/blob/main/distribution/botica-pre-commit/README.md)).
The recipe `base` installs these hooks from `catalog/recipes/files/prek-base.toml`, and writes the
selection to `botica.toml`.

## CI

A job of GitHub Actions runs the same selection on the lines that a pull request adds, and writes
each finding as an annotation of the diff. `--diff` needs the history down to the merge base, and
`<version>` is the version that `prek.toml` pins:

```yaml
- uses: actions/checkout@v7
  with:
    fetch-depth: 0
    persist-credentials: false
- uses: astral-sh/setup-uv@v10.3.0
- name: botica
  env:
    BASE_REF: ${{ github.base_ref }}
  run: uvx botica@<version> lint --diff "origin/$BASE_REF" --output-format github
```

Without `--diff`, the job lints every file. The recipe `ci` installs two workflows that read text
and no file: `conventional-commits.yml` runs `botica lint --select conventional-commits
--pr-title="$PR_TITLE"` on the title of a pull request, and `commit-policy.yml` runs
`botica lint --select no-ai-coauthorship --commits "$range"` on its commits.

## abide

The workflow `.github/workflows/abide.yml`, which the recipe `base` installs, judges the diff of
each pull request: `abide check --json --diff`, judged a second time when the first run has a
finding or fails, then `botica lint --judged` with `--fail-on warning`, and a table of the findings
(`--output-format markdown`) in the summary of the job.
It reads the secret `TYPESAFE_AI_API_KEY` and the variable `TYPESAFE_AI_BASE_URL` of the
repository; a pull request from a fork gets no secret, and the job fails.
[Judgment rules](../extend/rules.md#judgment-rules) says what abide judges.

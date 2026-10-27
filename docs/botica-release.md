# Releasing botica

`botica` is built from the Rust workspace at the root of the repository and shipped as wheels that carry the binary and the Python module `botica.sdk`. A tag
`vX.Y.Z` runs `.github/workflows/botica-release.yml` (the file name is the one the PyPI trusted
publisher is registered with).

A release does not need `main`: a tag runs the workflow as it is in the tagged commit. The
branch that holds the tag is pushed with it, because the job `bump` commits to that branch: `main`
when the tag is on `main`, or else the one remote branch that holds the tag.

1. `test` and `repo-tests` run the `CI` suite (fmt, clippy, `cargo test`) and the
   repository's `Tests` suite (prek, pytest).
2. `version` fails unless `X.Y.Z` equals `version` in `[workspace.package]` of `Cargo.toml`
   and `scripts/stamp_release.py check` finds every status of the catalog sealed in the history.
3. `draft` creates a draft GitHub Release.
4. `wheels` builds a wheel per platform and attaches it, with a `.sha256`, to the draft: Linux
   x64 and arm64 (glibc and musl), macOS arm64 and x64, Windows x64. Before attaching it, the job
   installs the wheel alone, with no index, and runs `scripts/check_wheel.sh` with it: `botica
   --version` reports the version of the tag, `botica rules list` and `botica recipe list` run,
   and `import botica.sdk` works. The musl wheels run in a `python:3.14-alpine` container. The
   wheel for macOS x64 is built on an arm64 runner and is not run.
5. `publish` makes the release public when every wheel is attached, and the release becomes
   "latest". By hand, `gh release edit vA.B.C --latest` marks another one.
6. `pypi` uploads the wheels to PyPI when the repository variable `PUBLISH_TO_PYPI` is `true`.
7. `mirror` waits until uv resolves the version from PyPI (`scripts/wait_for_pypi.sh`), and pushes
   the mirror with its tag (below).
8. `mirror-check` waits again with `scripts/wait_for_pypi.sh`, and runs the hooks of the mirror at
   the new tag on the commit of the tag
   (`uvx prek try-repo https://github.com/collectiveai-team/botica-pre-commit --rev vX.Y.Z
   --all-files`): prek installs the wheel from PyPI and runs `botica`, `botica-project`,
   `botica-agents-md` and `botica-rubric`. The hooks of this repository run the botica of its
   checkout (ADR 0013), so this is the job that runs the published botica here. When it fails,
   `bump` does not run; the version is published, and the fix is a patch release.
9. `bump` runs `scripts/bump_mirror_rev.py X.Y.Z` on the branch of the release and the tests of the
   repository (`uv run pytest`), and pushes the commit
   `chore(release): the docs name the mirror at vX.Y.Z` to that branch, with no
   pull request. When a test fails, nothing is pushed and the run ends red.

The Linux jobs build in a container that owns `dist/`, so the checksums are written to `sums/`
instead. The two calls to reusable workflows use GitHub's self-repository syntax
(`uses: $/.github/workflows/ci.yml`, github.com only, runner 2.336.0 or newer). `actionlint` 1.7.12 does not know the syntax yet
(rhysd/actionlint issue 711) and reports it; until it does, run it with
`-ignore 'reusable workflow call "\$/.+" at "uses" is not following the format'`.

`mirror-check` installs the wheel of Linux x64 from PyPI through the mirror and runs it. Not
checked: that the wheel for macOS x64 runs (it builds and is published, nothing more).

## PyPI and who may release

PyPI uses trusted publishing: no token is stored, and PyPI accepts an upload only from the workflow
it names. The publisher of the project `botica` is owner `collectiveai-team`, repository `botica`,
workflow `botica-release.yml`, environment `pypi`. The environment `pypi` of GitHub has no reviewer
and lets only tags `v*` use it (**Settings → Environments → pypi**). A published version cannot be
replaced or reused, only yanked.

The tag is the gate. The repository ruleset **botica release tags** (target `refs/tags/v*`) refuses
to create, move or delete those tags for everyone except the team `botique` (**Organization → Teams
→ botique**); to change the rule, **Settings → Rules → Rulesets**. Repository admins can edit the
ruleset. Moving a tag after a failed release (step 5 of "Each release") also needs the exception.

The `botica-pre-commit` project is not on PyPI: prek installs the mirror by cloning its GitHub
repository, and the package in it only depends on `botica==X.Y.Z`.

To rehearse, upload one wheel by hand to test.pypi.org with
`uv publish --publish-url https://test.pypi.org/legacy/ dist/*.whl`. PyPI refuses `linux_x86_64`
wheels: a wheel built on your machine needs `--compatibility manylinux_2_28` (and a container or
`--zig`).

## Each release

1. Set `version` in `[workspace.package]` of `Cargo.toml`, and run `cargo update --workspace`
   at the root. Without it the build, which uses `--locked`, fails; the change to `Cargo.lock` is a
   few lines for the crates of the project.
2. Seal the history: `uv run --no-project --with cyclopts python scripts/stamp_release.py stamp X.Y.Z` appends
   to `catalog/rules/history.toml` the status of each rule that no release has recorded. The `version`
   job runs `check` and fails the tag while one is left.
3. Commit (`chore(release): botica X.Y.Z`). The version of the mirror's package and every `rev`
   are moved by the workflow.
4. Tag and push the branch with the tag:
   `git tag -a vX.Y.Z -m "botica X.Y.Z" && git push origin HEAD vX.Y.Z`.
5. Watch the run. If a platform fails, fix it and move the tag, or delete the draft release and the
   tag and start again. Once `publish` has run, treat the version as final: when `mirror-check`
   fails, fix the cause and release X.Y.Z+1. Do not move the tag of the mirror: prek keeps the clone
   of a `rev` it has installed.
6. When the release carries a change that breaks a project, add it to the notes:
   `gh release edit vX.Y.Z --notes-file notes.md`, with a section `## Breaking changes` above the
   notes that GitHub wrote.

## Changes that break a project

A change breaks a project when a project that worked with the release before fails with this one
and no change of its own: a rule, topic or recipe renamed or removed, a key of `botica.toml` that
means something else, a command or an option removed. Such a change has a `!` in the title of its
pull request (`refactor(references)!: rename the llm topic to ai`, ADR 0010), and the release that
carries it raises the minor version while botica is below 1.0.

`--generate-notes` lists the pull requests by title and says nothing more, so the section
`## Breaking changes` of the notes, written by hand, names each break, what fails and what the
project changes. For a renamed topic:

```markdown
## Breaking changes

- The topic `llm` is now `ai`, with no alias. `botica refs show llm` fails with "no topic, rule or
  standard `llm`", and `[references] replace = ["llm"]` in `botica.toml` fails to load: write
  `ai`. A rule or standard of the project with `references = ["llm"]` fails `botica refs check`.
  The page of the topic moved from `references/llm/` to `references/ai/` on the site.
```

## The mirror repository

`collectiveai-team/botica-pre-commit` holds the hook definitions and a package that depends on the
wheel of the same version, so `rev` pins the binary. Its source is
`distribution/botica-pre-commit/` in this repository. The job `mirror` sets the version and the
`botica==` pin of its `pyproject.toml` and the `rev` of its README (`scripts/bump_mirror_rev.py`),
copies the directory and `LICENSE` into a clone of the mirror, commits (`chore: release X.Y.Z`),
and pushes a **lightweight** tag `vX.Y.Z`: prek does not install an annotated tag of the mirror.
The tags of this repository can be annotated. When the mirror has the tag, the job changes nothing.

The tag must come after the wheel is on PyPI, or prek cannot install the hook environment: the
job waits up to 10 minutes for uv to resolve the version. `scripts/wait_for_pypi.sh` asks uv, not
the JSON API of PyPI, because the JSON API lists an upload before the simple index that uv reads: in
the release of 0.14.2 the JSON API had the version and `bump` failed to install the hook. The wait
between checks doubles from 5 seconds up to 160. A job that fails on this wait is run again with
`gh run rerun <run> --failed`.

In a project, `rev = "vX.Y.Z"`; `prek update` moves it to the newest tag.

### Set up the deploy key once

The job pushes with a deploy key of the mirror that has write access; its private half is the
secret `MIRROR_DEPLOY_KEY` of this repository.

1. `ssh-keygen -t ed25519 -N "" -C "botica release" -f mirror_key`
2. `gh repo deploy-key add mirror_key.pub -R collectiveai-team/botica-pre-commit --allow-write --title "botica release"`
3. `gh secret set MIRROR_DEPLOY_KEY -R collectiveai-team/botica < mirror_key`, then delete both
   files.

## Files that name the mirror's `rev`

- The example of `website/docs/guide/prek-and-ci.md` and the mirror's README name the mirror and
  its `rev`. This repository's own `prek.toml` does not: its hooks of botica run the checkout
  (ADR 0013). The job `bump` moves them; `scripts/bump_mirror_rev.py` fails the release when one of
  them no longer names the mirror on the line above its `rev`.
- The templates of the recipes (`catalog/recipes/files/prek-base.toml` and the workflows of `base`
  and `ci`) write `{{ param.botica-version }}` for the `rev` of the mirror and for each
  `uvx botica@`. Its default is the release of the binary that runs `botica recipe install`, whose
  tag the mirror has once the binary is published, so a release moves nothing in them.
- Not published, and not planned yet: npm, Homebrew, a GitHub Action, standalone binaries.

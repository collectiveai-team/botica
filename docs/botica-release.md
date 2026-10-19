# Releasing botica

`botica` is built from the Rust workspace at the root of the repository and shipped as wheels that carry the binary and the Python module `botica.sdk`. A tag
`vX.Y.Z` runs `.github/workflows/botica-release.yml` (the file name is the one the PyPI trusted
publisher is registered with). `v0.1.0` to `v0.6.0` are releases of the removed `scaffolding`
package, so botica starts at `0.7.0`.

A release does not need `main`: a tag runs the workflow as it is in the tagged commit. The
branch that holds the tag is pushed with it, because the job `bump` commits to that branch: `main`
when the tag is on `main`, or else the one remote branch that holds the tag.

1. `test` and `repo-tests` run the `CI` suite (fmt, clippy, `cargo test`) and the
   repository's `Tests` suite (prek, pytest).
2. `version` fails unless `X.Y.Z` equals `version` in `[workspace.package]` of `Cargo.toml`
   and `scripts/stamp_release.py check` finds every status of the catalog sealed in the history.
3. `draft` creates a draft GitHub Release.
4. `wheels` builds a wheel per platform and attaches it, with a `.sha256`, to the draft: Linux
   x64 and arm64 (glibc and musl), macOS arm64 and x64, Windows x64.
5. `publish` makes the release public when every wheel is attached, and the release becomes
   "latest". By hand, `gh release edit vA.B.C --latest` marks another one.
6. `pypi` uploads the wheels to PyPI. It is **off** until the repository variable
   `PUBLISH_TO_PYPI` is `true`.
7. `mirror` waits until PyPI lists the version, and pushes the mirror with its tag (below).
8. `bump` runs `scripts/bump_mirror_rev.py X.Y.Z` on the branch of the release, runs the hook of
   botica at the new `rev` (`uvx prek run botica --all-files`) and the tests of the repository
   (`uv run pytest`), and pushes the commit
   `chore(release): the hooks of botica point at vX.Y.Z` to that branch, with no pull request. When
   the hook or a test fails, nothing is pushed and the run ends red.

`0.7.0` was released on 2026-10-12 (run `38197175421`), `0.7.1` the same day (run `38214642175`),
`0.8.0` on 2026-10-13 (run `38410864532`) and `0.9.0` on 2026-10-14 (run `38595149595`): every job
passed, the seven wheels are on GitHub and on PyPI, and the `publish` step of that time left
`v0.6.0` as "latest" each time (it was first used on 0.7.1). The
Linux jobs build in a container that owns `dist/`, so the checksums are written to `sums/`
instead. The two calls to reusable workflows use GitHub's self-repository syntax
(`uses: $/.github/workflows/ci.yml`, July 2026, github.com only, runner 2.336.0 or newer), which
worked on GitHub from 0.7.1. `actionlint` 1.7.12 does not know the syntax yet (rhysd/actionlint
issue 711) and reports it; until it does, run it with
`-ignore 'reusable workflow call "\$/.+" at "uses" is not following the format'`.

Checked by hand after each release: `uvx botica@X.Y.Z --version` installs the wheel from PyPI and
runs it (Linux x64; PyPI's index can take a minute to list a new version), and prek installs the
mirror repository at `vX.Y.Z` and runs the hook against that wheel. Not checked: that the wheels for
musl, arm64, macOS and Windows run (they build and are published, nothing more).

## Set up PyPI once

PyPI uses **trusted publishing**: no token is stored, and PyPI accepts an upload only from the
workflow you name. A published version cannot be replaced or reused, only yanked, so do this
before the first tag and publish nothing you have not looked at.

1. Check the names. On 2026-10-11 `botica` and `botica-pre-commit` did not exist on PyPI; `botica`
   was created by the first upload on 2026-10-12. A name is not reserved until something is
   uploaded, so register the pending publisher soon.
2. On PyPI, with the account that will own the project: **Your account → Publishing → Add a new
   pending publisher**. Project name `botica`; owner `collectiveai-team`; repository `botica`;
   workflow `botica-release.yml`; environment `pypi`. A pending publisher creates the project on
   the first upload.
3. On GitHub, an environment named `pypi` with a required reviewer, so every upload waits for an
   approval, and a deployment policy that only lets tags `v*` use it. **Done on
   2026-10-12** (reviewer: `jedzill4`); to change it, **Settings → Environments → pypi**.
4. The repository variable `PUBLISH_TO_PYPI` = `true` turns the `pypi` job on. **Set on
   2026-10-12.** Delete it, or set it to anything else, to turn the job off again.
5. Who may release. A tag `v*.*.*` publishes to PyPI, and the `pypi` environment has no reviewer
   (removed on 2026-10-12), so the tag is the gate. The repository ruleset **botica release tags**
   (target: tags, `refs/tags/v*`, active, created 2026-10-13) refuses to create, move or delete
   those tags for everyone except the team `botique`, which today is exactly `jedzill4` and
   `lionelchamorro`. A ruleset takes roles, teams and apps as exceptions, not single people, so who
   may release is who is in that team (**Organization → Teams → botique**); to change the rule
   itself, **Settings → Rules → Rulesets**. Repository admins can edit the ruleset: it is a gate
   against mistakes, not a lock against an admin. Moving a tag after a failed release (step 5 of
   "Each release") also needs the exception.

The `botica-pre-commit` project is **not** created on PyPI: prek installs the mirror by cloning
its GitHub repository, and the package in it only depends on `botica==X.Y.Z`.

To rehearse, repeat step 2 on test.pypi.org and upload one wheel by hand with
`uv publish --publish-url https://test.pypi.org/legacy/ dist/*.whl`. The workflow publishes
only to PyPI.

PyPI refuses `linux_x86_64` wheels: only the `manylinux`/`musllinux` ones that `maturin-action`
builds are accepted. A wheel built on your machine for a rehearsal needs
`--compatibility manylinux_2_28` (and a container or `--zig`) to be uploaded.

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
   tag and start again. Once `publish` has run, treat the version as final.
6. Approve the `pypi` environment when it asks.

## The mirror repository

`collectiveai-team/botica-pre-commit` holds the hook definitions and a package that depends on the
wheel of the same version, so `rev` pins the binary. Its source is
`distribution/botica-pre-commit/` in this repository. The job `mirror` sets the version and the
`botica==` pin of its `pyproject.toml` and the `rev` of its README (`scripts/bump_mirror_rev.py`),
copies the directory and `LICENSE` into a clone of the mirror, commits (`chore: release X.Y.Z`),
and pushes a **lightweight** tag `vX.Y.Z`: prek does not install an annotated tag of the mirror.
The tags of this repository can be annotated. When the mirror has the tag, the job changes nothing.

The tag must come after the wheel is on PyPI, or prek cannot install the hook environment: the
job waits up to 10 minutes for PyPI to list the version.

In a project, `rev = "vX.Y.Z"`; `prek update` moves it to the newest tag.

### Set up the deploy key once

The job pushes with a deploy key of the mirror that has write access; its private half is the
secret `MIRROR_DEPLOY_KEY` of this repository.

1. `ssh-keygen -t ed25519 -N "" -C "botica release" -f mirror_key`
2. `gh repo deploy-key add mirror_key.pub -R collectiveai-team/botica-pre-commit --allow-write --title "botica release"`
3. `gh secret set MIRROR_DEPLOY_KEY -R collectiveai-team/botica < mirror_key`, then delete both
   files.

## After the first release

- `catalog/recipes/files/prek-base.toml`, this repository's own `prek.toml`, the examples of
  `docs/botica.md` and `website/docs/use.md`, the mirror's README and the rule
  `py-ruff-format-modern` of the skill `engineering-rules` name the mirror and its `rev`.
  The job `bump` moves them all; `scripts/bump_mirror_rev.py` fails the release when one of them
  no longer names the mirror on the line above its `rev`.
- Not published, and not planned yet: npm, Homebrew, a GitHub Action, standalone binaries.

# Releasing botica

`botica` is built from the Rust workspace in `lint/` and shipped as binary-only wheels. A tag
`vX.Y.Z` runs `.github/workflows/botica-release.yml` (the file name is the one the PyPI trusted
publisher is registered with). `v0.1.0` to `v0.6.0` exist from `scaffolding`, so botica starts at
`0.7.0`. While `main` still publishes `scaffolding` releases, do not cut a `v*` tag there that
botica has taken. A botica release does not stay "latest" there: `publish` marks the previous
latest release again (see below).

A release does not need `main`: a tag runs the workflow as it is in the tagged commit, and
`git push origin vX.Y.Z` uploads that commit and its history without creating a branch.

1. `test` and `repo-tests` run the `Botica CI` suite (fmt, clippy, `cargo test`) and the
   repository's `Tests` suite (prek, pytest).
2. `version` fails unless `X.Y.Z` equals `version` in `[workspace.package]` of `lint/Cargo.toml`.
3. `draft` creates a draft GitHub Release.
4. `wheels` builds a wheel per platform and attaches it, with a `.sha256`, to the draft: Linux
   x64 and arm64 (glibc and musl), macOS arm64 and x64, Windows x64.
5. `publish` makes the release public when every wheel is attached. Publishing a draft makes it
   "latest" whatever `--latest=false` said when it was created, and `gh release edit --latest=false`
   does nothing (GitHub picks the newest release again), so the job reads the release that was
   latest before publishing and marks it latest again. By hand that is
   `gh release edit vA.B.C --latest`.
6. `pypi` uploads the wheels to PyPI. It is **off** until the repository variable
   `PUBLISH_TO_PYPI` is `true`.

`0.7.0` was released on 2026-10-12 (run `38197175421`): every job passed, the seven wheels are on
GitHub and on PyPI. The tag moved once on the way: the Linux jobs build in a container that owns
`lint/dist`, so the checksums are written to `lint/sums` instead. The `publish` step that puts
"latest" back was added afterwards and has not run on GitHub yet: look at it on the next release.

Checked by hand after the release: `uvx botica --version` and `uv tool install botica` install the
wheel from PyPI and run it (`botica 0.7.0`, Linux x64), and prek installs the mirror repository at
`v0.7.0` and runs the hook against that wheel. `actionlint` 1.7.12 finds nothing in the workflows.
`zizmor` 1.30.1 reports two low `self-repository` findings in `botica-release.yml`: it wants
`uses: $/.github/workflows/ci.yml` (GitHub's self-repository syntax, July 2026) instead of
`./.github/...`. `actionlint` 1.7.12 rejects the new form for reusable workflows and it cannot be
tried without a release, so the file keeps `./`. Not checked: that the wheels for musl, arm64,
macOS and Windows run (they build and are published, nothing more).

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

The `botica-pre-commit` project is **not** created on PyPI: prek installs the mirror by cloning
its GitHub repository, and the package in it only depends on `botica==X.Y.Z`.

To rehearse, repeat step 2 on test.pypi.org and upload one wheel by hand with
`uv publish --publish-url https://test.pypi.org/legacy/ lint/dist/*.whl`. The workflow publishes
only to PyPI.

PyPI refuses `linux_x86_64` wheels: only the `manylinux`/`musllinux` ones that `maturin-action`
builds are accepted. A wheel built on your machine for a rehearsal needs
`--compatibility manylinux_2_28` (and a container or `--zig`) to be uploaded.

## Each release

1. Set `version` in `[workspace.package]` of `lint/Cargo.toml` (it is `0.7.0`, the first release,
   and `Cargo.lock` already matches; later ones go up from there), and run `cargo update --workspace`
   in `lint/`. Without it the build, which uses `--locked`, fails; the change to `Cargo.lock` is a
   few lines for the crates of the project.
2. Commit it (`chore(lint): release X.Y.Z`) and push. Wait for `Botica CI` to pass.
3. Tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`.
4. Watch the run. If a platform fails, fix it and move the tag, or delete the draft release and the
   tag and start again. Once `publish` has run, treat the version as final.
5. Approve the `pypi` environment when it asks.
6. Update the mirror, below.

## The mirror repository

`collectiveai-team/botica-pre-commit` holds the hook definitions and a package that depends on the
wheel of the same version, so `rev` pins the binary. Its source is
`lint/distribution/botica-pre-commit/` in this repository; copy it over after each release.

1. In `lint/distribution/botica-pre-commit/pyproject.toml` set `version` and the `botica==` pin to
   `X.Y.Z`, and commit it here.
2. Copy that directory (and `LICENSE`) into a clone of the mirror, commit
   (`chore: release X.Y.Z`), push, and tag `vX.Y.Z`.
3. In a project, `rev = "vX.Y.Z"`; `prek autoupdate` moves it to the newest tag.

The tag must come after the wheel is on PyPI, or prek cannot install the hook environment.

## After the first release

- `scaffolding/templates/prek-generic.toml` and this repository's own `prek.toml` use the mirror
  (`repo = "https://github.com/collectiveai-team/botica-pre-commit"`, `rev`, `args`). They are not
  moved by a release: after the mirror has its new tag, set `rev` in both by hand.
- Not published, and not planned yet: npm, Homebrew, a GitHub Action, standalone binaries.

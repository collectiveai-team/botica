# Releasing botica

`botica` is built from the Rust workspace in `lint/` and shipped as binary-only wheels. botica
supersedes `scaffolding`, so a tag `vX.Y.Z` is the release of the repository and runs
`.github/workflows/botica-release.yml` (the old release workflow of `scaffolding` is gone).
`v0.1.0` to `v0.6.0` were `scaffolding`'s releases, so the first botica tag is `v0.7.0` or higher.
`scaffolding` still takes its own version from these tags (`hatch-vcs`).

1. `test` and `repo-tests` run the `Botica lint` suite (fmt, clippy, `cargo test`) and the
   repository's `Tests` suite (prek, pytest).
2. `version` fails unless `X.Y.Z` equals `version` in `[workspace.package]` of `lint/Cargo.toml`.
3. `draft` creates a draft GitHub Release.
4. `wheels` builds a wheel per platform and attaches it, with a `.sha256`, to the draft: Linux
   x64 and arm64 (glibc and musl), macOS arm64 and x64, Windows x64.
5. `publish` makes the release public when every wheel is attached.
6. `pypi` uploads the wheels to PyPI. It is **off** until the repository variable
   `PUBLISH_TO_PYPI` is `true`.

The workflows have not run on GitHub yet. What was checked by hand: a Linux x64 wheel builds with
`maturin`, installs in a clean environment, and `botica --version` runs; prek installs the mirror
repository and runs the hook against that wheel. Not checked: musl, arm64, macOS, Windows, and the
`PyO3/maturin-action` and `pypa/gh-action-pypi-publish` steps. Expect to fix the first run.

## Set up PyPI once

PyPI uses **trusted publishing**: no token is stored, and PyPI accepts an upload only from the
workflow you name. A published version cannot be replaced or reused, only yanked, so do this
before the first tag and publish nothing you have not looked at.

1. Check the names. On 2026-10-11 `botica` and `botica-pre-commit` did not exist on PyPI. A name is
   not reserved until something is uploaded, so register the pending publisher soon.
2. On PyPI, with the account that will own the project: **Your account → Publishing → Add a new
   pending publisher**. Project name `botica`; owner `collectiveai-team`; repository `botica`;
   workflow `botica-release.yml`; environment `pypi`. A pending publisher creates the project on
   the first upload.
3. On GitHub, an environment named `pypi` with a required reviewer, so every upload waits for an
   approval, and a deployment policy that only lets tags `v*` use it. **Done on 2026-10-12**
   (reviewer: `jedzill4`; the policy was `botica-v*` for a few hours); to change it,
   **Settings → Environments → pypi**.
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

1. Set `version` in `[workspace.package]` of `lint/Cargo.toml` (it is `0.0.0` today; the first
   release is `0.7.0` or higher), and run `cargo update --workspace`
   in `lint/`. Without it the build, which uses `--locked`, fails; the change to `Cargo.lock` is a
   few lines for the crates of the project.
2. Commit it (`chore(lint): release X.Y.Z`) and push. Wait for `Botica lint` to pass.
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

- `scaffolding/templates/prek-generic.toml` ships a local hook that needs `botica` on the `PATH`.
  Switch it to the mirror (`repo = "https://github.com/collectiveai-team/botica-pre-commit"`,
  `rev = "vX.Y.Z"`, `args = [...]`) so a new repository installs it by itself, and update
  `tests/test_prek_template.py` with it.
- This repository's own `prek.toml` still has the `python -c` `file-size-guard` hook. Replace it
  with the same hook then; CI cannot run `botica` before there is something to install.
- Not published, and not planned yet: npm, Homebrew, a GitHub Action, standalone binaries.

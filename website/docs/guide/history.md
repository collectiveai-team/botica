---
title: History report
---

# History report

`botica history report` reads the history that [`botica lint --record`](lint.md#history-of-findings)
writes and says which rules found nothing, which rules a comment silences often, which findings
a hook caught before the commit and which reached CI. The design is
[ADR 0019](https://github.com/collectiveai-team/botica/blob/main/docs/adr/0019-finding-history.md).

```bash
botica history report                      # the last 28 days of .botica/history/, as text
botica history report --days 90 --format markdown
botica history report --from ~/.cache/botica-ci   # also the runs of CI downloaded there
```

The command reads `.botica/history/` under the root of the project, whatever the working
directory, and writes nothing. It exits with `0` when it read the history, including an empty one,
and with `2` when it could not run.

| Option | Meaning |
|---|---|
| `--days <n>` | The window: the runs that started in the last `n` days. Default 28. |
| `--format text\|markdown` | Plain text, or markdown with a heading for each section. Default `text`. |
| `--from <dir>` | A folder with one folder for each run of CI. A run folder holds a `source.json` and the two files, in it or in a folder of it for each artifact. Repeat it for more folders. |
| `--min-suppressed <n>` | A rule has many suppressions from this many suppressed findings. Default 5. |
| `--min-share <percent>` | A rule has many suppressions from this share of suppressed findings, over reported plus suppressed. Default 50. |

## What it says

Rules with no findings
: A rule that is in `rules` of at least one trusted run of the window, of any mode and branch,
  counted over the runs that name it, with `hits`, `suppressed` and
  `baselined` at 0 in every run. A rule whose findings are all suppressed or in the baseline is not
  one, and a rule that is absent from `rules` did not run, which is another case. Zero can mean that
  the rule works: the agent may follow it already because `AGENTS.md` lists it. The list is a set of
  candidates for review, and a person decides to retire a rule.

Rules with many suppressions
: A rule whose `suppressed` over `hits` plus `suppressed` reaches `--min-share` with at least
  `--min-suppressed` suppressed findings. The two defaults are this command's own: ADR 0019 leaves
  what counts as many open.

Findings of the roles that `refute` did not confirm
: For a role of the review flow, the findings whose `refutation` is neither `null` nor `confirmed`
  over the findings that `refute` judged. Shown only when a run of a role is in the window.

Caught before the commit
: A finding of a hook run of mode `staged` that the paired CI run of mode `diff` does not hold.

Reached CI
: A finding of a CI run of mode `diff` that no hook run of mode `staged` before it holds. It is a gap
  of the local layer or a commit made with `--no-verify`, and the history cannot tell them apart.

The two paired readings need a hook that runs `botica lint --staged --record` with
`pass_filenames: false`: the hooks of the mirror run in mode `paths`. They need a CI job of `botica
lint --diff <base> --record` too, and the local history of the person who runs the report, since
hook runs never leave the checkout. A hook run and a CI run are paired when they have the same
branch and the `commit` of the hook run is a proper ancestor of the `head` of the CI run; when the
clone does not hold the commit, the time of the runs decides. The report says which of the two
sides is missing when a reading has no runs.

A rule takes part in a pairing only when both runs record the same `hash` for it. Until `botica
lint` computes that hash it records `null`, which the report reads as not comparable: it lists the
rule under "Rules that are not comparable" and counts none of its findings, and it does not
invent a hash.

## Trust

A run in `.botica/history/` counts as trusted for the person who wrote it, and the report names it
apart from the runs of CI. A run under `--from` is trusted when its `source.json` has
`"trusted": true`; a folder with no readable `source.json` is not trusted. The rules with no
findings and the noise of a rule read trusted runs only, since a person with write access to a
branch can upload lines. The paired readings also read runs that are not trusted, and the hook
runs they need are local ones only.

`botica history fetch` is not part of botica yet, so nothing in botica writes `source.json`. Whoever
downloads the artifacts writes it from the answer of the API of the CI system, and a `source.json`
that the artifact itself carried would let the author of a branch mark a run as trusted. The
files of an artifact may sit in `<id>/` or in a folder `<id>/<name>` of it.

The report skips a line that fails the checks of ADR 0019 decision 9.4 and counts the skipped
lines. It skips a file that is not a regular file, one over 50 MB, and what passes 2 GB in all, and
names each one, so that a cut history does not look complete. In GitHub Actions the output sits
between `::stop-commands::<token>` and `::<token>::`, and in markdown each value of the history
is inside code.

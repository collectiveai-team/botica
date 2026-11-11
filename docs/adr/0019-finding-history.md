# A run of botica lint can append its findings and the rules it evaluated to a local history

**Status:** proposed

The result cache of `botica lint` is keyed by the text of each file and is discarded when the
version, the configuration or the active rules change (`crates/botica-core/src/cache.rs`, and
`fingerprint` in `crates/botica-cli/src/rules.rs`, which hashes `Project::fingerprint` of
`crates/botica-cli/src/project/mod.rs`). It says whether a file was linted before and holds nothing
about a finding from one run to the next. Three facts that the review flow needs are missing:

- A finding has no identity. Nothing says that a finding of one run is the finding of the next run
  after the lines above it moved.
- A suppression is not counted. `Policy` (`crates/botica-core/src/suppress/mod.rs`) drops the
  findings that a comment silences and reports none of them, and the baseline of ADR 0011 hides the
  findings that it holds.
- A run does not say which rules ran. A rule with no findings in a month and a rule that never ran
  look the same, so nobody can say that a rule stopped being relevant.

The fingerprint of the cache cannot pair two runs either. `Project::fingerprint` hashes the list of
active rules after `--scope`, so the hook `botica` of the mirror (`botica lint --scope file`,
`distribution/botica-pre-commit/.pre-commit-hooks.yaml`), the hook `botica-project` (`--scope
project`) and a CI job with no `--scope` never share one, even with the same version and the same
`botica.toml`.

ADR 0018 decision 13 needs a place for the records of the classifier in shadow mode, and ADR 0017
produces findings of roles that nothing keeps after the pull request closes. This ADR defines one
history for all of them: what a run writes, how a finding is followed from run to run, how two
runs are compared, which origins write it, and how the files of CI jobs are joined.

## Decision

1. Recording is opt-in. `botica lint --record`, or `[history] record = true` in `botica.toml`,
   appends to two files in `.botica/history/` under the root of the project: `runs.jsonl`, one line
   per run, and `findings.jsonl`, one line per finding. `--no-record` turns it off for one run. The
   key applies to `botica lint` and to the commands of ADR 0017 and 0018 that record (decision 7);
   `botica rules eval` records nothing. The hooks of the mirror pass no `--record`, so a repository
   turns recording on for its hooks with the key.

   A run is recorded when it linted, whatever its exit code. A run that stops on an error of the
   configuration before the lint writes nothing. A history that cannot be written prints a warning
   and leaves the exit code as it was, as the cache does (`Cache::save`).

   The directory is created with a `.gitignore` that holds `*`, as `ensure_directory` in `cache.rs`
   does for the cache; when the directory exists without it, the writer adds it. The writer opens
   each file for append and refuses a `.botica`, a `.botica/history` or a file that is a symbolic
   link or not a regular file, so that a link in a checkout cannot redirect the append. When a file
   does not end in `\n`, the append starts with one.

2. Each file is JSON Lines: one object per line, UTF-8, ended by `\n`. botica only appends; it never
   rewrites or deletes a line. A run writes its finding lines first and its run line last, one
   append for each file. Every line has the key `v`, set to `1` by this ADR. A later change that
   adds keys keeps `v`; a change that alters the meaning of a key raises it. A reader ignores a key
   and a value of `mode` or `origin` that it does not know, and skips a line that does not parse or
   whose `v` it does not know.

   A reader ignores a finding whose `run_id` has no run line (a run that died half way). A run line
   equal to an earlier one is read once, which happens when the same artifact is downloaded twice;
   two different run lines with one `run_id` are both ignored, with a warning. A finding line is
   read once for each `run_id`, `fingerprint` and `occurrence`.

   The files hold paths, numbers, hashes and values of closed sets. They hold no message of a
   finding and no text of the code.

3. A run line has these keys:

   | Key | Value |
   |---|---|
   | `run_id` | 16 hex digits: the first 64 bits of the blake3 of the start time in nanoseconds, the process id and a counter of the process |
   | `started_at` | Unix seconds, an integer |
   | `commit` | `git rev-parse HEAD`, or `null` outside git. In `staged` mode this is the parent of the commit that the hook checks |
   | `head` | the commit under review in CI: `github.event.pull_request.head.sha` through `GITHUB_EVENT_PATH` on a pull request event, `CI_MERGE_REQUEST_SOURCE_BRANCH_SHA` on a GitLab merge request pipeline, and `commit` otherwise |
   | `branch` | `GITHUB_HEAD_REF` when it is set (GitHub sets it on `pull_request` and `pull_request_target`, with the branch of the pull request); otherwise `git branch --show-current`; with a detached `HEAD`, `CI_MERGE_REQUEST_SOURCE_BRANCH_NAME` and then `CI_COMMIT_REF_NAME`; otherwise `null` |
   | `botica` | the version of botica |
   | `rules_fingerprint` | the fingerprint that keys the result cache, computed on the configuration without its `[history]` table. It changes with `--scope`, `--select` and `--ignore`, and decision 8 does not pair runs with it |
   | `key_id` | decision 5 |
   | `mode` | decision 6 |
   | `origin` | decision 6 |
   | `rules` | an object from the slug of each evaluated rule to its counts and its hash, below |
   | `detail` | optional; an object that an origin adds, decision 7 |

   The entry of a rule in `rules` is `{"hits": h, "suppressed": s, "baselined": b, "hash": x}`.

   A rule is evaluated when the engine applied it to at least one file of the run, after the
   prefilter and the plan; when it ran, for a rule of project scope; and when it checked at least one
   message, for a rule of scope `commit` in a run of mode `message`. A rule that the configuration
   or `--scope` turns off, that the run never reached (the files of a `diff` run held none that the
   rule reads), or whose extension failed is absent from `rules`. This is the denominator: a rule
   that is present with its three counts at 0 ran and found nothing.

   The counts are taken over the findings that the rule produced and that the filter of `--diff`
   and `--staged` keeps (`Changes::keeps` in `crates/botica-core/src/diff.rs`). `hits` counts those
   that the run reports, `suppressed` those that a suppression channel silenced, and `baselined`
   those that the baseline hides.

   `hash` is 16 hex digits of the blake3 of what can change the findings of that one rule: the
   version of botica, the source of the rule as `rules::fingerprint` reads it (its YAML and its
   prefilter, or the `rule.toml` and the script of an extension rule), and the settings of the rule
   in the resolved configuration (`[rules.<id>]` and the `[[overrides]]` that name it). `--scope`,
   `--select` and the rules that run beside it do not enter it, so a rule has the same `hash` in a
   hook with `--scope file` and in a CI job with no scope when the version and its settings agree.

4. A finding line has these keys:

   | Key | Value |
   |---|---|
   | `v`, `run_id` | as in decision 2, the run that reported it |
   | `code`, `slug` | the code and the slug of the rule. A finding of a role has `code` `null` and the slug `review:<role>` |
   | `severity` | as the source writes it: `error`, `warning`, `info` or `hint` for a rule, and `P0` to `P3` for a role (ADR 0017 decision 5). The origin says which scale applies |
   | `path` | relative to the root of the project, with `/` separators, whatever the working directory of the run; empty for a finding about a whole change, a finding of mode `message`, and a path that decision 7 drops |
   | `line` | 1-based; 1 for a finding about a whole file, and 0 when `path` is empty |
   | `fingerprint` | decision 5 |
   | `occurrence` | 0 for the first finding of the run with that fingerprint, 1 for the next, in order of line |

   A finding of a role also has `ref`, the `id` of the finding in the JSON of the role, and
   `refutation`, the value that `refute.json` gave it (`confirmed`, `pre-existing`, `unreachable`,
   `not-reproduced` or `duplicate`), or `null` when `refute` did not judge it.

5. The fingerprint follows a finding when it changes line. It is 16 hex digits of the keyed blake3 of
   five fields, each preceded by its length in bytes as a 64-bit little-endian integer: the slug, the
   path, the nearest non-blank line above the finding, the line of the finding and the nearest
   non-blank line below it. Each line is trimmed and its runs of whitespace become one space. The
   line number is not an input, so a finding that moves keeps its fingerprint, and the neighbors
   tell apart two equal lines of one file.

   The text is the text that the run linted: the file on disk, or the staged blob in `staged` mode.
   Three cases have no line text, and the normalized message of the finding (trimmed, with its runs
   of whitespace turned into one space) takes the place of the three lines: a finding about a whole
   change, a finding of mode `message`, and a finding of a role, whose `title` stands for its
   message, since the line that a role gives is the line that the model read. For a finding about a
   whole file the three lines are empty.

   The key of the hash comes from the variable `BOTICA_HISTORY_KEY`: the blake3 key derived
   (`blake3::derive_key`, context `botica history fingerprint v1`) from its value. With the
   variable unset or empty, the key is derived from the empty string. The run line records `key_id`,
   the first 8 hex digits of the blake3 of the derived key, or `null` with no variable. A reader
   compares fingerprints only between runs with the same `key_id`. A line of code that holds a
   secret of low entropy, or a secret that someone already suspects, can be confirmed from an
   unkeyed fingerprint by trying candidates; a key that the artifact readers lack closes that. A
   repository whose artifacts a person outside it can download (a public repository) sets the
   variable as a secret of the recording jobs and gives it to the developers whose hooks record.

   A rule that is renamed, a file that is renamed, and an edit of the line or of a neighbor each
   start a new fingerprint; the history then shows one finding gone and another new.

6. The mode says what the run read, and the origin says who started it:

   | `mode` | A run that |
   |---|---|
   | `full` | walks the project |
   | `paths` | lints the files named on the command line, which is what the hook `botica` of the mirror passes |
   | `diff` | uses `--diff` or `--diff-file` |
   | `staged` | uses `--staged` |
   | `message` | uses `--commit-msg`, `--commits` or `--pr-title` |
   | `judged` | uses `--judged` |
   | `reviewed` | uses `--reviewed` (ADR 0017; the flag comes in a later change) |
   | `mutants` | uses `--mutants` (planned; the flag comes in a later change) |
   | `classify` | is a decision of `botica review classify` (ADR 0018) |

   | `origin` | When |
   |---|---|
   | `ci` | `GITHUB_ACTIONS` or `GITLAB_CI` is `true` |
   | `hook` | otherwise, `PRE_COMMIT=1` is set, which prek and pre-commit export to the processes of a hook (<https://prek.j178.dev/reference/environment-variables/>) |
   | `cli` | otherwise |
   | `review:<role>` | the run records the findings of one role (decision 7) |
   | `classify` | the run records a decision of the classifier (decision 7) |

   `ci` is tested before `hook`, so `prek run` inside a CI job counts as CI. The command that records
   sets `review:<role>` and `classify`; they are never detected. A CI system other than those two
   records `cli` until the detection adds it.

7. Two commands of the review flow record runs of their own, with `detail`:

   - `botica lint --reviewed <dir>` reads the JSON files of the roles (ADR 0017 decision 5) and
     writes one run for each role, with `origin` `review:<role>`, `mode` `reviewed`, `rules` with
     one entry `review:<role>` whose `hits` is the number of findings and whose `hash` is the
     blake3 of the `REVIEW.md` that the role read, and `detail` `{"verdict", "base", "stage"}`:
     `verdict` as the job computed it (ADR 0017 decision 9), `base` the base commit of the role
     file when it is 40 hex digits, and `stage` `local` or `ci` by the detection of decision 6. The
     run of `refute` has its `verdict` and no finding lines; `--reviewed` copies the `refutation`
     of `refute.json` onto the finding of the other role that it names.
   - `botica review classify --record` writes one run with `origin` and `mode` `classify`, no
     finding lines, an empty `rules`, and `detail` with the probabilities, the result and whether a
     merge would have happened (ADR 0018 decision 13), and the reasons of the handoff, each as the
     condition of the column `Reason` of ADR 0018 decision 10. The evidence and the steps of the
     handoff report hold file names, log lines and titles; they stay in the report that its job
     uploads and are not copied here. The answer of the person and a later revert go to a third
     file, `events.jsonl`, whose keys are decided with that command; each line names the `run_id`
     of the decision.

   The file of a role is written by a model, so `--reviewed` checks what it copies. It records a
   role only when the name matches `[a-z][a-z-]*` and is a role of the plan of ADR 0017 decision 8.
   It keeps the `path` of a finding only when the path is relative, has no `..` component and names
   a regular file of the checkout of the head that is not a symbolic link and has no symbolic link
   among its parent directories; otherwise `path` is empty and `line` is 0. It keeps `ref` only when
   it matches `[a-z][a-z-]*-[0-9]+`, and a `severity` only when it is `P0` to `P3`; a finding with
   another severity is not recorded and the run counts it in `detail.dropped`.

8. A comparison of two runs is made per rule. A rule takes part when it is in `rules` of both runs
   with the same `hash`, and a finding counts only for the rules that take part. Two runs with
   different `--scope`, `--select` or version share the rules whose source and settings agree, and a
   rule whose `hash` differs between them is left out of the comparison and listed by the report as
   not comparable. Findings are compared by `fingerprint` among runs with the same `key_id`.

   What the history lets a report read:

   | Reading | Runs and fields |
   |---|---|
   | Caught before the commit | a finding of rule R in a hook run of mode `staged` whose fingerprint the paired CI run of mode `diff` does not hold, R taking part in both. This is the main sign that a rule works |
   | Reached CI | a finding of rule R in a CI run of mode `diff` whose fingerprint no paired hook run of mode `staged` that precedes it holds, R taking part in both. It is a gap of the local layer, or a commit made with `--no-verify`; the history cannot tell them apart |
   | Noise | for a rule, `suppressed` over `hits` plus `suppressed`, undefined when both are 0; for a role, the findings whose `refutation` is neither `null` nor `confirmed` over the findings whose `refutation` is not `null`, undefined when there are none |
   | A rule with no findings | the rule is in `rules` with `hits`, `suppressed` and `baselined` at 0 in every run of the last N weeks, and in at least one run. A rule whose findings are all suppressed or in the baseline is not one. A rule that is absent from `rules` did not run and is another case |

   A hook run and a CI run are paired when they have the same `branch`, which is not `null`, the CI
   run has `origin` `ci`, and the hook run precedes the CI run. A hook run precedes a CI run when its
   `commit` is an ancestor of the `head` of the CI run, or is that commit. When the clone does not
   hold the `commit` of the hook run, or holds it off the history of `head` (an amend or a rebase
   rewrote it), the hook run precedes the CI run when its `started_at` is earlier than the
   `started_at` of the CI run. Only those hook runs depend on the clocks of two machines. The
   pairing reads `head` and never the merge commit of a `pull_request` run, which a clone does not
   hold, and it needs a clone that holds the branch. The CI run paired with a hook run is the
   earliest CI run of the branch that the hook run precedes.

   The two paired readings use mode `staged` on the local side because `--staged` applies the
   filter of `--diff`: a finding with a range stays when the range touches an added line, on both
   sides. A hook run of mode `paths` reports whole files, so a finding that was already in a touched
   file would count as caught. A repository that wants these readings runs a hook with `--staged`
   and `pass_filenames: false`; the hooks of the mirror run in mode `paths` today and feed the other
   readings. A rule of project scope in a `staged` run with `--scope project` takes part like any
   other rule, since `--scope` does not enter its `hash`.

   The hook runs stay in the local `.botica/history/`. A developer computes the paired readings by
   joining the local files with the files that decision 9 downloads, and only for the branches that
   have local hook runs, so the CI runs of another author's branch take part in no pair.

   A rule with no findings is a candidate for review, and a person decides to retire it. Zero can
   mean that the rule works: the agent may follow it already because `AGENTS.md` lists it. The runs
   of the default branch with mode `full` give the base of the readings that pair no hook. A finding
   that an edit next to it gave a new fingerprint counts as caught and as reached CI at once.

9. Joining the history of CI:

   1. A job that records uploads `runs.jsonl` and `findings.jsonl` as an artifact named
      `botica-history-<job>`, with the matrix values in the name, since `actions/upload-artifact`
      refuses two artifacts of one run with the same name
      (<https://github.com/actions/upload-artifact#not-uploading-to-the-same-artifact>). The step
      runs with `if: ${{ !cancelled() }}`, so a lint that failed still uploads its run, lists both
      files and sets `include-hidden-files: true`, because the action skips every file under a
      folder that starts with a dot. On GitLab the job sets `artifacts:when: always`. The retention
      follows the setting of the repository, 90 days by default on GitHub.
   2. `botica history fetch --since <days>` downloads the artifacts. The scheduled job and a
      developer run the same command, so the checks below hold for both. On GitHub it lists the
      runs of the workflows that record with `gh api --paginate
      repos/<repo>/actions/workflows/<file>/runs?created=>=<date>`, which returns `event` and
      `head_repository` for each run. It keeps a run whose `event` is `push`, `schedule` or
      `workflow_dispatch`; a run of `pull_request` whose `head_repository.full_name` is this
      repository; and a run of `pull_request_target` of the workflows of ADR 0017 and ADR 0018,
      whose file comes from the base branch and whose recording jobs run only for a pull request of
      this repository (ADR 0017 decision 9, ADR 0018 decision 11). It skips every other run: on a
      `pull_request` from a fork, the author edits the workflow that uploads the artifact.
      On GitLab it lists the pipelines of the project through its API and keeps a pipeline whose
      `source` is `push`, `schedule` or `web`, and one whose `source` is `merge_request_event` when
      the merge request has `source_project_id` equal to `target_project_id`; it skips the others.
   3. It downloads each kept run with `gh run download <id> --pattern 'botica-history-*' --dir
      <dir>/<id>`, where `<dir>` is a new empty directory that it creates outside the checkout with
      the permissions of its user only, and with `gh` 2.102.0 or later: earlier releases follow a
      symbolic link under the destination while extracting an artifact (GHSA-39wj-f2f4-978v). It
      stops when `gh` is older. On GitLab it downloads the archive of each job through the API and
      extracts only the two file names, refusing a member that is a link or has a `..` component.
   4. `botica history report` reads the joined files. It reads only regular files named
      `runs.jsonl` and `findings.jsonl`, each of at most 50 MB, at most 100 of each and 500 MB in
      all. It reads each file on its own and does not concatenate them, since a file with no final
      newline would glue its last line to the first line of the next. For each key that it knows it
      checks that `run_id` and `fingerprint` are 16 hex digits and `key_id` 8 hex digits or `null`;
      that `origin`, `mode`, `severity` and `refutation` are values of their tables; that `commit`
      and `head` are 40 hex digits or `null`; that `path` is relative, has no `..` component and no
      control character; that a `slug` and a `branch` have no control character and fit in 256
      bytes; and that `line`, `started_at` and the counts are non-negative integers. A line that
      fails is skipped and counted. The report escapes the values that it prints, runs nothing that
      a line holds, and starts no line of its output with `::`, which the runner of GitHub reads as
      a workflow command.
   5. A history longer than the retention of artifacts needs a store that is not an artifact; this
      ADR does not choose it.

10. `botica history report` is the reader of the history, with the checks of decision 9.4 for the
    local files as for the downloaded ones. Until it exists, a person may read the local files of
    the checkout with `jq`; no job reads downloaded files without it. If the queries grow, the
    report can load the files into an in-memory SQLite database; the files stay the source.

## Considered options

- SQLite as the file. Rejected: the history only appends, the files of CI jobs join by reading one
  after another, and SQLite adds a dependency and a binary file that merges badly. The in-memory
  load of decision 10 stays open.
- One file for runs and findings. Rejected: every finding would repeat the keys of its run, and a
  reader would scan the findings to list the runs.
- A file for each run, in a directory. Rejected: a hook writes one on every commit, and the
  directory grows with files that a reader opens one by one.
- Pairing runs by `rules_fingerprint`. Rejected: it hashes the active rules after `--scope`, so a
  hook with `--scope file` and a CI job with no scope never match, and the paired readings would be
  empty with no error. The `hash` of each rule (decision 3) compares what both runs share.
- Pairing runs by branch and ancestry alone, with no per-rule check. Rejected: a rule that a new
  release or a setting changed between the hook and CI would show its new findings as reached CI.
- Pairing by `started_at` alone. Rejected: the hook and CI run on two machines with two clocks; the
  ancestry of `head` orders them, and the time is a fallback for a rewritten commit.
- Joining the files with `cat`. Rejected: a file with no final newline glues its last line to the
  first line of the next file, and the run line that follows is lost with its findings.
- A fingerprint that includes the line number. Rejected: a finding changes line each time someone
  edits above it, and the history would count it as new.
- A fingerprint of the whole file. Rejected: any edit of the file would start every fingerprint of
  that file.
- A fingerprint without the neighbor lines. Rejected: two equal lines of a file would share it. The
  neighbors cost what decision 5 states: an edit next to a finding starts a new one.
- An unkeyed fingerprint. Rejected for a repository whose artifacts leave it: a candidate secret
  can be confirmed against it. Without the variable the history works and is unkeyed.
- A salt per checkout, kept in `.botica/history/`. Rejected: the hook and CI would hash with
  different salts, and no fingerprint of a hook run would match one of CI.
- Lines for the suppressed findings. Rejected: the readings need the count of each rule, which
  `rules` holds, and a line for each suppression grows the file with findings that nobody reports.
- Recording on by default. Rejected: it would write in every checkout and every hook of a project
  that has not asked. A later ADR can turn it on with data behind it.
- Storing the message of a finding or the text of its line. Rejected: the files travel as artifacts
  and a line of code can hold a secret.
- An ISO 8601 timestamp. Rejected: formatting a date needs a date dependency that the workspace does
  not have, and an integer sorts and converts with `jq`.
- A separate history for the classifier. Rejected: ADR 0018 decision 13 says that the history of
  findings holds its records, and one reader of one format is enough.
- Keeping only the runs of `push`, `schedule` and `pull_request`. Rejected: the roles of ADR 0017
  and the classifier of ADR 0018 run on `pull_request_target`, and their runs would never reach the
  report.

## Not decided

- The output of `botica history report`, the N weeks of a rule with no findings, and what counts as
  many suppressions.
- The rotation or pruning of the local files, which grow with every run.
- The keys of `events.jsonl`, decided with `botica review classify`.
- What `rules` holds for a `judged` run, until `--judged` records anything.
- How a report follows a rule through a rename (ADR 0016 keeps the old ids as aliases) and a finding
  through a renamed file.
- Storage beyond the retention of artifacts: an orphan branch `botica-history` or a bucket.
- The detection of `origin` on CI systems other than GitHub Actions and GitLab CI, and a `pr` key for
  the number of the pull request of a CI run.
- How a developer receives `BOTICA_HISTORY_KEY`, and the rotation of the key, which starts a new
  `key_id` and leaves the runs before it out of the comparisons.
- The JSON Schema of the `[history]` table in `schema/botica.schema.json`.

## Consequences

- The engine has to report what it does not report today: the rules that ran on each file and the
  findings that a comment or the baseline silenced. The result cache stores only diagnostics for
  each file, so its entries gain both, and the marker in `rules::fingerprint` (`diagnostic-v2`)
  changes, which discards the caches of the release before.
- The history is local to a checkout. Each worktree under `.worktrees/` has its own
  `.botica/history/`, and a hook run in one is invisible from another.
- A `botica.toml` with a `[history]` table fails to load in a release that predates the table,
  since the configuration rejects unknown keys.
- A line gives a rule, a path, a number and hashes, which is enough to count and to follow a
  finding; for what a run said, the report points to the log or the artifact of its job.
- The paired readings need a hook of `--staged`, while the hooks of the mirror run on the files
  that prek passes. A repository adopts the readings by adding that hook.
- A hook and CI on two releases of botica compare only the rules that the release did not change,
  and the report lists the others as not comparable.
- A finding is lost to the history when a rule, a file or a neighbor line changes, and the report
  reads that as a finding gone and a new one.
- The paired readings need a clone and the local files of a developer. The scheduled job alone
  computes the readings that pair no hook: noise, rules with no findings, and the findings of the
  roles.
- The default branch needs a scheduled run with `--record` so that the runs of mode `full` of
  decision 8 exist.
- A repository that sets `BOTICA_HISTORY_KEY` holds one more secret, and a developer without it
  writes runs whose `key_id` is `null`, which no comparison pairs with the keyed runs of CI.
- The retention of artifacts bounds what `botica history fetch` can read: 90 days by default on
  GitHub.

The changes that follow, one pull request each: the writer, the fingerprint and `--record` for the
modes that exist; the counts of the engine, the `hash` of each rule and the cache entries that hold
them; the upload in CI and `botica history fetch`; `botica history report`; and the recording of
`--reviewed` and of the classifier with the commands that add them.

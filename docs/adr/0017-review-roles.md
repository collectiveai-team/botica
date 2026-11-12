# A review role is a skill and a REVIEW.md that CI reads from the base branch

**Status:** proposed

A review by an agent asks different questions of the same diff: does it do what the issue asked,
does it open a security hole, do the tests assert anything, does it change a contract. One prompt
for all of them gives a review that is shallow on each. Each question is a role, and a role runs
in two places: on the machine of the author before the pull request, and in CI on the pull request.

The repository has the first piece since the change that added `skills/review/`: the skills
`review-spec`, `review-security`, `review-tests`, `review-architecture`, `review-contracts`,
`review-performance` and `review-refute`, and `review-panel`, which runs them. Each role skill
ships a `references/REVIEW.md` with the generic text of its role and a `references/output.md` with
the JSON that it writes; `review-panel` ships no `REVIEW.md`. What is missing is the contract between a skill, the text that a repository sets
for its role and the runner of CI: where each file lives, what its frontmatter says, which JSON
comes out, and which copy of the text a run trusts.

ADR 0008 puts the text of a rule in botica and leaves a skill with a workflow. A role has both
kinds of text: a procedure (how the agent reviews, which commands it runs) and what this
repository cares about (its invariants, its sensitive paths, what not to report). The two have
different authors and different places.

abide cannot run a role. Version 0.0.11 of `@coldtea/abide` takes no instructions file: `abide
check` judges the diff with the rules of `.abide/rubric.json` and `~/.abide/global.json` and reads
nothing else (`src/commands/check.ts`, `src/lib/loadRules.ts`). With a `REVIEW.md` and no rubric it
stops with `RUBRIC_MISSING`, and `abide check --rules REVIEW.md` is an unknown option. A verdict has
a rule id, a probability and a band, and no message, line or suggestion (`abide-schema/src/verdict.ts`).

## Decision

1. Two pieces for each role. The skill `review-<role>` holds the procedure; botica writes it and
   `npx skills` installs it into `.agents/skills`. The file `.github/review/<role>/REVIEW.md` holds
   what the role checks in this repository; botica installs a generic version and the owners of
   the repository adjust it. The skill reads the `REVIEW.md` of its role as input, so a local run
   and a CI run review against the same text, and the skill adds what only a local run can do:
   run the tests, reproduce a case, open the application. A skill states no rule (ADR 0008); when
   the role needs a rule of botica it names the slug and runs `botica explain`.

2. A role is a folder. `.github/review/<role>/` holds `REVIEW.md` and may hold more files for the
   role (examples of pull requests, eval cases, a list of invariants). `.github/review/README.md`
   holds what every role shares: the adversarial posture, the verdicts, the severities and the
   output format of decision 5. On GitLab the folder is `.gitlab/review/`; `[review] dir` in
   `botica.toml` sets another path.

3. The roles are `spec`, `security`, `tests`, `architecture`, `contracts`, `performance` and
   `refute`:

   | Role | Runs when | Looks for |
   |---|---|---|
   | `spec` | always | the criteria of the issue against the evidence in the pull request |
   | `security` | always, with more depth at tier `high` | authentication, data of another tenant, secrets, injection |
   | `tests` | the diff touches tests or `core` | weak assertions, deleted tests, `\|\| true`, lowered thresholds |
   | `architecture` | `core` and `platform` | layers, BTARCH rules, abstractions with one use |
   | `contracts` | the diff touches what another program consumes | a change of meaning with the same shape |
   | `performance` | queries, jobs or loops over large data | cost and scale |
   | `refute` | always, last | tries to refute each finding of the other roles in a clean session |

   `contracts` leaves the shape of a schema to the compatibility jobs (`openapi-compat`,
   `api-compat-python`, `asyncapi-compat`). It covers a change of meaning with the same shape
   (units, defaults, order, what `null` means, error codes, idempotence), the contracts with no
   schema (migrations, environment variables, CLI flags, file formats, the public interface of a
   module) and a new contract that no compatibility job compares, for which it suggests the job.
   A repository with no contract outside those jobs does not run it.

4. A `REVIEW.md` has a frontmatter for the wiring and a free body for the model:

   ```markdown
   ---
   role: security
   applies:
     paths: ["src/**/auth/**", "**/*.sql"]
     tiers: [medium, high]
     signals: [new-endpoint, new-dependency]
   stages: [local, ci]
   blocking: P1
   owners: ["@org/platform"]
   max-findings: 10
   ---

   ## Invariants
   ## Check always
   ## Do not report
   <!-- botica:covered -->
   <!-- /botica:covered -->
   ## Evidence
   ```

   | Key | Value | Default |
   |---|---|---|
   | `role` | the name of the folder | required |
   | `applies.paths` | globs; the role applies when a changed file matches one | empty: the whole diff |
   | `applies.tiers` | `low`, `medium` or `high`; the tier that `botica review tier` gives the change | all |
   | `applies.signals` | facts that botica detects in the diff, such as `new-endpoint` | none |
   | `stages` | `local`, `ci` or both | both |
   | `blocking` | the severity from which a finding fails the role | `P1` |
   | `owners` | the people or teams that answer for the file | none |
   | `max-findings` | the most findings the role reports, the most severe first | 10 |

   A role applies when every key present holds: a changed file matches `paths`, the tier is in
   `tiers`, and at least one of the `signals` is detected. The headings of the body are a
   convention that the generic files follow. The skills read one section by name,
   `## Do not report`.

5. Every role writes the same JSON, locally to `.tmp/review/<short-head-sha>/<role>.json` and in
   CI as a file of the artifact that `botica lint --reviewed <dir>` reads, as `--judged` reads the
   verdicts of abide. The format is the one in `skills/review/*/references/output.md`:

   - top level: `role`, `head`, `base`, `verdict`, `findings`, `checked`, `not_checked`, `commands`;
   - each finding: `id`, `severity`, `path`, `line`, `title`, `evidence`, `reproduced`
     (`yes`, `no` or `not-tried`), `how_to_verify` and `fix`;
   - `verdict` is `PASS`, `FAIL` or `INCONCLUSIVE`; `FAIL` means a finding at or above `blocking`, and for `refute` a confirmed finding at or above it;
   - severity is `P0` (breaks production, loses data or opens a security hole on the normal path),
     `P1` (wrong behaviour that a user or a caller reaches), `P2` (wrong on an edge case, or a gap
     that makes a later bug likely) or `P3` (worth knowing, never blocks);
   - `refute.json` keeps each input finding with its `id` and adds `refutation`: `confirmed`,
     `pre-existing`, `unreachable`, `not-reproduced` or `duplicate`.

   A role that ends without a valid file counts as `INCONCLUSIVE`.

6. The block `botica:covered` lists what the repository already checks for the role: the active
   rules of botica for its family, the hooks of prek and the jobs of CI. The role reports none of
   it. botica writes the text between the two markers, as it writes the block `botica:rules` of
   `AGENTS.md`, and the owners write the rest of the section. For `contracts` the block lists the
   installed compatibility jobs. `botica review check` fails on a block that is out of date.

7. A run reads the roles from the base branch. In CI, the job checks out the base commit for
   `.github/review/` and the head commit for the code under review. Locally, the skill runs `git
   show <base>:.github/review/<role>/REVIEW.md` and falls back to its own
   `references/REVIEW.md` when the base has no file. In CI a role that is expected and has no file on
   the base fails the job, as the next paragraph says. A pull request that changes
   `.github/review/` runs with the roles of the base and also goes to the owner of `platform`. The author of a pull
   request would otherwise write the instructions that judge it.

   The same holds for everything else that decides what runs or what a role reads. The job takes
   from the base commit the whole folder `.github/review/` (the folder of each role, the README
   and any `INVARIANTS.md`), `botica.toml` (the `[review]` tables, among them `dir` and the areas),
   `skills-lock.json` and the workflow `review.yml`; it takes the code under review from the head
   and runs a release of botica that the workflow pins. A role is expected when the list `roles` of
   the `[review]` table of `botica.toml` on the base names it, and that list holds the seven roles
   of decision 3 when the table or the key is missing. The plan skips an expected role whose
   `applies` does not hold, and a role that the plan selects and that has no file on the base fails
   the job; it is not skipped, and `botica review check` reports it before. `review.yml`,
   `skills-lock.json`, `botica.toml` and `.github/review/` go to the owner of `platform` through the CODEOWNERS file
   (the file and its entries are a later change).

8. `botica review plan --diff <base>` is the only decision of which roles run. It prints the tier
   and the roles as JSON, from `applies` and `stages` of the files on the base. The local gate
   (`engineering-pr-review`, through `review-panel`) and the CI workflow both read it, so both run
   the same roles. Today `engineering-pr-review` runs the skill `code-review` and not
   `review-panel`; a later change rewires it. `botica review check` validates the frontmatter,
   that each `paths` matches a file, that each `botica:covered` block is current and that each
   role has its skill. Both commands come in later changes.

9. The contract of a runner in CI is: it receives the role and the diff, and it returns the JSON of
   decision 5. In detail:
   - input: `.github/review/README.md` and the `REVIEW.md` of the role, both from the base commit,
     and the diff `base...head` of a checkout of the head;
   - output: `<role>.json`, validated against the schema, uploaded as an artifact;
   - the roles other than `refute` run as a matrix over the roles of the plan, with no access to
     each other's output; `refute` runs in a later job that downloads their artifacts;
   - the workflow runs on `pull_request_target`, since only that trigger loads the workflow file
     from the base commit; with `pull_request` the file comes from the merge commit and the author
     of the pull request edits the workflow that judges it. A job of the workflow runs only when
     `github.event.pull_request.head.repo.full_name == github.repository`. A job with no condition,
     `review-gate`, is the required check: it fails on a pull request from a fork, with the message
     that no role reviewed it and a person has to, because GitHub counts a skipped required check
     as passed; on any other pull request it passes only when every job of the roles passed. Because `pull_request_target` carries secrets, no step runs or
     installs code of the head: no build, no test, no hook, no `npm install`; the runner reads the
     head as files;
   - the permissions of the job and the tools of the runner are set when `review.yml` is written,
     under these limits: the token of the job reads the repository and writes nothing; the key of
     the model reaches only the step of the runner; the runner has a list of allowed tools with no
     shell and no network, since a role in CI reads the diff and the files and does not run code of
     the pull request, and reads only the workspace; the base commit is the workspace and the head
     commit is a subfolder that the runner reads. Before the runner starts, the job deletes from the head checkout, at every depth, the files
     that an agent loads as instructions or configuration (`CLAUDE.md`, `AGENTS.md`, `.claude/`,
     `.mcp.json`, `.cursorrules`) and every symbolic link, and the checkout step sets
     `persist-credentials: false`, so that a file of the pull request is not loaded as
     instructions and no link leads out of the workspace;
   - the text of a finding is data. In CI, `how_to_verify` is for a person: neither `refute` nor
     any other step runs it. `review-refute` runs those commands today (step 3 of its skill), on
     the machine of the author and against the diff of the author; its CI mode is a later change
     and leaves that step out;
   - the job does not trust the verdict that the model writes: it computes it from the findings
     and from `blocking` (`FAIL` when a finding reaches it) and replaces the model's value;
   - a runner that fails, times out or returns invalid JSON gives `INCONCLUSIVE`, and the job
     fails on `FAIL` and on `INCONCLUSIVE` alike, so a role that did not finish never passes the
     required check;
   - the job fails when a string of a finding holds the value of the key of the model, in plain
     text or in base64, since the findings go to an artifact.

   The first runner is `anthropics/claude-code-action` at a pinned commit. The text of the role
   goes in `prompt`, and `claude_args: --json-schema '<schema>'` makes the action return the
   findings as the `structured_output` output
   (<https://github.com/anthropics/claude-code-action/blob/main/docs/usage.md>). A later step reads
   that output through `env:`, since the action's documentation warns that an output written
   into a script with `${{ }}` can run as a command. The workflow `review.yml` is written in a
   later change.

10. `abide.yml` stays as it is. It judges the judgment rules of botica with the rubric that
    `botica rules export --format abide` writes. A role covers a focus and the rubric covers every
    judgment rule, and the two do not share a runner.

11. A later change makes the recipe `base` install `.github/review/README.md`, the generic `REVIEW.md` of each role and
    the `review-*` skills; `catalog/recipes/base.toml` has none of them yet. `install` writes a file that is missing and leaves an existing one as it
    is (ADR 0006), so a repository that adjusted a `REVIEW.md` keeps it. `review-tests` calls
    `test-smell-review` for the smells of a test.

## Considered options

- abide as the runner, with one rule per role that `botica review plan` writes into the rubric
  (`manual: true`, `source.path` the `REVIEW.md`, `scope` the paths of the role, `when: turn`).
  Rejected: a rule asks one typed question whose `instructions` are at most 2000 characters
  (`abide-schema/src/rubric.ts`), so the prose of a role has to be summarized; the verdict has no
  message or line, so botica would map a rule id back to a text to write a finding; and botica
  would depend on the `version: 1` schema of the rubric. The path stays open if the runner of
  decision 9 does not work out, and costs a new step in `review.yml`.
- `codex` as the first runner. Not chosen: the contract of decision 9 does not depend on the
  runner, so a change of runner replaces one step of `review.yml` and nothing else. The first
  runner stays open to the review of this ADR.
- One `REVIEW.md` at the root for all roles. Rejected: each role runs in its own session and would
  read the instructions of the others, and a folder per role holds examples and eval cases that a
  single file would grow to carry.
- Only the skill, with the repository text inside it. Rejected: the repository text would sit in
  `.agents/skills`, which is gitignored and rebuilt from `skills-lock.json`, and the owners of the
  repository would edit a file that botica owns.
- Only the `REVIEW.md`, with the procedure inside it. Rejected: the procedure of a local run
  (running tests, opening the application) has no place in CI, and a copy of the procedure in each
  repository drifts from the skill (ADR 0008).
- Read the roles from the head of the pull request. Rejected: see decision 7.
- The findings as markdown. Rejected: `botica lint --reviewed` and `review-refute` read them, and a
  fixed JSON is checked against a schema.

## Not decided

- The names of the signals of `applies.signals` and how botica detects each one.
- How `botica lint --reviewed` reads the findings. Their JSON Schema is
  `schema/review-findings.schema.json`, written in draft-07.
- The command that writes the `botica:covered` blocks, and the CODEOWNERS file with its entries.
- The model, the limits of time and cost, and the version of the runner of CI.
- The tier of a change and the areas that give it (`botica review tier`): this ADR only reads the
  tier.

## Consequences

- A role is two files that change at different times. A new release of botica changes a skill and
  leaves the `REVIEW.md` of the repository as it is; the generic `REVIEW.md` reaches a repository
  that has none, and an existing one needs a manual merge, since `install` leaves it.
- A change to `.github/review/` has no effect on its own pull request. A repository adopting the
  flow merges the first `REVIEW.md` before CI runs the roles.
- The generic `REVIEW.md` files in `skills/review/*/references/` follow decision 4 except for the
  `botica:covered` block, which none has yet, and `owners`, which is optional. Adding the block
  belongs to the change that adds `botica review check`.
- `review-panel` and the workflow `review.yml` run the same roles only while both read
  `botica review plan`. The local fallback to `references/REVIEW.md` in decision 7 is a case where
  a local run and a CI run can differ, for a repository with no file on its base.
- A key of the model is a new secret of the repository for the roles. `abide.yml` already holds
  `TYPESAFE_AI_API_KEY`; the runner of decision 9 needs its own.

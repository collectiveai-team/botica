# A change gets a risk tier from its paths, and a classifier can only raise it

**Status:** proposed

ADR 0017 defines the roles that review a diff and reads a tier from `botica review tier`, which it
leaves undecided. Nothing yet says how much review a change needs or when no person has to look at
it. Today every pull request goes to a person, whatever it touches, except the labeled pull
requests of Dependabot, which `.github/workflows/dependabot-automerge-run.yml` merges once the
checks pass: a typo in a page and a change to the release workflow wait for the same review.

Two questions are different. The first is how sensitive the files are, and a list of paths answers
it without a model. The second is whether this pull request, with all its checks green, still holds
a decision that someone has to take or a part that only a person can judge. A list of paths cannot
see that, and a model can estimate it. This ADR keeps them apart: the paths set a floor, a
classifier runs last and can only move a change up, and a person is the default when anything is
missing.

## Decision

1. Four areas with fixed names, declared once in `botica.toml`:

   | Area | Holds | Floor | People |
   |---|---|---|---|
   | `critical` | authentication, billing, migrations, permissions | `high` | the owner of the area |
   | `platform` | CI, Dockerfile, build, release, infrastructure as code | `medium` | the owner of platform |
   | `core` | the domain, `core/`, public contracts (`api/`) | `medium` | one person |
   | `periphery` | documentation, tests, patch updates of dependencies | `low` | none, with CI green |

   ```toml
   [[review.areas]]
   name = "critical"
   paths = ["src/**/auth/**", "db/migrations/**"]
   owners = ["@org/platform"]
   tier = "high"
   approvals = 2
   ```

   | Key | Value | Default |
   |---|---|---|
   | `name` | `critical`, `platform`, `core` or `periphery` | required |
   | `paths` | globs, as in `applies.paths` of ADR 0017 | required |
   | `owners` | the people or teams that answer for the paths | none; required for `critical` and `platform` |
   | `tier` | `low`, `medium` or `high` | the floor of the table |
   | `approvals` | the approvals that the paths ask for | 1; 0 for `periphery` |

   An area name may appear in several entries, each with its own paths and owners. `botica.toml`
   fails to load when a `tier` is lower than the floor of the table for that name, when
   `approvals` of `critical` is under 1, or when an entry of `critical` or `platform` has no
   `owners`. A file that no entry matches belongs to `core`, so a repository with no
   `[[review.areas]]` has no change at tier `low`. A rename counts the old path and the new one.
   The settings of a feature of `periphery` (a flag) are not detected: a path is `periphery` only
   when an entry says so.

   Some paths are `platform` whatever the entries say, since they decide how a pull request is
   reviewed, built and merged, or are loaded by an agent as instructions: `botica.toml`,
   `.github/review/**` and `.gitlab/review/**` (and the folder of `[review] dir`),
   `skills-lock.json`, `.github/workflows/**`, `.github/actions/**`, `.gitlab-ci.yml`,
   `.github/CODEOWNERS`, `.gitlab/CODEOWNERS`, `.gitattributes`, `prek.toml`, `.abide/**`,
   `AGENTS.md`, `CLAUDE.md`, `INVARIANTS.md` and `**/SKILL.md`. The list includes the files that
   ADR 0017 decision 7 reads from the base and the files that decision 9 strips from the head. An
   entry of `periphery` cannot cover them, and `botica.toml` fails to load if one does. Their
   owners and approvals come from the entries of `platform`, and a `botica.toml` that has
   `[[review.areas]]` and no `platform` entry fails to load.

2. The tier of a change is the highest floor among its files and its signals. A file that several
   entries match takes the highest tier among them. The change touches each area that one of its
   files matches, and the output lists all of them. The diff sets the tier and the author does not:
   the job reads `botica.toml` from the base branch, as ADR 0017 decision 7 says for the rest of
   `[review]`.

3. A signal is a name for a fact that botica reads from the diff with no model. This ADR fixes two,
   the ones that ADR 0017 uses as examples. `new-dependency` holds when a manifest or a lockfile
   lists a package that the base did not list (a transitive one included), changes the source,
   the registry or the checksum of a package, loosens a version constraint, or changes a version
   by more than a patch. Only a patch version of a package already listed, from the same source
   and with the same constraint, is not one. The signal holds as well when botica has no detector
   for the file or cannot parse it, so a missing detector raises the floor and never lowers it.
   `new-endpoint` holds when the diff adds a route to a service that the repository exposes. Each
   signal raises the floor to `medium`, selects the roles whose `applies.signals` name it (ADR
   0017), and goes into the state of the classifier. How botica detects each one is a later change.

4. `botica review tier --diff <base>` prints the tier, the floor of each area and each signal, the
   areas touched, and the rule of `[[review.areas]]` and the files behind each, as text or JSON. A
   workflow reads the JSON and sets the labels `risk:<tier>` and `area:<area>`, one `area:` label
   per area touched. `botica review export --format codeowners` writes `.github/CODEOWNERS` or
   `.gitlab/CODEOWNERS` from the same entries, with the sections `[critical][2]` and `^[periphery]`
   on GitLab. CODEOWNERS takes the last matching line, so the export writes the lines from the
   lowest tier to the highest, and a path that several entries match ends on the line of the
   highest tier. GitHub has no approval count in CODEOWNERS and a branch protection holds one
   count for the whole branch, so a required check reads the reviews, counts the approvals of the
   owners of each area touched and fails below the `approvals` of that area. Both commands and
   that check come in later changes.

5. A classifier decides whether a pull request can go on without a person. It runs after the
   checks, with their results, and asks one thing; it repeats neither the roles nor the CI:

   ```
   review tier ─► CI + lint ─► roles ─► gates (code) ─┬─► a gate fails ─────────┐
                                                      ▼                         ▼
                                                 Jev (3 questions) ─► person ─► handoff report
                                                      │
                                                      ▼
                                                 automatic merge
   ```

6. The gates are code, and Jev does not run when one of them fails. The pull request goes to a
   person when:
   - the floor is not `low`, or the change touches `critical` or `platform`;
   - the pull request comes from a fork, since ADR 0017 decision 9 runs no role on one;
   - the CI or `botica lint` is not green: a check that failed, is pending, was skipped or is
     missing counts as not green, and so does an error of lint. The checks that must be present
     are the names in `[review.classifier] required-checks` of the base `botica.toml`, and an
     empty list sends every pull request to a person, since a repository with no branch
     protection has no other list;
   - a role has no valid JSON file, or ended in `FAIL` or `INCONCLUSIVE` (ADR 0017 decision 5), or
     a finding of severity `P0` to `P2` is left that `refute` did not discard, which means
     `confirmed` or no `refute.json`. The roles checked are `spec` and `security`, whatever
     `roles` and `applies` say, and each role that the plan of ADR 0017 selects;
   - the change passes `max-changed-lines`, 400 by default, or holds a file that the diff cannot
     show as text: a binary, a symbolic link, a submodule, or a file that a `.gitattributes` marks
     `-diff` (the size is counted with the attributes of the base, and a file that still prints no
     lines counts as over the limit). Generated files are not counted when the base lists them in
     `[review] generated`, as globs of `botica.toml`; a header or a `.gitattributes` of the head
     does not decide it.

7. The questions go to Jev in one request to `POST /v1/systemone`, about the whole pull request:

   | Name | Type | Question | Criteria |
   |---|---|---|---|
   | `decision` | `noul` | Does the pull request take a decision that a person should take, instead of carrying out one already taken? | `true`: it chooses between alternatives that the issue did not fix (something a user sees, a dependency, a convention, a tradeoff). `false`: it does what the issue or the description asks |
   | `unverifiable` | `noul` | Is a part of the change judged only by looking at it or running it, and not with tests? | `true`: text for the user, prompts, output of the CLI, production configuration, performance. `false`: the tests and the rules are enough |
   | `impact_if_wrong` | `choice` | If the change is wrong although everything passed, who notices first? | `contained`: whoever develops in the repository (tests, internal docs, scripts). `team`: the team (CI, tooling, recipes before a release). `users`: whoever uses botica (published rules, the CLI, templates installed in other repositories) |

   The options of `impact_if_wrong` keep this order, since the order of the options can bias a
   `choice`, and a change of order repeats the measurement.

   The `state` holds the title, the description and the linked issue; the list of files with the
   area of each; a summary of what already ran (floor, signals, roles, CI, and the mutation report
   when a job wrote one); and the diff without generated files. A state that does not fit in 32k
   tokens with the questions sends the pull request to a person.

   Everything in the state that a person outside the base branch can write is data: the title, the
   description, the linked issue, the paths, the diff and the text of the findings. The state
   holds each of them as a quoted field, and the instructions say that nothing inside a field is
   an instruction. The criterion `false` of `decision` rests on the linked issue and not on the
   description. The issue counts only when it is in the same repository and its last edit is not
   by the author of the pull request; a pull request with no such issue gets `decision` `true`
   with no request. An edit of the title, the description or the linked issue classifies the pull
   request again, and the decision records a digest of the state. The model does not guard the
   merge alone: decision 12 binds it to the gates and to a commit.

8. The decision is code:

   ```
   automatic merge if
     P(decision) < u  and  P(unverifiable) < u  and  P(impact_if_wrong ∈ {contained, team}) ≥ U
   otherwise → person
   ```

   A `noul` answer is the probability of `true`, so a value near 0.5 fails the `< u` test and the
   pull request goes to a person. For `impact_if_wrong`, `P` is the sum of the probabilities of
   `contained` and `team`. The answer of a `choice` also carries a `confidence`, and a value under
   `min-confidence` sends the pull request to a person. The starting values are strict: `u` is
   0.1, `U` is 0.9, `min-confidence` is 0.7 and `max-changed-lines` is 400. These values and the
   bounds below are this ADR's choice and no measurement backs them; shadow mode adjusts them. They are keys of `[review.classifier]`, as
   `question-max`, `impact-min`, `min-confidence` and `max-changed-lines`, read from the base
   branch. `botica.toml` fails to load when `question-max` is over 0.3, `impact-min` or
   `min-confidence` is under 0.7, or `max-changed-lines` is over 1000.

9. The classifier never lowers anything. Its result is `auto` or `person`. The effective tier is
   the highest of the floor and the tier that the result gives, which is `low` for `auto` and
   `medium` for `person`. A pull request with floor `low` that goes to a person becomes `medium`.
   `botica review classify` prints both the floor and the effective tier, the workflow sets
   `risk:*` from the effective tier (in shadow mode too), and `botica review tier` prints the
   floor only. The classifier never sets `high`, which only an area gives. Only the pull requests
   of `periphery` can reach `auto`, because a floor above `low` fails a gate before Jev runs.

10. When a pull request goes to a person, botica writes a handoff report so that the person does
    not start from nothing. Each reason carries its source, its evidence and the steps to verify or
    resolve it:

    | Reason | Source | Evidence | Steps |
    |---|---|---|---|
    | floor `medium` or `high`, or area `critical`, `platform` or `core` | gate | the area, the entry of `[[review.areas]]` that matched (or that no entry matched and the file fell to `core`), the signals, the files | who owns it (CODEOWNERS); the `INVARIANTS.md` of the area, if there is one |
    | fork, a role with no valid file, or an empty `required-checks` | gate | the repository of the head, the role and the missing file, or the empty list | a person reviews the pull request; for a role, run `botica review check` |
    | CI or lint not green | gate | the job, its first failing line, the link to the log | the command that reproduces it locally (`botica lint --diff <base>`, `cargo test -p <crate>`) |
    | role with `FAIL` or `INCONCLUSIVE`, or a `P0` to `P2` finding that `refute` did not discard | gate | the findings with `path:line`, severity, and what `refute` could not refute | the steps of each finding; for `INCONCLUSIVE`, what the role lacked |
    | pull request too large, or a file with no text diff | gate | the size, the files that show no lines, and how many intents the title and the commits name | split it (BTPROC001), or a stack with the skill `pr` |
    | state over the limit, no linked issue of the repository, or an invalid answer of Jev | classifier | the size of the state, the issue that is missing, or the answer that failed validation | none; the person reviews as if there were no classifier |
    | `decision` | Jev | Jev gives no explanation, so the same question runs for each file, in parallel, and the files with a high probability are listed | review that decision against the issue; if the issue fixes it, add it to the description |
    | `unverifiable` | Jev | the same per-file question, plus the texts, prompts or flags that changed in those files | run what changed on the base and on the head and compare, for example the output of `botica <command>` |
    | `impact_if_wrong` has less than `U` on `contained` and `team` | Jev | the probabilities of the three options and, when `users` leads, the published surface that the change touches: a rule, a command, a template of a recipe | try the recipe in a test repository; note it in the changelog |
    | Jev fails, times out or has low `confidence` | classifier | the error, the model that answered and the token `usage` (the documented response has no request id) | none; the person reviews as if there were no classifier |

    The per-file questions run only on a referral, for the reason that Jev raised, and cost one
    request per file. Code fills a template for each reason; a model does not write the report. A
    summary in prose is optional and follows the evidence. The report has two outputs:
    `.botica/review/handoff.json`, which the workflow uploads as an artifact and which enters the
    history, and a comment on the pull request that each push edits in place. It ends with what
    already ran, what was not reviewed (roles that did not run, files outside the state) and the
    decision that it expects: approve, ask for changes, or raise the area to `critical`.

11. The limits of the classifier:
    - A failed call, a timeout, an invalid answer or a low `confidence` sends the pull request to a
      person.
    - The adapter of Jev is behind a port `Classifier`, and the tests use an in-memory adapter
      (BTTEST001). There is no second model.
    - The `model` is pinned (`jev-1.14.0`) and never `jev-latest`. A new version is a pull request
      that repeats the measurement against the labeled set. `GET /v1/models` lists the names that
      the account can send, and the first run checks that the pinned name is among them.
    - The account is the TypeSafe account of abide, with its own key for the classifier:
      `TYPESAFE_AI_BASE_URL` is shared, and the key is a new secret, since the key of `abide.yml`
      reaches a job that builds code of the head. The classifier follows the limits of ADR 0017
      decisions 7 and 9: it reads `botica.toml` and the workflow from the base, runs no code of the
      head, and runs only for a pull request of the same repository. Two jobs share the work. The
      job that classifies reads the repository and the artifacts of the roles, holds the key and
      has no write permission; it uploads the decision and the report as an artifact. The other
      job has no key and no checkout of the head; it reads that artifact, sets the labels, edits
      the comment and, in `enforce` mode, merges. The text of the report that comes from the pull
      request (log lines, titles of findings, the `how_to_verify` of a finding) is data: the
      comment shows it in a code block whose fence is longer than any run of backticks in the
      text, and no step runs it.
    - A missing or invalid decision artifact, and a missing `mode`, give `person` and `shadow`.
    - The roles and `refute` are models too, and the text of the diff can move them. The classifier
      reads their verdicts as one more input and does not replace the gates that need a person:
      `critical`, `platform` and every floor above `low` stay with a person whatever the roles say.

12. Four rules hold whatever the model says:
    - `critical` and `platform` never go to automatic merge.
    - A new commit classifies the pull request again and rewrites the report.
    - A bot can approve, and its approval does not take the place of a person.
    - The merge is the work of the workflow and not of botica, and it is bound to one commit: it
      reads the decision of the artifact, checks that the head of the pull request is the commit
      and the base branch is the commit that the decision names, and merges with
      `--match-head-commit`. A push after the decision leaves the pull request unmerged until the
      next run decides again. The classification is a required check of the base branch, and no
      job reads the labels `risk:*` and `area:*` as input. `gh pr merge --auto` needs a branch
      protection that names required checks; where there is none it merges at once (see the header
      of `dependabot-automerge-run.yml`), so `enforce` stays off until the protection exists, or
      the workflow waits as that file does.
    - `dependabot-automerge-run.yml` keeps merging on the label `automerge`, with no floor and no
      gate. This ADR leaves it as it is; a later change either routes it through the gates of
      decision 6 or limits the label to people with write access.

13. The classifier starts in shadow mode, `[review.classifier] mode = "shadow"`. It decides, writes
    the report and records the decision, and never merges. Each record keeps the probabilities, the
    result, whether a merge would have happened, the answer of the person (approved, asked for
    changes, changed the area) and whether a revert or an incident followed. The history of
    findings (planned as ADR 0019) holds the records. The measure is that the time of people per
    pull request goes down and the reverts of pull requests that merged automatically do not go up.
    A set of labeled pull requests of botica measures each change of a question or a threshold.
    The mode `enforce` and the workflow that merges are off, and turning them on is a decision of a
    person, together with the branch protection.

14. `botica review classify --base <rev>` reads the results of the earlier stages, prints JSON and,
    when it sends the pull request to a person, writes the report. It comes in a later change, as
    the other commands do.

15. The review rules that depend on risk (the diff sets the tier and the author does not, a bot does
    not take the place of a person, a new commit asks for approval again, and others) go to the text
    of a standard and to the `REVIEW.md` files. They are not code.

## Considered options

- The classifier alone sets the tier, with no floor from paths. Rejected: an answer of a model
  can be wrong in either direction, and a path list cannot be argued with. With a floor, the worst
  failure of the model is a pull request that goes to a person.
- One score question, "how risky is this change", in place of three. Rejected: a score does not say
  why, and each of the three questions maps to its own evidence and its own steps in the report.
- A second model that votes with Jev. Rejected: it needs its own calibration and a rule for a
  disagreement, and the measurement of decision 13 would compare two systems.
- The classifier before the roles. Rejected: it needs their verdicts in its state, and a gate that
  fails would spend a request that the pull request does not need.
- Automatic merge for `medium`. Rejected: `medium` means one person, and `core` holds the public
  contracts. The first version opens `low` only, and a later ADR can widen it with the history
  behind it.
- Starting in `enforce` with high thresholds. Rejected: no threshold has data behind it yet.
- Labels set by `botica review tier`. Not chosen: setting a label needs a token that the command
  does not hold, and a workflow that reads the JSON sets them.
- A signal that only selects roles and never moves the floor. Possible; it costs a change to
  decision 3 and nothing else.

## Not decided

- The areas of botica: its paths, owners and approvals.
- The detection of `new-endpoint` and `new-dependency` per language, and more signals.
- The final values of `question-max`, `impact-min`, `min-confidence` and `max-changed-lines`, which
  shadow mode sets within the bounds of decision 8.
- The globs of `[review] generated` that botica proposes for its own repository.
- The text of the templates of the handoff report.
- The timeout and the retries of the call to Jev.
- The set of labeled pull requests and how each is labeled.
- The workflows that set the labels, run the classifier and merge, and the branch protection.
- The standard of decision 15 and the CODEOWNERS file with its entries.

## Consequences

- In shadow mode nothing merges by itself, and a person sees the same pull requests as today plus
  a report on those that the classifier would have referred.
- A repository with no `[[review.areas]]` gets `core` for every file, so none of its changes can
  reach `auto`. It has to declare a `periphery` area before the classifier can pass anything.
- Only `periphery` reaches `auto`. The gain is limited to documentation, tests and patch updates
  of dependencies.
- The title, the description and the diff of a pull request go to the account of TypeSafe that
  abide already uses. A repository that cannot send diffs there does not turn the classifier on.
- The description of a pull request is input of the model, and its author can write text that
  moves a probability. Decision 7 quotes that text and ties `false` to the linked issue. The damage
  is bounded: the change has to be `periphery`, with CI and the roles green. An answer of Jev can
  only add a reason to refer, never remove a gate; the roles can be moved by the same text, and
  decision 11 says what stays with a person when they are.
- Decision 1 fixes the paths that decide how a pull request is reviewed, built and merged, and
  the files that an agent loads as instructions (`botica.toml`, `.github/review/**`, the
  workflows, CODEOWNERS, `AGENTS.md`, `SKILL.md` and others) as `platform`, so a documentation
  entry cannot cover them. A repository cannot put those files in `periphery`.
- Gate 3 needs `required-checks` in `botica.toml`. A repository that does not list them sends
  every pull request to a person.
- A required check that counts the approvals per area is a new job. Until it exists, `approvals`
  above 1 is documentation and the branch protection holds one count.
- A change to the wording or the order of a question, or to the pinned model, repeats the
  measurement. The questions are part of the contract of this ADR for that reason.
- Each classified pull request costs one request for the three questions, and one more per file
  for each Jev reason on a referral. The response reports `usage`.
- The effective tier of a referred pull request is `medium`, so a label that a person sees can be
  higher than the floor that `botica review tier` printed.

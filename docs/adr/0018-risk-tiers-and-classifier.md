# A change gets a risk tier from its paths, and a classifier can only raise it

**Status:** proposed

ADR 0017 defines the roles that review a diff and reads a tier from `botica review tier`, which it
leaves undecided. Nothing yet says how much review a change needs or when no person has to look at
it. Today every pull request of the repository goes to a person, whatever it touches: a typo in a
page and a change to the release workflow wait for the same review.

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
   | `periphery` | documentation, tests, internal features behind a flag, patch updates of dependencies | `low` | none, with CI green |

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
   | `owners` | the people or teams that answer for the paths | none |
   | `tier` | `low`, `medium` or `high` | the floor of the table |
   | `approvals` | the approvals that the paths ask for | 1; 0 for `periphery` |

   An area name may appear in several entries, each with its own paths and owners. `botica.toml`
   fails to load when a `tier` is lower than the floor of the table for that name. A file that no
   entry matches belongs to `core`, so a repository with no `[[review.areas]]` has no change at
   tier `low`. A rename counts the old path and the new one.

2. The tier of a change is the highest floor among its files and its signals. A file that several
   entries match takes the highest tier among them. The change touches each area that one of its
   files matches, and the output lists all of them. The diff sets the tier and the author does not:
   the job reads `botica.toml` from the base branch, as ADR 0017 decision 7 says for the rest of
   `[review]`.

3. A signal is a name for a fact that botica reads from the diff with no model. This ADR fixes two,
   the ones that ADR 0017 uses as examples. `new-dependency` holds when a manifest lists a package
   that the base did not list; a new version of a package already listed is not one.
   `new-endpoint` holds when the diff adds a route to a service that the repository exposes. Each
   signal raises the floor to `medium`, selects the roles whose `applies.signals` name it (ADR
   0017), and goes into the state of the classifier. How botica detects each one is a later change.

4. `botica review tier --diff <base>` prints the tier, the floor of each area and each signal, the
   areas touched, and the rule of `[[review.areas]]` and the files behind each, as text or JSON. A
   workflow reads the JSON and sets the labels `risk:<tier>` and `area:<area>`, one `area:` label
   per area touched. `botica review export --format codeowners` writes `.github/CODEOWNERS` or
   `.gitlab/CODEOWNERS` from the same entries, with the sections `[critical][2]` and `^[periphery]`
   on GitLab. GitHub has no approval count in CODEOWNERS, so `approvals` reaches GitHub through the
   branch protection that a person sets. Both commands come in later changes.

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
   - the CI or `botica lint` is red, where lint is red when it reports an error;
   - a role ended in `FAIL` or `INCONCLUSIVE` (ADR 0017 decision 5), or a finding of severity `P0`
     to `P2` is left that `refute` did not discard, which means `confirmed` or no `refute.json`;
   - the change passes `max-changed-lines`, 400 by default, counted without generated files.

7. The questions go to Jev in one request to `POST /v1/systemone`, about the whole pull request:

   | Name | Type | Question | Criteria |
   |---|---|---|---|
   | `decision` | `noul` | Does the pull request take a decision that a person should take, instead of carrying out one already taken? | `true`: it chooses between alternatives that the issue did not fix (something a user sees, a dependency, a convention, a tradeoff). `false`: it does what the issue or the description asks |
   | `unverifiable` | `noul` | Is a part of the change judged only by looking at it or running it, and not with tests? | `true`: text for the user, prompts, output of the CLI, production configuration, performance. `false`: the tests and the rules are enough |
   | `impact_if_wrong` | `choice` | If the change is wrong although everything passed, who notices first? | `contained`: whoever develops in the repository (tests, internal docs, scripts). `team`: the team (CI, tooling, recipes before a release). `users`: whoever uses botica (published rules, the CLI, templates installed in other repositories) |

   The options of `impact_if_wrong` keep this order, since the order of the options can bias a
   `choice`, and a change of order repeats the measurement.

   The `state` holds the title, the description and the linked issue; the list of files with the
   area of each; a summary of what already ran (floor, signals, roles, CI, mutation); and the diff
   without generated files. A state over 32k tokens sends the pull request to a person.

8. The decision is code:

   ```
   automatic merge if
     P(decision) < u  and  P(unverifiable) < u  and  P(impact_if_wrong ∈ {contained, team}) ≥ U
   otherwise → person
   ```

   A `noul` answer is the probability of `true`, so a value near 0.5 fails the `< u` test and the
   pull request goes to a person. For `impact_if_wrong`, `P` is the sum of the probabilities of
   `contained` and `team`. The answer of a `choice` also carries a `confidence`, and a value under
   `min-confidence` sends the pull request to a person. The three values start strict: `u` is 0.1,
   `U` is 0.9 and `min-confidence` is 0.7. Shadow mode adjusts them. They are keys of
   `[review.classifier]`, as `question-max`, `impact-min` and `min-confidence`.

9. The classifier never lowers anything. Its result is `auto` or `person`. The effective tier is
   the highest of the floor and the tier that the result gives, which is `low` for `auto` and
   `medium` for `person`. A pull request with floor `low` that goes to a person becomes `medium`,
   and the label `risk:*` shows the effective tier, while the JSON keeps the floor. The classifier
   never sets `high`, which only an area gives. Only the pull requests of `periphery` can reach
   `auto`, because a floor above `low` fails a gate before Jev runs.

10. When a pull request goes to a person, botica writes a handoff report so that the person does
    not start from nothing. Each reason carries its source, its evidence and the steps to verify or
    resolve it:

    | Reason | Source | Evidence | Steps |
    |---|---|---|---|
    | floor `high` or area `critical` or `platform` | gate | the area, the entry of `[[review.areas]]` that matched, the files | who owns it (CODEOWNERS); the `INVARIANTS.md` of the area, if there is one |
    | CI or lint red | gate | the job, its first failing line, the link to the log | the command that reproduces it locally (`botica lint --diff <base>`, `cargo test -p <crate>`) |
    | role with `FAIL` or `INCONCLUSIVE` | gate | the findings of the role with `path:line`, severity, and what `refute` could not refute | the steps of each finding; for `INCONCLUSIVE`, what the role lacked |
    | pull request too large | gate | the size and how many attempts it seems to hold | split it (BTPROC001), or a stack with the skill `pr` |
    | `decision` | Jev | Jev gives no explanation, so the same question runs for each file, in parallel, and the files with a high probability are listed | review that decision against the issue; if the issue fixes it, add it to the description |
    | `unverifiable` | Jev | the same per-file question, plus the texts, prompts or flags that changed in those files | run what changed on the base and on the head and compare, for example the output of `botica <command>` |
    | `impact_if_wrong` is `users` | Jev | the published surface that the change touches: a rule, a command, a template of a recipe | try the recipe in a test repository; note it in the changelog |
    | Jev fails, times out or has low `confidence` | classifier | the error and the `request_id` | none; the person reviews as if there were no classifier |

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
      the account can use.
    - The credentials are those of abide: `TYPESAFE_AI_API_KEY` and `TYPESAFE_AI_BASE_URL`. The
      job that classifies follows the limits of ADR 0017 decisions 7 and 9: it reads the
      configuration from the base and runs no code of the head.

12. Four rules hold whatever the model says:
    - `critical` and `platform` never go to automatic merge.
    - A new commit classifies the pull request again and rewrites the report.
    - A bot can approve, and its approval does not take the place of a person.
    - The merge is the work of the workflow (`gh pr merge --auto`) and not of botica.

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
  shadow mode sets.
- How generated files are recognized in the state and in the size count.
- The text of the templates of the handoff report.
- The set of labeled pull requests and how each is labeled.
- The workflows that set the labels, run the classifier and merge, and the branch protection.
- The standard of decision 15 and the CODEOWNERS file with its entries.

## Consequences

- In shadow mode nothing merges by itself, and a person sees the same pull requests as today plus
  a report on those that the classifier would have referred.
- A repository with no `[[review.areas]]` gets `core` for every file, so none of its changes can
  reach `auto`. It has to declare a `periphery` area before the classifier can pass anything.
- Only `periphery` reaches `auto`. The gain is limited to documentation, tests, patch updates and
  internal features behind a flag.
- The title, the description and the diff of a pull request go to the account of TypeSafe that
  abide already uses. A repository that cannot send diffs there does not turn the classifier on.
- The description of a pull request is input of the model, and its author can write text that
  moves a probability. The damage is bounded: the change has to be `periphery`, with CI and the
  roles green, and an answer of the model can only add a reason to refer, never remove a gate.
- A change to the wording or the order of a question, or to the pinned model, repeats the
  measurement. The questions are part of the contract of this ADR for that reason.
- Each classified pull request costs one request for the three questions, and one more per file
  for each Jev reason on a referral. The response reports `usage`.
- The effective tier of a referred pull request is `medium`, so a label that a person sees can be
  higher than the floor that `botica review tier` printed.

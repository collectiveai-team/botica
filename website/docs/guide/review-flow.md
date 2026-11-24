# Review a change from pinned evidence

`botica review tier --diff <destination> --integration-method merge --format json`
assesses branch risk from every best common ancestor and integration risk from the
merge result. It reads policy from the destination. Text uses explicit conservative
policy bounds when the supported v1 dependency and endpoint detectors cannot establish
their facts. Non-text inputs and unsupported detector versions remain incomplete.

`botica review plan` takes the same flags and selects roles. `--stage ci` selects
the CI roles; the default is local. `botica review check` also checks the derived
skills tree and declared check references without executing them.

## Code hosts

`review tier`, `plan`, `check`, `validate` and `export` read Git and local files, and `run` adds
the OpenAI API, so they run on any host. Roles are read from `.github/review/` or
`.gitlab/review/`, and `review export --host gitlab` writes `.gitlab/CODEOWNERS`. The options that
read a pull request (`review context`, and `--github-pr` of `classify`, `feedback` and
`authorize`) call the GitHub API through `gh`, and the review workflows of this repository are
GitHub Actions workflows. On GitLab, `classify` takes a local `--checks` attestation, which leaves
`producer_verified` false, and `authorize` without `--github-pr` cannot verify branch protection,
so automation stays blocked. The `base` recipe installs the roles under `.github/review/`.

## Execute and validate

```sh
botica review run --diff <destination> --integration-method merge \
  --context .tmp/change.json --output .tmp/review/<new-run>
botica lint --reviewed .tmp/review/<new-run> --diff <destination> \
  --review-context .tmp/change.json --record
```

Local runs default to OpenAI and read `OPENAI_API_KEY`. Pass `--provider nvidia`
to use NVIDIA NIM and `NVIDIA_API_KEY`. The NVIDIA default is
`nvidia/nemotron-3.5-super-vl-120b-a12b`; `--model` accepts the ID from the
[NVIDIA API catalog](https://build.nvidia.com/models), without the OpenCode
provider prefix. Botica's CI selects NVIDIA NIM and reads the `NVIDIA_API_KEY`
Actions secret. A missing credential blocks execution.

The runner gives the model no tools. Each role gets a separate request; refute
runs last. CI runs one selected role per matrix job. No head code is built,
installed or executed by the jobs that receive the credential.

The model returns only the verdict, findings, `checked` and `not_checked`. The
runner adds the role, the full base and head, the exact `inputs` object of the
current plan and an empty `commands` list, replacing any value the model
returned for those fields. A failed request reports its HTTP status and class
(credential, rate limit or quota, request, provider or transport) and never the
response body. A rate limit, provider error, timeout or cut-off response is sent
again, up to three attempts in total; a rejected credential or request is not. Missing roles, changed policy and stale heads fail validation.
Changing the issue context also invalidates earlier role evidence.
Verdicts are recomputed from evidence. Refute may add a disposition but may not
rewrite the original finding. Completed role results survive a later failure.
Only validated duplicates can be omitted from the gate. Source-quote observations
such as `pre-existing` or `unreachable` do not prove a dismissal by themselves.

## Classify in shadow mode

```sh
botica review classify --diff <destination> --integration-method merge \
  --results .tmp/review/<new-run> --repository <owner/repository> --github-pr <number> \
  --context .tmp/change.json
```

The context file holds a title, description and linked issue bodies or explicit
acceptance criteria as JSON data. `review context --repository <owner/repository>
--pr <number> --base <sha> --head <sha>` fetches closing issue bodies from GitHub.
A link without its issue body cannot pass spec review.

Classification verifies local role results against the latest successful associated
GitHub role artifact. Offline results remain unverified for automation.
It reads current GitHub Actions conclusions, verifies their integration
tree against the assessed tree, and checks base/head again after collection.
Every configured required check must pass. Any other failing or pending check or
commit status also blocks. The authorization check itself is excluded to avoid a
self-dependency. `--checks <file>` can describe offline
evidence, but is always unverified and cannot produce automation eligibility.
Before emitting a candidate, the command rereads roles, checks and acceptance
context. A changed snapshot blocks that recommendation. This is a recorded snapshot,
not a lock on future GitHub changes; automation remains disabled.

Jev uses `TYPESAFE_AI_API_KEY`, optional `TYPESAFE_AI_BASE_URL`, pinned model
`jev-1.14.0` and a 90-second request timeout. Its state contains compact decision
evidence and the diff without generated files; full provenance stays in the local gates.
The adapter accepts at most 32,000 reported input tokens, including the questions.
This is checked after the request, so a rejected request may incur service cost.
A separate 256 KiB request cap bounds local memory; it is not a token count.
Responses are bounded to 2 MiB. The destination's thresholds apply. They
remain uncalibrated until measured against confirmed labels.
An observed `decision`, `unverifiable` or user-impact reason raises effective risk
one tier, capped at high. The deterministic tier remains recorded separately.
Low confidence and service failures block automation without inventing a risk signal.
Only critical paths require human review, even when the classifier raises risk.

The command writes `.botica/review/handoff.json` and `handoff.md`, plus a history
record. Each reason has evidence and verification/resolution steps. Technical
failures block automation without requiring human review. Only critical changes
require a human. Labels are informational; network failures can leave remote labels
stale. Classification and automation must validate the evidence, never labels.
The Markdown handoff includes localized files, their probabilities and unavailable
localization evidence. It is the content published in the PR comment.

Shadow mode never merges. No command in this page authorizes enabling automation.

`review feedback --handoff <handoff.json> --repository <owner/repository>
--github-pr <number>` records GitHub human reviews of that handoff's commit, even
after the PR advances or merges. `--head <sha>` instead of `--handoff` records only
review observations, without claiming a classification exists. Neither path grants
authorization. Bots and reviews of other commits are excluded.

The passive `review-observation.yml` workflow signals submitted or dismissed
reviews without running PR code. `review-feedback.yml` then uses default-branch
code and queries GitHub reviews directly, without accepting artifacts from that
event. Its history artifact retains observations separately from classifications.

## Evaluate and keep automation disabled

`review evaluate --dataset bench/review-classifier/cases.jsonl` measures recorded
observations. The tracked seed includes actual reverted and maintenance PRs, with
missing labels and observations explicit. The README of that set states the proposed
sample size and statistical limits. It currently cannot establish calibration.

`review authorize` recomputes current evidence and reads the evaluation set from the
destination. It requires explicit `enforce` policy, passing measured evidence, and
strict up-to-date branch protection requiring the GitHub Actions-owned
`botica-authorization` check. It changes no settings and performs no merge.

`review-automation.yml` has a dispatch parameter defaulting to false and a separate
hard-disabled policy. Even a true dispatch input cannot enable it. Its future
authorized path publishes a head-bound authorization check before requesting
auto-merge, so a later head needs fresh authorization. Activation is not part of
this implementation.

# Review a change from pinned evidence

`botica review tier --diff <destination> --integration-method merge --format json`
assesses branch risk from every best common ancestor and integration risk from the
merge result. It reads policy from the destination. Rust source uses explicit
conservative bounds when the general dependency and endpoint detectors cannot
establish their facts. Other unsupported inputs remain incomplete.

`botica review plan` takes the same flags and selects roles. `--stage ci` selects
the CI roles; the default is local. `botica review check` also checks the derived
skills tree and declared check references without executing them.

## Execute and validate

```sh
botica review run --diff <destination> --integration-method merge \
  --context .tmp/change.json --output .tmp/review/<new-run>
botica lint --reviewed .tmp/review/<new-run> --diff <destination> \
  --review-context .tmp/change.json --record
```

The runner uses `OPENAI_API_KEY` with OpenAI's chat-completions API. It gives the
model no tools. A new request reviews each role; refute runs last. In CI the same
runner runs one selected role per matrix job. No head code is built, installed or
executed by the credential-bearing review jobs. A missing credential blocks the
job. No credential is created by these commands.

Each result must carry the full base, head and exact `inputs` object from the
current plan. Missing roles, changed policy and stale heads fail validation.
Changing the issue context also invalidates earlier role evidence.
Verdicts are recomputed from evidence. Refute may add a disposition but may not
rewrite the original finding. Completed role results survive a later failure.

## Classify in shadow mode

```sh
botica review classify --diff <destination> --integration-method merge \
  --results .tmp/review/<new-run> --checks .tmp/checks.json \
  --context .tmp/change.json
```

The context file holds the title, description and linked issue as JSON data.
Checks have the exact plan `inputs`, `ci`, `lint` and `mutation` statuses, and a
`jobs` object with a `PASS` value for every destination-configured required check.
These are attestations from their producer, not proof created by writing the file.
The CI producer must establish their provenance and bind them to the checked head.

Jev uses `TYPESAFE_AI_API_KEY`, optional `TYPESAFE_AI_BASE_URL`, pinned model
`jev-1.14.0`, a 90-second request timeout and a 32,000-byte conservative context
bound. Responses are bounded to 2 MiB. The destination's thresholds apply. They
remain uncalibrated until measured against confirmed labels.

The command writes `.botica/review/handoff.json` and `handoff.md`, plus a history
record. Each reason has evidence and verification/resolution steps. Technical
failures block automation without requiring human review. Only critical changes
require a human. Labels are informational; network failures can leave remote labels
stale. Classification and automation must validate the evidence, never labels.

Shadow mode never merges. No command in this page authorizes enabling automation.

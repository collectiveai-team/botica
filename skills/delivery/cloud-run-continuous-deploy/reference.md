# Reference: the twelve invariants

Each one guards a specific failure. The templates encode them; a template edit
that drops one is a bug, and `tests/test_cloud_run_deploy_skill.py` in the
scaffolding repo fails when it happens.

## 1. An untrusted ref never receives a credential

The review-tag path is three stages: an unprivileged listener, a credential-free
`validate` job, and a privileged `deploy` job that depends on it. Nothing that
runs before validation can reach WIF, an Environment, or a secret.

**Prevents:** a tag pointing at arbitrary code that builds and runs with deploy
credentials in scope.

**Check:** the block above `deploy:` in `deploy-integration.yml` contains no
`environment:`, no `google-github-actions/auth`, and no `${{ secrets.` reference.

**This invariant is false without a deployment branch policy on the environment.**
`deploy-integration.yml` binds `environment: integration` unconditionally, because
it is only ever meant to be reached through `validate`. But a `workflow_dispatch`
run executes the workflow file *from the dispatched ref*. Anyone with push access
can push a branch carrying a modified copy — `needs: validate` deleted, a step
added that exfiltrates `${{ toJSON(secrets) }}` — and dispatch it from the
attacker-authored listener, which holds `actions: write`. The run binds
`environment: integration` and receives every secret in it.

The WIF attribute condition (invariant 4) denies the *GCP* token for such a ref,
so no Google credential is issued — but it does nothing about GitHub Environment
secrets, and the integration `DATABASE_URL` alone is enough. Phase 2's branch
policy is what closes this. Nothing else in the design does, and no test in this
repo can detect its absence, because it is server-side state rather than file
content. Verify it by hand:

```bash
gh api "repos/$REPO/environments/integration/deployment-branch-policies" \
  --jq '.branch_policies[].name'
```

**A narrower warning belongs here too: `docker build` is deliberately the one
exception.** The `deploy` job is credentialed — it holds WIF and an Environment's
secrets — and it also builds the PR author's own tree, because that is the entire
point of a review deployment. `docker build` is safe as written because it is
sandboxed from the runner: no host bind mount lets the build reach the job's
filesystem or its token, and no client-supplied BuildKit secret hands the PR's
Dockerfile anything to steal. The PR's code runs, but only inside the image it
produces, after the credentialed job has finished with it.

That containment is specific to `docker build`. **Do not add any other step that
runs the PR tree's code directly on the runner** — an `npm ci` or `pip install`
against the checked-out source, a `make` target, a test runner invoked outside
the container, a `uses:` action whose input points into the PR checkout. Every
one of those executes with the job's ambient credentials in scope: the WIF token,
the Environment secrets, `GITHUB_TOKEN`. A `postinstall` script in the PR
author's `package.json` would be enough. If you are about to add a build-time
test or lint step to `deploy-integration.yml`, put it inside the image the same
way the application code already runs there — never as a bare step on the
runner.

## 2. The listener is attacker-controlled code

For `on: push: tags:`, GitHub loads the workflow file **from the tagged commit's
own tree**. Whoever creates the tag can rewrite every line of the listener,
including any check you add to it.

**Prevents:** the belief that validating in the listener is worth anything. It is
fail-fast ergonomics. The boundaries are invariant 4 and the re-validation in the
trusted workflow, and both must hold with the listener assumed hostile.

**Check:** the listener carries the comment saying so, and holds no secret,
environment, or federation step.

## 3. `environment:` derives from the ref, never from an input

The expression reads `github.ref_name`. A dispatch input may be cross-checked
against the ref — it may never be substituted for it.

**Prevents:** anyone with `workflow_dispatch` rights selecting `prod` from a
feature branch and receiving production secrets.

**Check:** a bare substring grep for `environment:` against `deploy.yml`
produces two false positives on the correct, unmodified template — the
`workflow_dispatch` input, which is itself **named** `environment` (its
declaration line is `      environment:`), and the Summary step's
`echo "- environment: ${{ steps.env.outputs.env }}"`, which reports the
resolved value but does not derive it. Filtering that grep down to lines
containing `${{` removes the input declaration (it has no expression on that
line) but *not* the echo line, which also contains `${{` — so `${{` alone is
not enough either.

The filter that actually isolates the job-level expression is a **line
anchor**: after stripping leading whitespace, the line must *begin with*
`environment:`, and it must also contain `${{`. The echo line fails the
anchor (it begins with `echo`, not `environment:`); the input declaration
fails the `${{` test. Only the job-level line,
`environment: ${{ github.ref_name == ... }}`, survives both:

```bash
grep -E '^[[:space:]]*environment:.*\$\{\{' deploy.yml
```

Every line that survives must contain `github.ref_name` and must not contain
`inputs.`. This is exactly what
`test_environment_derives_from_the_ref_not_an_input` in
`tests/test_cloud_run_deploy_skill.py` does —
`line.strip().startswith("environment:") and "${{" in line` — and the anchor
is not incidental to that test: drop it, and the test (like a hand-rolled
version of this Check) starts failing against correct, unmodified code. Before
"simplifying" this check back to a bare `environment:` grep, or even to a
`${{`-only filter, run it against the shipped template and confirm it still
passes.

**The filter must also produce at least one survivor — zero is a failure, not
a pass.** GitHub Actions accepts `environment:` in block form too:

```yaml
environment:
  name: ${{ inputs.environment }}
```

Neither line of that form begins with `environment:` *and* contains `${{` on
the same line, so this filter selects nothing from it. A regression rewritten
into block form therefore produces empty grep output — and "every surviving
line satisfies the rule" is vacuously true over an empty set, so a check that
only inspects survivors reports a pass on a file where `environment:` derives
from `inputs.environment` with no ref check at all. The test guards against
this explicitly — `assert environment_lines, "deploy.yml declares no job
environment expression"` — and this Check must be read the same way: empty
output is not "the invariant holds unchallenged", it is "the filter did not
find what it expects to find here", and the file must be inspected by hand
before concluding anything.

## 4. The WIF condition pins repository and branch refs only

`assertion.repository == '<repo>' && (assertion.ref == 'refs/heads/dev' || assertion.ref == 'refs/heads/main')`

**Prevents:** a federated token reachable by anyone who can push a tag. This is
the reason the review-tag path runs its privileged half from the trusted branch
rather than from the tag.

**Check:** `bootstrap-gcp.sh` contains `refs/heads/` and does not contain
`refs/tags/`.

## 5. Untrusted heads are checked out defensively

Into a subdirectory, with `persist-credentials: false`, only after validation.
Trusted control code is checked out separately, from the trusted branch.

**Prevents:** a token in the `.git/config` of attacker-controlled code.

## 6. Resource inspection fails closed

A `describe` that fails for any reason other than a recognized not-found
signature aborts the script.

**Prevents:** a transient permission or network error reading as "absent", after
which the bootstrap cheerfully recreates — or resets — a live resource.

**Check:** `bootstrap-gcp.sh` contains `refusing to treat it as absent`.

## 7. Environment-suffixed secret names for the destructive environment

`integration` uses `<NAME>_INTEGRATION`; `dev` and `prod` keep bare names.

**Prevents:** the worst failure in the whole design. GitHub resolves a secret
reference environment → repository → organization. A bare `DATABASE_URL` missing
from the `integration` Environment resolves *silently* to a repository- or
organization-scoped secret of the same name — and that DSN is handed to a job
whose entire purpose is to drop a schema. Suffixed, a missing secret resolves to
the empty string and the workflow's guard fails loudly.

`dev` and `prod` run migrations, never a destructive reset, so they are not
exposed to this and keep the bare names.

**Operator note:** the failure names the Secret Manager destination
(`database-url-integration`), not the GitHub Environment secret
(`DATABASE_URL_INTEGRATION`). Document that mapping in the runbook.

## 8. Runtime identities get resource-scoped IAM only

One service account per environment. Secret access bound per secret, storage per
bucket, impersonation per account. No runtime identity holds a project-level role.

**Prevents:** an integration deployment reading production secrets because both
runtimes happen to hold `roles/secretmanager.secretAccessor` on the project.

## 9. Immutable image tags from the validated head SHA

`<env>-<sha12>`, or `integration-pr-<n>-<sha12>`. Never `latest`.

**Prevents:** a revision nobody can trace to a commit, and a redeploy that
silently ships different bytes.

## 10. A destructive reset is gated three ways

`DEPLOY_ENV=integration`, `ALLOW_DATABASE_RESET=true`, and a marker row in a
schema outside the one being dropped.

**Prevents:** resetting the wrong database — as long as the misconfiguration is
accidental. Read that qualifier carefully, because the gate's protection stops
exactly there.

**This is a misconfiguration control, not a defence against a malicious PR.**
The reset job runs the PR author's own image, built from their own Dockerfile,
with the integration database's credentials mounted into it. The three-way gate
— including the marker — is checked by that entrypoint, and the entrypoint is
the PR author's code. Nothing stops them from shipping an entrypoint that skips
the check entirely, or one that reports success without touching the database,
or one that reads `DATABASE_URL_INTEGRATION` and does something else with it.
"The marker travels with the database" describes what stops a *correctly
configured, honest* entrypoint from resetting the wrong target when the two
environment variables are wrong. It says nothing about what a dishonest
entrypoint can be made to do, because a dishonest entrypoint is not obligated to
look at the marker at all. A reader who takes "the marker travels with the
database" as protection against an attacker-controlled image is worse off than
one who is told plainly that it is not — they will reach for the marker to
justify a design decision it cannot support.

**The actual control against a malicious PR reaching production data is
upstream of this gate, not inside it: the reset job's secrets name an
integration-only credential.** Invariant 7's `_INTEGRATION`-suffixed secrets are
what the PR author's image is handed — a role scoped to a disposable database
that holds no production data and that gets rebuilt from migrations on every
reset. The image can do anything it wants with that credential; the worst
outcome is damage to a database that is *supposed* to be reset routinely. If a
`DATABASE_URL_INTEGRATION` secret ever pointed at a production instance, or if
the integration service account's IAM binding (invariant 8) ever widened past
that one instance, this invariant would provide no backstop.

So: keep the three-way gate, because it is real insurance against a fat-fingered
`ALLOW_DATABASE_RESET=true` in the wrong environment. But when explaining this
invariant to someone, or reviewing a change to it, do not describe it as a
security boundary against the PR's own code — describe invariants 7 and 8 as
that boundary instead. The marker is initialized once, by an operator, against
the dedicated integration database. Never automate it — an automated write lets
a misconfigured deploy mark production as disposable and then reset it. That
risk is exactly the accidental-misconfiguration class this invariant defends
against; it is not the malicious-PR class, which invariants 7 and 8 defend
against instead.

## 11. Non-cancelling concurrency on shared environments

`concurrency: {group: integration, cancel-in-progress: false}`.

**Prevents:** a second review tag cancelling a run mid-reset and leaving a
half-migrated database that the next deploy then builds on.

## 12. Workload Identity Federation only

No service-account key is created, downloaded, stored in GitHub, or written to an
operator machine.

**Prevents:** a permanent, unrevoked credential sitting in a repo, a CI log, or a
laptop backup. WIF tokens expire; key files do not.

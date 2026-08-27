# Design: `cloud-run-continuous-deploy` skill

**Date:** 2026-08-26
**Status:** Approved for planning
**Prior art:** `datagenero/defensoria`, branch `agent/pr-integration-review`

## Problem

Standing up continuous deployment to Cloud Run is a multi-day job that is
mostly identical across repos and mostly wrong when done from memory. The
defensoria implementation got it right, but every name, path and secret in it
is hardcoded to that application. The valuable part — a dozen security
invariants, each guarding a specific and non-obvious failure — is embedded in
files that cannot be copied.

This skill extracts that reusable core so any repo with a `docker-compose.yml`
can be prepared for three-trigger continuous deployment without rediscovering
the invariants.

## Goal

Given a repo with a `docker-compose.yml`, produce a working, reviewed
deployment setup with three triggers:

| Event | GitHub Environment | Workflow |
| --- | --- | --- |
| push to `dev` (a PR merged into `dev`) | `dev` | `deploy.yml` |
| push to `main` (a PR merged into `main`) | `prod` | `deploy.yml` |
| tag matching `review/pr-<n>/<sha>` | `integration` | `integration-tag.yml` → `deploy-integration.yml` |

Branch names are asked per repo; `dev` and `main` are the defaults. The three
**GitHub Environment names are fixed** at `dev`, `prod` and `integration`
regardless of what the branches are called, because they are also the suffix
for every derived resource name and a configurable one buys nothing.

## Non-goals

- Implementing any application's database reset. The skill defines the
  contract and the guards; the repo supplies the entrypoint.
- Provisioning managed backing services (Cloud SQL, Memorystore, Supabase).
  The inventory names what must be externalized and stops there.
- Migrating an existing hand-written deployment. The skill writes new files
  and refuses to clobber; reconciliation is the operator's.
- Any non-GCP target.

---

## Section 1 — Deployment topology

### 1.1 Preferred: single container behind nginx

Every HTTP service from compose is baked into one image. nginx binds Cloud
Run's `$PORT` and reverse-proxies to loopback ports by path prefix:

```
$PORT ──nginx──┬── /api  → 127.0.0.1:8000   (api)
               └── /     → 127.0.0.1:3000   (web)
```

This is preferred because it removes machinery rather than adding it. Compared
with a service-per-container deployment it deletes:

- the CORS configuration entirely — same origin, so no `CORS_ORIGINS`
  environment variable and no post-deploy `gcloud run services update` step;
- the second Cloud Run service, its URL discovery, and its canonical-URL
  variant (defensoria needed both forms because Cloud Run serves two
  hostnames per service);
- the separate frontend service account;
- one scale-to-zero billing unit and one cold start per request path.

Processes are supervised by `deploy/entrypoint.sh`, not supervisord:

```bash
#!/usr/bin/env sh
set -eu
trap 'kill 0' EXIT INT TERM
<each service command> &
nginx -g 'daemon off;' &
wait -n
exit $?
```

`wait -n` returns when the *first* child exits, so a dead backend takes the
container down and Cloud Run replaces it. A supervisor that restarts children
in place would keep a broken revision serving traffic.

### 1.2 Fallback: service-per-container

Chosen when any of these hold, asked explicitly during inventory:

- two services need materially different CPU/memory or scaling limits;
- one service must be private (`--no-allow-unauthenticated`) while another is
  public;
- base images cannot be combined (for example a JVM service and a Node service
  where the combined image becomes unreasonable);
- the operator wants independent rollback per service.

This is defensoria's shape and the templates support it, including the CORS
step with both URL forms.

### 1.3 Service classification

Non-HTTP services become Cloud Run **jobs** in either topology. Stateful
services cannot run on Cloud Run at all and are reported as *externalize*:

| compose image | classification | managed equivalent named |
| --- | --- | --- |
| `postgres`, `mysql`, `mariadb` | externalize | Cloud SQL, Supabase, AlloyDB |
| `redis`, `valkey`, `memcached` | externalize | Memorystore |
| `minio` | externalize | Cloud Storage |
| `elasticsearch`, `opensearch` | externalize | Vertex AI Search, self-hosted GCE |
| `rabbitmq`, `kafka` | externalize | Pub/Sub |
| `qdrant`, `weaviate`, `chroma` | externalize | Vertex AI Vector Search, self-hosted |

Any service declaring a named volume is reported as stateful regardless of
image, because Cloud Run's filesystem does not survive a revision.

---

## Section 2 — The mapping worksheet

Inventory and generation are separated by one committed artifact,
`deploy/cloud-run.map.yml`. The agent fills it in with the operator during
inventory; template generation reads only this file. Re-running the skill is
therefore deterministic, and the mapping is reviewable in a PR.

```yaml
project_id: my-gcp-project
region: us-central1
integration_region: us-central1   # optional; defaults to `region`
ar_repo: myapp
topology: single-container        # or: per-service
branches:
  dev: dev
  prod: main
review_tag_pattern: "review/pr-*/*"
services:
  - compose: api
    role: http                    # http | job | externalize
    port: 8000
    path_prefix: /api
  - compose: web
    role: http
    port: 3000
    path_prefix: /
    build_args: [NEXT_PUBLIC_API_URL]
  - compose: worker
    role: job
    command: ["python", "-m", "app.worker"]
  - compose: db
    role: externalize
    managed: cloud-sql-postgres
reset_entrypoint: ./reset.sh      # optional; integration environment only
```

Exactly one `http` service may declare `path_prefix: /` under
`single-container`. Under `per-service`, `path_prefix` is ignored.

`integration_region` exists so the integration environment can be placed in a
different region from `dev` and `prod` — defensoria did this deliberately, and
it is worth keeping available: a region boundary means a fat-fingered
`gcloud run services list` or a region-scoped script cannot see, let alone
touch, the production services while working on a review deployment. It
defaults to `region` because the isolation costs an extra Artifact Registry
pull across regions on every review deploy, and not every repo will want to pay
that.

**Derived resource names.** Everything is mechanical from `ar_repo` and the
fixed environment name, so nothing else needs to be recorded:

| Resource | Pattern | Example (`integration`) |
| --- | --- | --- |
| Cloud Run service | `<compose>-<env>`, or `<ar_repo>-<env>` for single-container | `myapp-integration` |
| Cloud Run job | `<compose>-<env>` | `worker-integration` |
| Runtime service account | `<ar_repo>-runtime-<env>` | `myapp-runtime-integration` |
| Secret Manager secret | `<key-as-kebab>-<env>` | `database-url-integration` |
| Uploads bucket | `<ar_repo>-<env>-uploads` | `myapp-integration-uploads` |
| Image tag | `<env>-<sha12>` / `integration-pr-<n>-<sha12>` | `integration-pr-42-a1b2c3d4e5f6` |

The Secret Manager name is the GitHub Environment secret's key lowercased with
underscores turned into hyphens, plus the environment suffix — and it is
derived from the **bare** key, so `DATABASE_URL_INTEGRATION` in the
`integration` Environment still targets `database-url-integration`. That
mapping is the one §3.4 warns the operator about.

`inventory-compose.sh` is read-only. It resolves the compose file with
`docker compose config --format json` — which expands anchors, `extends` and
interpolation, so the skill never parses YAML itself — and formats the
worksheet with `python3` and the standard library. Both tools are assumed
present; their absence is a hard, explained failure, not a fallback.

---

## Section 3 — Secrets: Varlock to GitHub Environments

### 3.1 The constraint

The agent must never hold a secret value. This repo's own configuration
enforces that: `opencode.jsonc` denies `varlock printenv*`, `varlock load
--show*`, `varlock load --format*` and `varlock load -f*`, and the mirrored
`.claude/settings.json` denies reading `.env*`. The design works inside those
rules rather than around them.

### 3.2 Name discovery, by the agent

```bash
varlock load --agent --filter="@sensitive"
```

`--agent` redacts sensitive values and defaults to JSON; `--filter="@sensitive"`
selects exactly the items the deployment must carry. Neither flag is `--format`,
so the invocation sits outside the deny-glob and is legitimate for the agent to
run. `.env.schema` stays the single source of truth for *which* secrets exist,
with no duplicated list to drift.

The `.env*` deny-glob also blocks the agent from reading `.env.schema`
directly. That is intentional and the skill says so: ask the operator for names
that command cannot produce, never work around the rule.

If a Varlock release stops composing `--agent` with `--filter`, the fallback is
`--format json-full` **inside the sync script only**, filtering
`isSensitive == true`. The agent still never runs it.

### 3.3 Value transfer, by the script

`scripts/sync-github-secrets.sh <env>` is invoked as:

```bash
APP_ENV=integration varlock run --inject vars -- scripts/sync-github-secrets.sh integration
```

`varlock run` injects resolved values as process environment variables and
auto-redacts piped stdout; `--inject vars` omits the `__VARLOCK_ENV` blob. The
script obtains the key list from §3.2, reads each value from its own
environment by name, and pipes it to `gh` over stdin:

```bash
printf '%s' "$value" | gh secret set "$target" --env "$env"
```

stdin, never `--body`, because an argv value is visible in `ps`. No value is
ever echoed, logged, or written to a file. The script prints one line per key —
the key name and a checkmark — and a final count.

### 3.4 Environment-suffixed names for the destructive environment

For `dev` and `prod` the GitHub Environment secret keeps its bare name. For
`integration` the target is `<NAME>_INTEGRATION`.

This asymmetry is deliberate and is the single most important thing the skill
carries forward. GitHub resolves a secret reference environment → repository →
organization. A bare `DATABASE_URL` in the `integration` Environment therefore
falls back silently to a repository- or organization-scoped secret of the same
name if the Environment secret is missing, unset, or not yet created — and that
DSN is then handed to a job whose entire purpose is to drop a schema. The
suffix makes a missing Environment secret resolve to the empty string, and the
workflow's guard fails loudly instead.

`dev` and `prod` run migrations, never a destructive reset, so they are not
exposed to this failure mode and keep the bare names.

The generated runbook must state that the resulting failure names the Secret
Manager *destination* (`database-url-integration`), not the GitHub Environment
secret (`DATABASE_URL_INTEGRATION`), and must give the operator that mapping.

### 3.5 Environment creation

Three environments are created: `dev`, `prod`, `integration`. **Each is created with
a deployment branch policy naming only its trusted branch.** The policy is not
optional and not defence in depth — without it, invariant 1 is false.

```bash
create_environment() {          # $1 = environment name, $2 = its only allowed branch
  gh api -X PUT "repos/$REPO/environments/$1" \
    -F 'deployment_branch_policy[protected_branches]=false' \
    -F 'deployment_branch_policy[custom_branch_policies]=true'
  gh api -X POST "repos/$REPO/environments/$1/deployment-branch-policies" \
    -f "name=$2"
}

create_environment dev         "$DEV_BRANCH"
create_environment prod        "$PROD_BRANCH"
create_environment integration "$DEV_BRANCH"

gh variable set NAME --env "$ENV"     # non-secret configuration
```

**Why the policy is load-bearing.** `deploy-integration.yml` binds
`environment: integration` unconditionally, because it is only ever meant to be
reached through the `validate` job. But a `workflow_dispatch` run executes the
workflow file **from the dispatched ref**. Anyone with push access can therefore
push a branch carrying a modified copy of that workflow — `needs: validate`
deleted, a step added that exfiltrates `${{ toJSON(secrets) }}` — and dispatch it
from the attacker-authored listener, which holds `actions: write`. The run binds
`environment: integration` and receives every secret in it.

The WIF attribute condition (§4, invariant 4) correctly denies the *GCP* token for
a ref outside the trusted branches, so no Google credential is issued. It does
nothing about GitHub Environment secrets, and the integration `DATABASE_URL`
alone is enough. The branch policy is what closes this; nothing else in the
design does.

`deploy.yml` is not exposed the same way — its `environment:` is an expression
over `github.ref_name` that yields `invalid` for any other ref (invariant 3) — but
all three environments get the policy anyway, because relying on an expression to
be written correctly forever is weaker than a server-side constraint.

---

## Section 4 — The security invariants

These are the skill's payload. `reference.md` states each one with the failure
it prevents; a template that violates one is a bug in the skill.

1. **An untrusted ref never receives credentials.** The tag path is split into
   an unprivileged listener, a credential-free validation job, and a
   privileged deploy job. Nothing that runs before validation can reach WIF,
   an Environment, or a secret.

2. **The listener is attacker-controlled code.** For `on: push: tags:`, GitHub
   loads the workflow file from the tagged commit's own tree, so whoever
   creates the tag can rewrite every line of the listener. It is fail-fast
   ergonomics, never a boundary. The real boundaries are the federation
   attribute condition and the trusted workflow's own re-validation, and both
   must hold with the listener assumed hostile.

3. **`environment:` derives from the ref, never from an input.** A dispatch
   input selecting the environment hands production secrets to anyone with
   dispatch rights. The expression reads `github.ref_name`; an input may only
   be cross-checked against it, never substituted for it.

4. **The WIF attribute condition pins repository and trusted branch refs
   only.** `assertion.repository == '<repo>' && (assertion.ref ==
   'refs/heads/dev' || assertion.ref == 'refs/heads/main')`. It is never
   widened to tag refs — that would hand a federated token to anyone who can
   push a tag.

5. **Untrusted heads are checked out defensively:** into a subdirectory, with
   `persist-credentials: false`, and only after validation. Trusted control
   code is checked out separately from the trusted branch.

6. **Resource inspection fails closed.** A `describe` that fails for any reason
   other than a recognized not-found signature aborts the script. Treating an
   ambiguous error as "absent" is how a bootstrap recreates or resets a live
   resource.

7. **Environment-suffixed secret names for any destructive environment.**
   See §3.4.

8. **Per-environment runtime service accounts with resource-scoped IAM only.**
   No runtime identity receives a project-level role. Secret access is bound
   per secret, storage per bucket, and impersonation per service account.

9. **Immutable image tags derived from the validated head SHA.** Never
   `latest`. The tag is `<env>-<sha12>`, or `integration-pr-<n>-<sha12>` for
   review deployments, so a deployed revision is traceable to one commit.

10. **A destructive reset is gated three ways:** `DEPLOY_ENV=integration`,
    `ALLOW_DATABASE_RESET=true`, and a marker row in a schema outside the one
    being dropped. The reset queries the marker before any `DROP` and refuses
    an unmarked database, a wrong or duplicated marker, or an inaccessible
    one. Two environment variables alone are not sufficient — they travel with
    a misconfigured workflow; the marker travels with the database.

11. **Non-cancelling concurrency on shared environments.** The integration
    group sets `cancel-in-progress: false`. Cancelling mid-reset leaves a
    half-migrated database.

12. **Workload Identity Federation only.** No service-account key is ever
    created, downloaded, stored in GitHub, or written to an operator machine.

---

## Section 5 — The integration environment's reset contract

The skill cannot know any application's database. It defines the contract and
supplies the guards; the repo supplies the entrypoint.

**The skill provides:**

- `templates/integration-marker.sql` — creates a protected marker outside
  `public`, idempotently:

  ```sql
  CREATE SCHEMA IF NOT EXISTS review_control;
  CREATE TABLE IF NOT EXISTS review_control.environment_marker (
      environment text PRIMARY KEY
  );
  INSERT INTO review_control.environment_marker (environment)
  VALUES ('integration')
  ON CONFLICT DO NOTHING;
  ```

- the workflow step that runs the reset as a Cloud Run job with `DEPLOY_ENV`
  and `ALLOW_DATABASE_RESET` set, ordered before migrations and before any
  service deploy;
- a checklist for the entrypoint the repo must write: verify both flags,
  verify exactly one marker row for `integration`, and only then drop.

**The repo provides:** the executable named by `reset_entrypoint`. If
`reset_entrypoint` is absent from the worksheet, the reset step is omitted
entirely and the integration environment runs migrations only. The workflow
must not be generated with a reset step that points at a missing entrypoint.

Marker initialization is a one-time manual operator step against the dedicated
integration database, documented in the generated runbook and never automated —
an automated marker write would defeat the guard's purpose.

---

## Section 6 — Skill contents and procedure

### 6.1 Layout

```
skills/delivery/cloud-run-continuous-deploy/
  SKILL.md                     procedure and invariants, dense
  reference.md                 each invariant with its failure mode
  agents/openai.yaml           codex interface block
  scripts/
    inventory-compose.sh       read-only compose → worksheet
  templates/
    deploy.yml
    integration-tag.yml
    deploy-integration.yml
    bootstrap-gcp.sh
    review_tag.py
    sync-github-secrets.sh
    nginx.conf
    entrypoint.sh
    Dockerfile.combined
    integration-marker.sql
```

`review_tag.py` ships verbatim — tag grammar plus open-PR and head-SHA
validation against `gh api` is application-independent. It is the one template
with no substitutions.

### 6.2 Procedure

| Phase | Action | Who runs it |
| --- | --- | --- |
| 0 | Preconditions: `gh auth status`, `gcloud`, `varlock`, committed `.env.schema`, a compose file, both branches exist | agent |
| 1 | `inventory-compose.sh`; interview per service; write `deploy/cloud-run.map.yml` | agent, with operator |
| 2 | Create the three environments; write and run `sync-github-secrets.sh` once per environment | agent |
| 3 | Generate `scripts/bootstrap-gcp.sh`; **operator runs it**; set repo secrets `GCP_PROJECT_ID` and `WIF_PROVIDER` from its output | operator |
| 4 | Generate the three workflows, plus `deploy/` for single-container topology | agent |
| 5 | Lint, walk the invariant checklist, then a live acceptance run | agent, with operator |

Phase 3 is the operator's because bootstrap needs project-admin rights that a
deploy identity must never hold.

Phase 5's acceptance run is the only proof that matters: open a harmless PR
into `dev`, confirm the dev deploy; merge to `main`, confirm prod; push a
`review/pr-<n>/<sha>` tag and confirm the listener dispatches, validation
passes, the integration environment deploys, and the evidence artifact carries
the SHA and image digests.

### 6.3 Clean-adds only

The skill never overwrites an existing workflow, script, or Dockerfile. A
collision is reported with the path and deferred to the operator. This matches
the bootstrap posture the rest of this repo already takes.

---

## Section 7 — Registration in botica

The skill is a house-baseline local skill in a new `delivery` category. All
registration points are already test-enforced by `tests/test_skill_catalog.py`:

| Point | File | Enforced by |
| --- | --- | --- |
| Baseline list | `scaffolding/skills.py` `LOCAL_SKILLS` | — |
| On-disk layout | `skills/delivery/cloud-run-continuous-deploy/SKILL.md` | `test_local_skills_exist_on_disk` |
| Codex policy | `agents/openai.yaml` | `test_user_invoked_local_skills_carry_codex_policy` |
| Install command | `README.md` (×2), `guide.md` (×2) | `test_docs_carry_the_exact_local_skill_list` |
| Catalog prose | `README.md` | — |
| Router entry | `skills/productivity/ask-user/SKILL.md` | `test_ask_user_routes_every_installed_skill` |

`_local_skill_root` globs `skills/*/<name>/SKILL.md`, so the new `delivery`
category needs no test change.

### 7.1 New tests

`tests/test_cloud_run_deploy_skill.py` asserts the shipped templates uphold the
invariants that can be checked statically:

- the listener template contains no `google-github-actions/auth`, no
  `secrets.` reference, and no `environment:` key (invariants 1, 2);
- `deploy.yml`'s `environment:` expression references `github.ref_name` and
  does not reference `inputs.` (invariant 3);
- `bootstrap-gcp.sh`'s WIF attribute condition contains `refs/heads/` and no
  `refs/tags/` (invariant 4);
- the untrusted checkout in `deploy-integration.yml` sets
  `persist-credentials: false` (invariant 5);
- `deploy-integration.yml` sets `cancel-in-progress: false` (invariant 11);
- no template contains `iam service-accounts keys create` (invariant 12).

These tests are the reason the invariants survive future edits to the
templates.

---

## Open questions

None.

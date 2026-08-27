---
name: cloud-run-continuous-deploy
description: Prepare a repo with a docker-compose.yml for continuous deployment to Google Cloud Run — dev on merge to dev, prod on merge to main, and a resettable integration environment from an immutable PR review tag. Use when wiring a repo to Cloud Run, when a deploy workflow hands credentials to an untrusted ref, when review deployments need isolation from prod, or when secrets must reach GitHub Environments without passing through an agent.
---

# Cloud Run continuous deploy

Three triggers, three environments, one rule: **an untrusted ref never receives a credential.**

| Event | Environment | Workflow |
| --- | --- | --- |
| merge into `dev` | `dev` | `deploy.yml` |
| merge into `main` | `prod` | `deploy.yml` |
| tag `review/pr-<n>/<sha>` | `integration` | `integration-tag.yml` → `deploy-integration.yml` |

Read [reference.md](reference.md) before generating anything. It holds the twelve
invariants and the failure each one prevents. The templates encode them; the
reference is why you must not edit them into something more convenient.

## Phase 0 — Preconditions

Stop and report if any of these fail. Do not work around them.

```bash
gh auth status
command -v gcloud                    # one per tool, on purpose
command -v varlock
command -v docker
command -v python3
git rev-parse --verify origin/dev && git rev-parse --verify origin/main
ls .env.schema docker-compose.yml
```

One `command -v` per tool is not verbosity. `command -v gcloud varlock docker
python3` prints whichever names it resolves and **exits 0 if any single one of
them resolves** — so it passes with `gcloud` and `varlock` both missing, and
Phase 3 then fails in the operator's hands instead of here.

## Phase 1 — Inventory and worksheet

```bash
scripts/inventory-compose.sh
```

It classifies every compose service as `http`, `job` or `externalize` and prints
a starter worksheet after a `--- worksheet ---` line. **Every value it emits is a
guess except the service names.** Walk them with the operator, then write the
confirmed result to `deploy/cloud-run.map.yml`.

Ask, in this order:

1. **Topology.** Prefer `single-container` — one nginx process fronting every
   HTTP service on Cloud Run's `$PORT`. It removes CORS entirely, along with the
   second service, its canonical-URL variant and its service account. Choose
   `per-service` only when two services need different scaling or resource
   profiles, one must stay private while another is public, or the base images
   cannot reasonably be combined.
2. **Path prefixes**, if single-container. Exactly one service takes `/`.
3. **Externalized services.** Name the managed replacement for each. Do not
   generate anything that assumes a compose-provided database still exists.
4. **`reset_entrypoint`.** Optional. Omit it and the integration environment
   runs migrations only — never generate a reset step pointing at a missing
   entrypoint. `deploy-integration.yml` ships the reset step hardcoded, so
   "omit it" means an edit, not a substitution: **delete the whole
   `- name: Reset, migrate and seed the integration database` step** (both the
   `gcloud run jobs deploy` and the `gcloud run jobs execute` invocation, down
   to the blank line before `- name: Deploy the integration service`), and then
   also drop `resources/integration-marker.sql` from Phase 4's file table and
   the marker command from Phase 5 — with no reset there is nothing for the
   marker to gate.
5. **`integration_region`.** Optional; defaults to `region`. A separate region
   for `integration` means a region-scoped script or a fat-fingered
   `gcloud run services list` cannot see, let alone touch, production while
   working on a review deployment. It costs a cross-region Artifact Registry
   pull on every review deploy.

## Phase 2 — Environments and secrets

You must never hold a secret value. Discover names, never values:

```bash
varlock load --agent --filter='@sensitive'
```

`--agent` redacts values and neither flag is `--format`, so this stays outside
the agent deny-glob. If you need something that command cannot tell you, **ask
the operator** — `.env.schema` is deny-listed on purpose and reading it around
the rule is not an option.

Create each environment **with a deployment branch policy naming only its trusted
branch**. This is not optional. `deploy-integration.yml` binds
`environment: integration` unconditionally, and a `workflow_dispatch` run executes
the workflow file *from the dispatched ref* — so without the policy, anyone with
push access can push a branch carrying a modified copy of that workflow (with
`needs: validate` removed and a step that exfiltrates `${{ toJSON(secrets) }}`),
dispatch it from the attacker-authored listener, and receive every integration
secret. The WIF condition denies the GCP token for such a ref, but does nothing
about GitHub Environment secrets, and the integration `DATABASE_URL` alone is
enough. See the spec's §3.5 for the full attack sequence.

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
```

Verify the policy took effect before continuing — an environment created without
it looks identical in the API response that creates it:

```bash
gh api "repos/$REPO/environments/integration/deployment-branch-policies" \
  --jq '.branch_policies[].name'      # must print the trusted branch, and only it
```

Copy `templates/sync-github-secrets.sh` to `scripts/`, **make it executable**,
then run it once per environment. The operator populates Varlock first:

```bash
cp <skill>/templates/sync-github-secrets.sh scripts/sync-github-secrets.sh
chmod +x scripts/sync-github-secrets.sh      # a copy does not carry the mode

APP_ENV=dev         varlock run --inject vars -- scripts/sync-github-secrets.sh dev
APP_ENV=prod        varlock run --inject vars -- scripts/sync-github-secrets.sh prod
APP_ENV=integration varlock run --inject vars -- scripts/sync-github-secrets.sh integration
```

**The `integration` environment's secrets are suffixed `_INTEGRATION`.** This is
not a style choice — see invariant 7. Tell the operator that a later failure
naming `database-url-integration` means the GitHub Environment secret
`DATABASE_URL_INTEGRATION` is missing.

## Phase 3 — GCP bootstrap (the operator runs this)

Copy `templates/bootstrap-gcp.sh` to `scripts/`, substituting `{{AR_REPO}}`,
`{{DEV_BRANCH}}`, `{{PROD_BRANCH}}` and `{{SECRET_KEYS}}`, and **make it
executable**. Then hand it over:

```bash
chmod +x scripts/bootstrap-gcp.sh            # a copy does not carry the mode

scripts/bootstrap-gcp.sh --project PROJECT --region REGION \
  --integration-region INTEGRATION_REGION --repo ORG/REPO
```

Pass `--integration-region` even when it equals `--region`: it is the value
`{{INTEGRATION_REGION}}` in `deploy-integration.yml` resolves to, and leaving it
implicit is how that placeholder ends up unsubstituted.

You do not run this. It needs project-admin rights that no deploy identity may
hold. It prints `GCP_PROJECT_ID` and `WIF_PROVIDER` — set both as **repository**
secrets, not environment secrets.

## Phase 4 — Generate

Copy and substitute. Never overwrite: if a target path exists, stop and report it.

| Template | Target |
| --- | --- |
| `deploy.yml` | `.github/workflows/deploy.yml` |
| `integration-tag.yml` | `.github/workflows/integration-tag.yml` |
| `deploy-integration.yml` | `.github/workflows/deploy-integration.yml` |
| `review_tag.py` | `scripts/review_tag.py` (verbatim — the only file with no placeholders) |
| `nginx.conf` | `deploy/nginx.conf` — substitute `{{API_PREFIX}}`, `{{API_PORT}}`, `{{WEB_PORT}}` (single-container only) |
| `entrypoint.sh` | `deploy/entrypoint.sh` — substitute `{{API_COMMAND}}`, `{{WEB_COMMAND}}` (single-container only; the Dockerfile chmods it at build time) |
| `Dockerfile.combined` | `deploy/Dockerfile` — **renamed**, not `deploy/Dockerfile.combined` (single-container only) |
| `integration-marker.sql` | `resources/integration-marker.sql` (verbatim) |

`nginx.conf` and `entrypoint.sh` are **not** copied unchanged. Both carry
placeholders, and both fail at container start rather than at generation time if
you leave them: nginx cannot parse `proxy_pass http://127.0.0.1:{{WEB_PORT}};`,
and `{{API_COMMAND}} &` is not a command. The container exits on its first boot.

The Dockerfile rename is required, not cosmetic: both `deploy.yml` and
`deploy-integration.yml` hardcode `docker build -f deploy/Dockerfile .`. Copy
`Dockerfile.combined` under its template name and the first build in Phase 5
fails on a missing file.

### Derived resource names

Nothing below is recorded in the worksheet, because all of it is mechanical from
`ar_repo` and the fixed environment name (`dev`, `prod`, `integration` — fixed
regardless of what the branches are called).

| Resource | Pattern | Example (`integration`) |
| --- | --- | --- |
| Cloud Run service | `<compose>-<env>`, or `<ar_repo>-<env>` for single-container | `myapp-integration` |
| Cloud Run job | `<compose>-<env>` | `worker-integration` |
| Runtime service account | `<ar_repo>-runtime-<env>` | `myapp-runtime-integration` |
| Secret Manager secret | `<key-as-kebab>-<env>` | `database-url-integration` |
| Uploads bucket | `<ar_repo>-<env>-uploads` | `myapp-integration-uploads` |
| Image tag | `<env>-<sha12>` / `integration-pr-<n>-<sha12>` | `integration-pr-42-a1b2c3d4e5f6` |

**The Secret Manager name is derived from the bare key**: lowercase it, turn
underscores into hyphens, append `-<env>`. The `_INTEGRATION` suffix on the
GitHub Environment secret (invariant 7) is *not* part of the key — so
`DATABASE_URL_INTEGRATION` in the `integration` Environment still targets
`database-url-integration`, exactly as `DATABASE_URL` in `dev` targets
`database-url-dev`. This mapping is what `{{SYNC_SECRET_CALLS}}` and
`{{RUNTIME_SECRETS}}` encode, and it is the one Phase 2 warns the operator
about: a failure naming `database-url-integration` means the *GitHub* secret
`DATABASE_URL_INTEGRATION` is missing.

### Placeholders

Every `{{...}}` in every template. Substitute all of them; a leftover placeholder
is a runtime failure, not a generation-time one.

| Placeholder | Used by | Substitute with |
| --- | --- | --- |
| `{{AR_REPO}}` | `deploy.yml`, `deploy-integration.yml`, `bootstrap-gcp.sh` | `ar_repo` from the worksheet. Also the stem of every derived name above. |
| `{{DEV_BRANCH}}` | `deploy.yml`, `deploy-integration.yml`, `integration-tag.yml`, `bootstrap-gcp.sh` | `branches.dev` (default `dev`). In `deploy-integration.yml` this is the trusted control ref — see invariant 1. |
| `{{PROD_BRANCH}}` | `deploy.yml`, `bootstrap-gcp.sh` | `branches.prod` (default `main`). |
| `{{REGION}}` | `deploy.yml`, `deploy-integration.yml` | `region`. In `deploy-integration.yml` it is `AR_REGION` — where the Artifact Registry repo lives, which stays the primary region even when integration does not. |
| `{{INTEGRATION_REGION}}` | `deploy-integration.yml` | `integration_region`, defaulting to `region`. Pass it to `bootstrap-gcp.sh --integration-region` too. |
| `{{REVIEW_TAG_PATTERN}}` | `integration-tag.yml` | `review_tag_pattern` (default `review/pr-*/*`). The listener's `on: push: tags:` glob. |
| `{{HEALTH_PATH}}` | `deploy.yml`, `deploy-integration.yml` | The path the smoke test curls, e.g. `/api/health`. Must be unauthenticated and cheap. |
| `{{MAX_INSTANCES}}` | `deploy.yml` | `--max` for `dev`/`prod`. Integration is hardcoded to 1. |
| `{{SECRET_KEYS}}` | `bootstrap-gcp.sh` | A bash array body: the **already-kebabbed** keys, quoted and space-separated — `'database-url' 'session-secret'`. The line is `SECRET_KEYS=({{SECRET_KEYS}})` and the script appends `-<env>` to each, so passing `DATABASE_URL` here creates a secret literally named `DATABASE_URL-dev`. |
| `{{SECRET_ENV_BLOCK}}` | `deploy.yml`, `deploy-integration.yml` | YAML `env:` entries mapping each bare key to its Environment secret: `DATABASE_URL: ${{ secrets.DATABASE_URL }}` in `deploy.yml`, `${{ secrets.DATABASE_URL_INTEGRATION }}` in `deploy-integration.yml`. |
| `{{SYNC_SECRET_CALLS}}` | `deploy.yml`, `deploy-integration.yml` | One `sync_secret "<kebab>-<env>" "$KEY"` per key, e.g. `sync_secret "database-url-integration" "$DATABASE_URL"`. |
| `{{RUNTIME_SECRETS}}` | `deploy.yml`, `deploy-integration.yml` | The `--set-secrets` value for the service: `KEY=<kebab>-<env>:latest`, comma-separated. `:latest` is a *secret version*, not an image tag — invariant 9 does not apply. |
| `{{MIGRATE_SECRETS}}` | `deploy.yml` | Same form, for the migrate job. Usually just the database. |
| `{{RESET_SECRETS}}` | `deploy-integration.yml` | Same form, `-integration` suffixed. This is the credential the PR author's image is handed; invariants 7 and 8 are what keep it harmless. |
| `{{MIGRATE_ENTRYPOINT}}` | `deploy.yml` | The image command that runs migrations, e.g. `./migrate.sh`. |
| `{{RESET_ENTRYPOINT}}` | `deploy-integration.yml` | `reset_entrypoint` from the worksheet. If it was omitted, delete the reset step instead of substituting — see Phase 1. |
| `{{API_PREFIX}}` | `nginx.conf` | The `path_prefix` of the non-root HTTP service, without a trailing slash (`/api`). |
| `{{API_PORT}}`, `{{WEB_PORT}}` | `nginx.conf` | The loopback `port` of each HTTP service from the worksheet. |
| `{{API_COMMAND}}`, `{{WEB_COMMAND}}` | `entrypoint.sh` | The command that starts each HTTP service in the foreground, bound to its loopback port. |
| `{{API_BASE_IMAGE}}`, `{{WEB_BASE_IMAGE}}` | `Dockerfile.combined` | The build-stage base image for each service. |
| `{{API_CONTEXT}}`, `{{WEB_CONTEXT}}` | `Dockerfile.combined` | Each service's compose `build.context`, relative to the repo root. |
| `{{API_BUILD_COMMAND}}`, `{{WEB_BUILD_COMMAND}}` | `Dockerfile.combined` | Each service's build step (`npm ci && npm run build`, `uv sync --frozen`). |
| `{{RUNTIME_BASE_IMAGE}}` | `Dockerfile.combined` | The final-stage image both services run in. It must have `bash` and `gettext-base`; the Dockerfile installs them. |

Under `per-service` topology, `nginx.conf`, `entrypoint.sh` and
`Dockerfile.combined` are not generated at all, so their twelve container
placeholders do not apply.

Then write `docs/cloud-deploy.md` recording: the Supabase or Cloud SQL instance
per environment, the marker initialization command below, the GitHub Environment
secret names, and the Secret Manager mapping from the table above.

### The one-time marker initialization

Invariant 10's third gate is a marker row that must exist before the first
integration reset succeeds. Nothing in the design writes it — deliberately: an
automated marker write would let a misconfigured deploy mark production as
disposable and then reset it. The **operator** runs this once, by hand, against
the **dedicated integration database and no other**:

```bash
psql "$DATABASE_URL_INTEGRATION" -v ON_ERROR_STOP=1 -f resources/integration-marker.sql
```

It is idempotent (`CREATE ... IF NOT EXISTS`, `ON CONFLICT DO NOTHING`), so a
re-run is harmless — but a run against the wrong DSN is how a production
database gets marked resettable. Record the exact command and the instance it
was run against in `docs/cloud-deploy.md`.

## Phase 5 — Verify

Walk every invariant in [reference.md](reference.md) against the generated files.
Then the acceptance run, which is the only proof that counts:

1. Open a harmless PR into `dev`, merge it, confirm the `dev` deploy.
2. Merge `dev` into `main`, confirm the `prod` deploy.
3. **Have the operator initialize the marker, once, against the integration
   database** — `psql "$DATABASE_URL_INTEGRATION" -v ON_ERROR_STOP=1 -f
   resources/integration-marker.sql`. Step 5's reset is gated on it (invariant
   10) and fails without it. You do not run this; you do not hold the DSN.
4. Open a PR into `dev`, tag its head `review/pr-<n>/<sha>`, push the tag.
5. Confirm: the listener dispatches, `validate` passes without credentials, the
   integration environment deploys, smoke tests pass, and the evidence artifact
   carries the head SHA and the image reference.
6. Confirm the services scaled to zero afterwards.

## Do not

- Do not widen the WIF attribute condition to tag refs to "make the tag flow simpler". That is the attack.
- Do not add a check to `integration-tag.yml` and believe it protects anything.
- Do not let a `workflow_dispatch` input choose an environment.
- Do not create a service-account key. Ever, for any reason.

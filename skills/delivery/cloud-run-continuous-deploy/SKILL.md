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
command -v gcloud varlock docker python3
git rev-parse --verify origin/dev && git rev-parse --verify origin/main
ls .env.schema docker-compose.yml
```

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
   entrypoint.

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

Copy `templates/sync-github-secrets.sh` to `scripts/`, then run it once per
environment. The operator populates Varlock first:

```bash
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
`{{DEV_BRANCH}}`, `{{PROD_BRANCH}}` and `{{SECRET_KEYS}}`. Then hand it over:

```bash
scripts/bootstrap-gcp.sh --project PROJECT --region REGION --repo ORG/REPO
```

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
| `review_tag.py` | `scripts/review_tag.py` (verbatim, no substitution) |
| `nginx.conf`, `entrypoint.sh`, `Dockerfile.combined` | `deploy/` (single-container only) |
| `integration-marker.sql` | `resources/integration-marker.sql` |

Then write `docs/cloud-deploy.md` recording: the Supabase or Cloud SQL instance
per environment, the one-time marker initialization command, the GitHub
Environment secret names, and the Secret Manager mapping from Phase 2.

## Phase 5 — Verify

Walk every invariant in [reference.md](reference.md) against the generated files.
Then the acceptance run, which is the only proof that counts:

1. Open a harmless PR into `dev`, merge it, confirm the `dev` deploy.
2. Merge `dev` into `main`, confirm the `prod` deploy.
3. Open a PR into `dev`, tag its head `review/pr-<n>/<sha>`, push the tag.
4. Confirm: the listener dispatches, `validate` passes without credentials, the
   integration environment deploys, smoke tests pass, and the evidence artifact
   carries the head SHA and the image reference.
5. Confirm the services scaled to zero afterwards.

## Do not

- Do not widen the WIF attribute condition to tag refs to "make the tag flow simpler". That is the attack.
- Do not add a check to `integration-tag.yml` and believe it protects anything.
- Do not let a `workflow_dispatch` input choose an environment.
- Do not create a service-account key. Ever, for any reason.

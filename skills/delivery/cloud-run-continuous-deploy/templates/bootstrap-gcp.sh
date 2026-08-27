#!/usr/bin/env bash
# Idempotent bootstrap of the GCP resources this repo's deployment needs.
#
#   scripts/bootstrap-gcp.sh --project PROJECT [--region REGION] \
#     [--integration-region REGION] --repo ORG/REPO
#
# Run by an operator with project-admin rights. The deploy identity this script
# creates must never hold them.
set -euo pipefail

usage() {
  printf 'Usage: %s --project PROJECT [--region REGION] [--integration-region REGION] --repo ORG/REPO\n' "$0" >&2
}

die() {
  printf 'bootstrap-gcp: %s\n' "$1" >&2
  usage
  exit 2
}

PROJECT=''
REGION='us-central1'
INTEGRATION_REGION=''
REPO=''

while [[ $# -gt 0 ]]; do
  case "$1" in
    --project|--region|--integration-region|--repo)
      [[ $# -ge 2 && -n "$2" && "$2" != -* ]] || die "missing value for $1"
      case "$1" in
        --project) [[ -z "$PROJECT" ]] || die 'duplicate --project'; PROJECT="$2" ;;
        --region) REGION="$2" ;;
        --integration-region) INTEGRATION_REGION="$2" ;;
        --repo) [[ -z "$REPO" ]] || die 'duplicate --repo'; REPO="$2" ;;
      esac
      shift 2
      ;;
    *) die "unknown argument: $1" ;;
  esac
done

[[ -n "$PROJECT" ]] || die 'missing required --project'
[[ -n "$REPO" ]] || die 'missing required --repo'
[[ "$REPO" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || die '--repo must be ORG/REPO'
[[ "$PROJECT" =~ ^[a-z0-9-]+$ ]] || die '--project must be lowercase letters, digits and hyphens'
[[ -n "$INTEGRATION_REGION" ]] || INTEGRATION_REGION="$REGION"

command -v gcloud >/dev/null 2>&1 || die 'gcloud CLI is required'

AR_REPO='{{AR_REPO}}'
DEV_BRANCH='{{DEV_BRANCH}}'
PROD_BRANCH='{{PROD_BRANCH}}'
POOL='github-pool'
PROVIDER='github-provider'
DEPLOY_SA="${AR_REPO}-deployer"
DEPLOY_SA_EMAIL="${DEPLOY_SA}@${PROJECT}.iam.gserviceaccount.com"

lowercase() { printf '%s' "$1" | tr '[:upper:]' '[:lower:]'; }

DESCRIBE_OUTPUT=''

# Fail closed. A describe that fails for any reason other than a recognised
# not-found signature aborts: treating an ambiguous error as "absent" is how a
# bootstrap silently recreates or resets a live resource.
describe_resource() {
  local output
  if output=$("$@" 2>&1); then
    DESCRIBE_OUTPUT="$output"
    return 0
  fi
  shopt -s nocasematch
  case "$output" in
    *'not_found'*|*'not found'*|*'does not exist'*|*'http 404'*|*'404 not found'*)
      shopt -u nocasematch
      DESCRIBE_OUTPUT="$output"
      return 1
      ;;
  esac
  shopt -u nocasematch
  printf 'bootstrap-gcp: resource inspection failed; refusing to treat it as absent:\n%s\n' \
    "${output:-<no diagnostic output>}" >&2
  exit 1
}

printf '== Enabling APIs ==\n'
gcloud services enable \
  artifactregistry.googleapis.com cloudresourcemanager.googleapis.com \
  iam.googleapis.com iamcredentials.googleapis.com run.googleapis.com \
  secretmanager.googleapis.com storage.googleapis.com sts.googleapis.com \
  --project="$PROJECT"

PROJECT_NUM="$(gcloud projects describe "$PROJECT" --format='value(projectNumber)')"

printf '== Artifact Registry ==\n'
if describe_resource gcloud artifacts repositories describe "$AR_REPO" \
  --location="$REGION" --project="$PROJECT" --format='value(format,location)'; then
  read -r repo_format repo_location <<<"$DESCRIBE_OUTPUT"
  [[ "$(lowercase "$repo_format")" == 'docker' ]] || die "$AR_REPO is not a Docker repository"
  [[ "$(lowercase "$repo_location")" == "$(lowercase "$REGION")" ]] ||
    die "$AR_REPO is in $repo_location, expected $REGION"
else
  gcloud artifacts repositories create "$AR_REPO" \
    --repository-format=docker --location="$REGION" --project="$PROJECT"
fi

printf '== Workload Identity Federation ==\n'
# Branch refs only. Widening this to tag refs would hand a federated token to
# anyone who can push a tag, which is the whole reason the review-tag path runs
# its privileged half from the trusted branch instead.
ATTRIBUTE_CONDITION="assertion.repository == '${REPO}' && (assertion.ref == 'refs/heads/${DEV_BRANCH}' || assertion.ref == 'refs/heads/${PROD_BRANCH}')"

if ! describe_resource gcloud iam workload-identity-pools describe "$POOL" \
  --location=global --project="$PROJECT"; then
  gcloud iam workload-identity-pools create "$POOL" --location=global --project="$PROJECT"
fi

if describe_resource gcloud iam workload-identity-pools providers describe "$PROVIDER" \
  --location=global --workload-identity-pool="$POOL" --project="$PROJECT"; then
  PROVIDER_VERB='update-oidc'
else
  PROVIDER_VERB='create-oidc'
fi

gcloud iam workload-identity-pools providers "$PROVIDER_VERB" "$PROVIDER" \
  --location=global \
  --workload-identity-pool="$POOL" \
  --issuer-uri=https://token.actions.githubusercontent.com \
  --attribute-mapping='google.subject=assertion.sub,attribute.repository=assertion.repository,attribute.ref=assertion.ref' \
  --attribute-condition="$ATTRIBUTE_CONDITION" \
  --project="$PROJECT"

printf '== Service accounts ==\n'
RUNTIME_ACCOUNTS=()
for environment in dev prod integration; do
  RUNTIME_ACCOUNTS+=("${AR_REPO}-runtime-${environment}")
done

for account in "$DEPLOY_SA" "${RUNTIME_ACCOUNTS[@]}"; do
  email="${account}@${PROJECT}.iam.gserviceaccount.com"
  if ! describe_resource gcloud iam service-accounts describe "$email" --project="$PROJECT"; then
    gcloud iam service-accounts create "$account" \
      --display-name="${AR_REPO} deploy (${account})" --project="$PROJECT"
  fi
done

printf '== Deployer roles ==\n'
# run.admin is the only project-level grant, and only the deployer holds it.
gcloud projects add-iam-policy-binding "$PROJECT" \
  --member="serviceAccount:${DEPLOY_SA_EMAIL}" \
  --role=roles/run.admin --condition=None >/dev/null

gcloud artifacts repositories add-iam-policy-binding "$AR_REPO" \
  --location="$REGION" --project="$PROJECT" \
  --member="serviceAccount:${DEPLOY_SA_EMAIL}" \
  --role=roles/artifactregistry.writer >/dev/null

for account in "${RUNTIME_ACCOUNTS[@]}"; do
  gcloud iam service-accounts add-iam-policy-binding \
    "${account}@${PROJECT}.iam.gserviceaccount.com" \
    --project="$PROJECT" \
    --member="serviceAccount:${DEPLOY_SA_EMAIL}" \
    --role=roles/iam.serviceAccountUser >/dev/null
done

printf '== WIF binding: repository -> deployer ==\n'
gcloud iam service-accounts add-iam-policy-binding "$DEPLOY_SA_EMAIL" \
  --project="$PROJECT" --role=roles/iam.workloadIdentityUser \
  --member="principalSet://iam.googleapis.com/projects/${PROJECT_NUM}/locations/global/workloadIdentityPools/${POOL}/attribute.repository/${REPO}" >/dev/null

printf '== Secret Manager ==\n'
# Placeholders only. Real values arrive from GitHub Environments at deploy time,
# so no value passes through this script or an operator terminal.
SECRET_KEYS=({{SECRET_KEYS}})

for environment in dev prod integration; do
  runtime_email="${AR_REPO}-runtime-${environment}@${PROJECT}.iam.gserviceaccount.com"
  for key in "${SECRET_KEYS[@]}"; do
    secret="${key}-${environment}"
    if ! describe_resource gcloud secrets describe "$secret" --project="$PROJECT"; then
      printf '%s' 'placeholder-set-by-deploy-workflow' |
        gcloud secrets create "$secret" --replication-policy=automatic \
          --data-file=- --project="$PROJECT"
    fi
    gcloud secrets add-iam-policy-binding "$secret" --project="$PROJECT" \
      --member="serviceAccount:${DEPLOY_SA_EMAIL}" \
      --role=roles/secretmanager.secretVersionAdder >/dev/null
    gcloud secrets add-iam-policy-binding "$secret" --project="$PROJECT" \
      --member="serviceAccount:${runtime_email}" \
      --role=roles/secretmanager.secretAccessor >/dev/null
  done
done

printf '== Uploads buckets ==\n'
for environment in dev prod integration; do
  bucket="${AR_REPO}-${environment}-uploads"
  runtime_email="${AR_REPO}-runtime-${environment}@${PROJECT}.iam.gserviceaccount.com"
  bucket_region="$REGION"
  [[ "$environment" == integration ]] && bucket_region="$INTEGRATION_REGION"

  if describe_resource gcloud storage buckets describe "gs://${bucket}" \
    --project="$PROJECT" --raw \
    --format='value(projectNumber,location,iamConfiguration.uniformBucketLevelAccess.enabled)'; then
    read -r bucket_project bucket_location bucket_uniform <<<"$DESCRIBE_OUTPUT"
    [[ "$bucket_project" == "$PROJECT_NUM" ]] ||
      die "bucket $bucket belongs to project number $bucket_project, expected $PROJECT_NUM"
    [[ "$(lowercase "$bucket_location")" == "$(lowercase "$bucket_region")" ]] ||
      die "bucket $bucket is in $bucket_location, expected $bucket_region"
    [[ "$(lowercase "$bucket_uniform")" == 'true' ]] ||
      die "bucket $bucket does not have uniform bucket-level access"
  else
    gcloud storage buckets create "gs://${bucket}" \
      --location="$bucket_region" --uniform-bucket-level-access --project="$PROJECT"
  fi

  gcloud storage buckets add-iam-policy-binding "gs://${bucket}" \
    --project="$PROJECT" \
    --member="serviceAccount:${runtime_email}" \
    --role=roles/storage.objectAdmin >/dev/null
done

WIF_PROVIDER="projects/${PROJECT_NUM}/locations/global/workloadIdentityPools/${POOL}/providers/${PROVIDER}"

printf '\nBootstrap complete.\n\n'
printf 'Set these repository secrets (Settings -> Secrets and variables -> Actions):\n'
printf '  GCP_PROJECT_ID = %s\n' "$PROJECT"
printf '  WIF_PROVIDER   = %s\n\n' "$WIF_PROVIDER"
printf 'Runtime service accounts:\n'
for account in "${RUNTIME_ACCOUNTS[@]}"; do
  printf '  %s@%s.iam.gserviceaccount.com\n' "$account" "$PROJECT"
done
printf '\nAuthentication is WIF only. Do not create, download or store service-account keys.\n'

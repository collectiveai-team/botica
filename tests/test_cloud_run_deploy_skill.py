"""Invariants over the deployment workflows.

These exist because the skill's value is a dozen security invariants, and prose
does not fail CI. A template edit that drops one fails here instead of in a
production deploy. Every assertion names the invariant from the spec.
"""

from __future__ import annotations

import re
import subprocess

import pytest
from cloud_run_support import _extract_case_block, _job_block, _step_block, _template


def test_listener_never_receives_credentials():
    # Invariant 1 and 2: this file is loaded from the tagged commit's own tree,
    # so its author controls every line. It must be worth nothing to compromise.
    listener = _template("integration-tag.yml")
    assert "google-github-actions/auth" not in listener
    assert "${{ secrets." not in listener
    assert "environment:" not in listener


def test_listener_documents_that_it_is_not_a_boundary():
    # The comment is load-bearing: without it the next reader adds a check here
    # and believes it protects something.
    listener = _template("integration-tag.yml")
    assert "not a security control" in listener.lower() or "never a boundary" in listener.lower()


def test_validate_job_runs_before_any_credential():
    # Invariant 1: ordering is the boundary. deploy must depend on validate.
    workflow = _template("deploy-integration.yml")
    assert re.search(r"needs:\s*validate", workflow)
    validate_block = workflow.split("deploy:", 1)[0]
    assert "google-github-actions/auth" not in validate_block
    assert "environment:" not in validate_block
    assert "${{ secrets." not in validate_block


def test_validate_control_checkout_pins_the_trusted_branch():
    # Invariant 1, and the half of it that `test_validate_job_runs_before_any_
    # credential` cannot see. That test only proves the validate job holds no
    # credential; it says nothing about WHOSE code establishes the facts the
    # credentialed job then trusts.
    #
    # Mutating this checkout to `ref: ${{ inputs.review_tag }}` keeps the
    # validate job credential-free -- so every other invariant-1 assertion still
    # passes -- while handing the tag author scripts/review_tag.py itself.
    # Validation becomes self-attested: emit any head_sha, exit 0, and the
    # deploy job builds arbitrary code with WIF and the integration
    # Environment's secrets. The whole three-stage design falls to one token.
    workflow = _template("deploy-integration.yml")
    validate = _job_block(workflow, "validate")
    checkout = _step_block(validate, "Check out trusted control code")

    ref_lines = [line.strip() for line in checkout.splitlines() if line.strip().startswith("ref:")]
    assert ref_lines == ["ref: {{DEV_BRANCH}}"], (
        f"the control checkout must pin the trusted branch literally, got {ref_lines!r}"
    )

    # Nothing the tag author can influence may reach this step's inputs, in any
    # form -- not the ref, not a path, not a fetch spec.
    for expression in re.findall(r"\$\{\{[^}]*\}\}", checkout):
        assert "inputs." not in expression, expression
        assert "github.event" not in expression, expression

    assert validate.count("actions/checkout") == 1, (
        "an added checkout in the validate job could reintroduce untrusted code"
    )


def test_deploy_job_binds_the_integration_environment():
    # Invariant 1 and 7. `environment: prod` here ships green under every other
    # assertion in this file, and hands production Environment secrets to the one
    # job that builds and runs a PR author's tree.
    workflow = _template("deploy-integration.yml")
    deploy = _job_block(workflow, "deploy")

    environment_lines = [
        line.strip() for line in deploy.splitlines() if line.strip().startswith("environment:")
    ]
    assert environment_lines == ["environment: integration"], (
        f"the deploy job must bind exactly the integration environment, got {environment_lines!r}"
    )


def test_untrusted_head_checkout_drops_credentials():
    # Invariant 5: a persisted token in a checkout of attacker-controlled code is
    # a token in attacker-controlled code.
    #
    # Step-scoped on purpose. A whole-file `"persist-credentials: false" in
    # workflow` is satisfied by that line sitting on the TRUSTED control
    # checkout in the validate job, which is both useless here and wrong there
    # (the control checkout's token is what fetches the tag from a private repo).
    workflow = _template("deploy-integration.yml")
    deploy = _job_block(workflow, "deploy")
    checkout = _step_block(deploy, "Check out the validated pull request head")

    assert "persist-credentials: false" in checkout, (
        "the untrusted head checkout must drop the token"
    )
    # Without `path:`, the attacker's tree lands in the workspace root, next to
    # (and able to shadow) anything the job later runs.
    assert "path: source" in checkout, "the untrusted head must be checked out into a subdirectory"

    # M9: every checkout in the credentialed job is accounted for. A second,
    # rogue `actions/checkout` -- of the tag, of a fork, with credentials -- is
    # invisible to a test that only inspects the step it already knows about.
    assert deploy.count("actions/checkout") == 1, (
        "the deploy job must contain exactly one checkout, the validated head"
    )


def test_evidence_step_takes_the_review_tag_through_the_environment():
    # T6-M2: an expression interpolated into a `run:` body is substituted before
    # bash parses the line. Safe only for as long as review_tag.py's TAG_PATTERN
    # holds -- a safety argument in a different file that this workflow neither
    # cites nor enforces. The env var is quoted by bash instead.
    workflow = _template("deploy-integration.yml")
    deploy = _job_block(workflow, "deploy")
    step = _step_block(deploy, "Write the deployment evidence artifact")

    body = step.split("run: |", 1)
    assert len(body) == 2, "evidence step has no run block"
    assert "${{" not in body[1], (
        f"no GitHub expression may be interpolated into the run body: {body[1]}"
    )
    assert "REVIEW_TAG: ${{ needs.validate.outputs.review_tag }}" in body[0], (
        "the review tag must reach the script through the step's env block"
    )


def test_integration_concurrency_does_not_cancel():
    # Invariant 11: cancelling mid-reset leaves a half-migrated database.
    workflow = _template("deploy-integration.yml")
    assert "cancel-in-progress: false" in workflow


def test_reset_requires_both_flags():
    # Invariant 10, the two flags. The marker is asserted separately.
    workflow = _template("deploy-integration.yml")
    # Extract only the reset step to ensure flags are in the right place
    reset_step_start = workflow.find("- name: Reset, migrate and seed the integration database")
    assert reset_step_start != -1, "Reset step not found"
    reset_step_end = workflow.find("- name:", reset_step_start + 1)
    if reset_step_end == -1:
        reset_step = workflow[reset_step_start:]
    else:
        reset_step = workflow[reset_step_start:reset_step_end]
    assert "ALLOW_DATABASE_RESET=true" in reset_step, "ALLOW_DATABASE_RESET=true not in reset step"
    assert "DEPLOY_ENV=integration" in reset_step, "DEPLOY_ENV=integration not in reset step"


def test_validate_checkout_pins_specific_sha():
    # Invariant 3: the deploy job must check out the exact SHA that validate
    # selected, never re-resolve the tag (which could change between validate and
    # deploy, creating a force-push race). The checkout must use
    # needs.validate.outputs.head_sha, and inputs.review_tag must not appear
    # anywhere in the deploy job.
    workflow = _template("deploy-integration.yml")
    deploy_block = workflow.split("deploy:", 1)[1]

    # Verify inputs.review_tag is never used in the deploy job
    assert "inputs.review_tag" not in deploy_block, (
        "deploy job must not reference inputs.review_tag (would re-resolve the tag)"
    )

    # Extract the checkout step ("Check out the validated pull request head")
    # and verify its ref: line is exactly needs.validate.outputs.head_sha
    checkout_start = deploy_block.find("- name: Check out the validated pull request head")
    assert checkout_start != -1, "Checkout step not found in deploy job"

    # Find the next step boundary
    checkout_end = deploy_block.find("- name:", checkout_start + 1)
    if checkout_end == -1:
        checkout_step = deploy_block[checkout_start:]
    else:
        checkout_step = deploy_block[checkout_start:checkout_end]

    # Find and validate the ref: line within the checkout step
    for line in checkout_step.splitlines():
        if "ref:" in line:
            ref_line = line.strip()
            expected = "ref: ${{ needs.validate.outputs.head_sha }}"
            assert ref_line == expected, (
                f"checkout ref must be exactly '{expected}', got '{ref_line}'"
            )
            return

    raise AssertionError("ref: line not found in checkout step")


def test_marker_lives_outside_the_dropped_schema():
    # Invariant 10, the marker. A marker inside `public` is destroyed by the very
    # reset it is supposed to authorise, so the second run would find none.
    marker = _template("integration-marker.sql")
    assert "review_control" in marker
    assert "public." not in marker


def test_integration_images_are_tagged_by_pr_and_sha():
    # Invariant 9: a deployed revision must be traceable to one commit.
    #
    # Only the IMAGE tag is checked. Secret Manager references legitimately end
    # in `:latest` -- that is a secret version, not an image tag, and asserting
    # on a bare ":latest" would conflate the two.
    workflow = _template("deploy-integration.yml")
    # Find the IMAGE= assignment line specifically
    for line in workflow.splitlines():
        if 'IMAGE="' in line and "${AR_REGION}-docker.pkg.dev" in line:
            assert "integration-pr-" in line, f"integration-pr- not in IMAGE line: {line}"
            assert "${SHORT_SHA}" in line, f"SHORT_SHA not in IMAGE line: {line}"
            assert not line.rstrip().endswith(":latest"), f"IMAGE line ends in :latest: {line}"
            break
    else:
        raise AssertionError("IMAGE= assignment line not found")


def test_environment_derives_from_the_ref_not_an_input():
    # Invariant 3: `environment: ${{ inputs.environment }}` hands production
    # secrets to anyone with workflow_dispatch rights.
    #
    # `${{` is required in the match because the workflow_dispatch input is
    # itself NAMED `environment`, so its declaration line also strips to
    # "environment:" -- matching on the prefix alone flags a false positive.
    workflow = _template("deploy.yml")
    environment_lines = [
        line
        for line in workflow.splitlines()
        if line.strip().startswith("environment:") and "${{" in line
    ]
    assert environment_lines, "deploy.yml declares no job environment expression"
    for line in environment_lines:
        assert "github.ref_name" in line, line
        assert "inputs." not in line, line


def test_dispatch_input_is_cross_checked_against_the_ref():
    # An input may narrow what a ref permits; it may never widen it.
    workflow = _template("deploy.yml")
    assert "REQUESTED_ENVIRONMENT" in workflow
    assert "Deployments require" in workflow


def test_branch_images_are_tagged_by_sha():
    # Invariant 9. As above, only the image tag -- Secret Manager's `:latest`
    # version reference is a different thing and is correct.
    workflow = _template("deploy.yml")
    assert "/app:latest" not in workflow
    assert "rev-parse --short=12" in workflow


def test_case_statement_has_all_four_arms():
    # Invariant 3 (extended): The case statement must map each pattern to exactly
    # the correct environment value. A one-token edit changing `ENV=dev` to
    # `ENV=prod` in a dev arm would be catastrophic (dev deploy as prod) and must
    # be caught, not just by presence of patterns and count of assignments.
    workflow = _template("deploy.yml")

    # Extract the case statement
    case_anchor = 'case "${{ github.event_name }}:$GITHUB_REF_NAME:$REQUESTED_ENVIRONMENT" in'
    case_start = workflow.find(case_anchor)
    assert case_start != -1, "case statement anchor not found"

    case_end = workflow.find("esac", case_start)
    assert case_end != -1, "esac anchor not found after case"

    case_block = workflow[case_start : case_end + len("esac")]

    # Parse each arm and verify the mapping: pattern -> ENV value
    # Arms are separated by ;; with patterns before ) and assignment after )
    dev_patterns = [
        "push:{{DEV_BRANCH}}:|workflow_dispatch:{{DEV_BRANCH}}:dev",
    ]
    prod_patterns = [
        "push:{{PROD_BRANCH}}:|workflow_dispatch:{{PROD_BRANCH}}:prod",
    ]

    # Verify dev patterns map to ENV=dev
    for pattern in dev_patterns:
        pattern_line = pattern + ") ENV=dev"
        assert pattern_line in case_block, f"dev pattern {pattern} must assign ENV=dev"

    # Verify prod patterns map to ENV=prod
    for pattern in prod_patterns:
        pattern_line = pattern + ") ENV=prod"
        assert pattern_line in case_block, f"prod pattern {pattern} must assign ENV=prod"

    # Verify no extra arms assign ENV (only dev and prod should)
    env_lines = [line for line in case_block.splitlines() if "ENV=" in line]
    assert len(env_lines) == 2, f"Expected exactly 2 ENV assignments, got {len(env_lines)}"

    # The rejection arm must be present and only in the rejection
    assert "*) " in case_block or "*)" in case_block, "rejection arm missing"
    # Extract rejection arm content
    rejection_start = case_block.find("*)")
    rejection_end = case_block.find(";;", rejection_start)
    rejection_block = case_block[rejection_start:rejection_end]
    assert "exit 1" in rejection_block, "rejection must exit, not assign ENV"
    assert "ENV=" not in rejection_block, "rejection arm must not assign ENV"


def test_runtime_service_account_derives_from_env():
    # Invariant 8 (critical): dev and prod must have separate service accounts.
    # Hardcoding `runtime_sa=${AR_REPO}-runtime-prod` would run dev as prod.
    workflow = _template("deploy.yml")

    # Find the runtime_sa assignment
    sa_line_start = workflow.find('echo "runtime_sa=${AR_REPO}-runtime-${ENV}"')
    assert sa_line_start != -1, "runtime_sa assignment line not found (must interpolate $ENV)"

    # Verify it uses ${ENV}, not a literal
    sa_section = workflow[max(0, sa_line_start - 100) : sa_line_start + 100]
    assert "${ENV}" in sa_section, "runtime_sa must interpolate ${ENV}"
    assert "runtime-dev" not in sa_section, "runtime_sa must not hardcode 'dev'"
    assert "runtime-prod" not in sa_section, "runtime_sa must not hardcode 'prod'"


def test_resource_names_interpolate_env():
    # Invariant 13: The migrate job name and Cloud Run service name must derive
    # from $ENV to enforce environment isolation. Hardcoding one environment is
    # a critical vulnerability.
    workflow = _template("deploy.yml")

    # Migrate job must use ${ENV}
    migrate_start = workflow.find('gcloud run jobs deploy "migrate-${ENV}"')
    assert migrate_start != -1, "migrate job must use migrate-${ENV}, not a literal"

    # Service name must use ${ENV}
    service_start = workflow.find('gcloud run deploy "${AR_REPO}-${ENV}"')
    assert service_start != -1, "service must use ${AR_REPO}-${ENV}, not a literal"

    # Smoke test URL should use the service output, not a hardcoded env
    # (implicitly tested by the service name interpolation above)


def test_gcp_project_id_validation_present():
    # Invariant 14: GCP_PROJECT_ID must be validated to be a valid project ID
    # before any gcloud commands run.
    workflow = _template("deploy.yml")

    regex_pattern = '[[ "$GCP_PROJECT_ID" =~ ^[a-z0-9-]+$ ]]'
    validate_start = workflow.find(regex_pattern)
    assert validate_start != -1, "GCP_PROJECT_ID regex validation not found"

    # This validation must come before any gcloud command
    validate_section_anchor = "Validate deployment target"
    validate_section = workflow.find(validate_section_anchor)
    assert validate_section != -1, f"{validate_section_anchor} section anchor not found"

    first_gcloud = workflow.find("gcloud", validate_section)
    first_validate = workflow.find("GCP_PROJECT_ID", validate_section)
    assert first_validate != -1, "GCP_PROJECT_ID validation anchor not found"
    assert first_gcloud != -1, "gcloud command anchor not found"

    assert first_validate < first_gcloud, "validation must happen before gcloud"


def test_deploy_concurrency_never_cancels():
    # Invariant 11 (extended to all envs): Migrations run as a Cloud Run Job.
    # Cancelling the runner does not cancel the GCP-side execution, so a second
    # push would start a second migration alongside the first, corrupting the database.
    workflow = _template("deploy.yml")
    assert "cancel-in-progress: false" in workflow, "cancel-in-progress must be false"
    assert "cancel-in-progress: true" not in workflow, "cancel-in-progress must not be true"


def test_secret_placeholders_not_substituted():
    # Invariant 15: Secret placeholders must remain as {{...}} and never be
    # replaced with literal values. A dev deploy could be pointed at prod
    # secrets otherwise. Extra secrets cannot be smuggled in as appended values.
    workflow = _template("deploy.yml")

    # All four secret placeholders must be present exactly
    assert "{{MIGRATE_SECRETS}}" in workflow, "MIGRATE_SECRETS placeholder missing"
    assert "{{RUNTIME_SECRETS}}" in workflow, "RUNTIME_SECRETS placeholder missing"
    assert "{{SECRET_ENV_BLOCK}}" in workflow, "SECRET_ENV_BLOCK placeholder missing"
    assert "{{SYNC_SECRET_CALLS}}" in workflow, "SYNC_SECRET_CALLS placeholder missing"

    # Each --set-secrets line must use exactly the placeholder with no appends
    migrate_found = False
    runtime_found = False
    for line in workflow.splitlines():
        if "--set-secrets" in line:
            if "MIGRATE_SECRETS" in line:
                migrate_found = True
                # Must be exactly this value, not with appended secrets
                assert '--set-secrets "{{MIGRATE_SECRETS}}"' in line, (
                    f"MIGRATE_SECRETS must be exact placeholder: {line}"
                )
            if "RUNTIME_SECRETS" in line:
                runtime_found = True
                # Must be exactly this value, not with appended secrets
                assert '--set-secrets "{{RUNTIME_SECRETS}}"' in line, (
                    f"RUNTIME_SECRETS must be exact placeholder: {line}"
                )

    assert migrate_found, "MIGRATE_SECRETS --set-secrets line not found"
    assert runtime_found, "RUNTIME_SECRETS --set-secrets line not found"

    # SECRET_ENV_BLOCK must appear exactly
    sync_start = workflow.find("env:")
    assert sync_start != -1, "env section not found"

    sync_start = workflow.find("{{SECRET_ENV_BLOCK}}", sync_start)
    assert sync_start != -1, "SECRET_ENV_BLOCK not found"

    # SYNC_SECRET_CALLS must appear exactly
    sync_start = workflow.find("sync_secret()")
    assert sync_start != -1, "sync_secret function not found"

    sync_end = workflow.find("{{SYNC_SECRET_CALLS}}", sync_start)
    assert sync_end != -1, "SYNC_SECRET_CALLS placeholder not found"


@pytest.mark.parametrize(
    ("event_name", "ref_name", "input_env", "expected_env"),
    [
        ("push", "dev", "", "dev"),
        ("push", "main", "", "prod"),
        ("workflow_dispatch", "dev", "dev", "dev"),
        ("workflow_dispatch", "main", "prod", "prod"),
        ("workflow_dispatch", "dev", "prod", "rejected"),
        ("workflow_dispatch", "main", "dev", "rejected"),
        ("push", "feature/x", "", "rejected"),
        ("workflow_dispatch", "feature/x", "prod", "rejected"),
        ("push", "dev", "prod", "rejected"),
    ],
)
def test_case_statement_resolves_correctly(
    event_name: str, ref_name: str, input_env: str, expected_env: str
) -> None:
    # Invariant 3 (behavioral): The case statement must correctly route all
    # legitimate and illegitimate inputs to the right branch. Arm reordering,
    # unreachable arms, swapped bindings, and mismatched dispatch inputs must
    # all be caught by a test that actually executes the logic.
    case_block = _extract_case_block()

    # Substitute placeholders with concrete values
    case_code = case_block.replace("{{DEV_BRANCH}}", "dev")
    case_code = case_code.replace("{{PROD_BRANCH}}", "main")
    # Replace GitHub Actions template syntax with actual values
    case_code = case_code.replace("${{ github.event_name }}", event_name)
    case_code = case_code.replace("$GITHUB_REF_NAME", ref_name)
    case_code = case_code.replace("$REQUESTED_ENVIRONMENT", input_env)

    # Build a bash script that runs the case block and echoes ENV
    bash_script = f"""
set -euo pipefail
{case_code}
echo "$ENV"
"""

    result = subprocess.run(
        ["bash", "-c", bash_script],
        capture_output=True,
        text=True,
        timeout=5,
        check=False,
    )

    if expected_env == "rejected":
        # Expected to reject (exit non-zero)
        assert result.returncode != 0, (
            f"Expected rejection for event={event_name}, ref={ref_name}, "
            f"input={input_env}, but case block succeeded with ENV={result.stdout.strip()}"
        )
    else:
        # Expected to succeed with specific ENV value
        assert result.returncode == 0, (
            f"Case block failed for event={event_name}, ref={ref_name}, "
            f"input={input_env}: {result.stderr}"
        )
        env_value = result.stdout.strip()
        assert env_value == expected_env, f"Expected ENV={expected_env}, got ENV={env_value}"

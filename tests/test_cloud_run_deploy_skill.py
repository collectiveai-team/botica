"""Static invariant tests over the shipped templates.

These exist because the skill's value is a dozen security invariants, and prose
does not fail CI. A template edit that drops one fails here instead of in a
production deploy. Every assertion names the invariant from the spec.
"""

from __future__ import annotations

import re
import subprocess
import tempfile
from pathlib import Path

import pytest

TEMPLATES = (
    Path(__file__).resolve().parent.parent / "skills/delivery/cloud-run-continuous-deploy/templates"
)


def _template(name: str) -> str:
    return (TEMPLATES / name).read_text(encoding="utf-8")


def _extract_describe_resource() -> str:
    """Extract the describe_resource function from bootstrap-gcp.sh."""
    script = _template("bootstrap-gcp.sh")
    lines = script.splitlines(keepends=True)

    start_idx = None
    for i, line in enumerate(lines):
        if line.strip().startswith("describe_resource() {"):
            start_idx = i
            break

    assert start_idx is not None, "describe_resource function not found"

    # Find matching closing brace at column 0
    end_idx = None
    for i in range(start_idx + 1, len(lines)):
        if lines[i].rstrip() == "}":
            end_idx = i
            break

    assert end_idx is not None, "describe_resource closing brace not found"

    return "".join(lines[start_idx : end_idx + 1])


def _extract_case_block() -> str:
    """Extract the case block from deploy.yml validation step."""
    workflow = _template("deploy.yml")
    lines = workflow.splitlines(keepends=True)

    start_idx = None
    for i, line in enumerate(lines):
        if 'case "${{ github.event_name }}:$GITHUB_REF_NAME' in line:
            start_idx = i
            break

    assert start_idx is not None, "case statement anchor not found"

    # Find the matching esac
    end_idx = None
    for i in range(start_idx, len(lines)):
        if lines[i].strip() == "esac":
            end_idx = i
            break

    assert end_idx is not None, "esac anchor not found"

    return "".join(lines[start_idx : end_idx + 1])


def _job_block(workflow: str, job: str) -> str:
    """Slice one top-level job out of a workflow.

    Whole-file substring assertions are the root cause of several findings in
    this suite's review history: a security line asserted against the whole file
    can be satisfied by the wrong job. Callers assert against a slice instead.
    Missing anchors raise rather than returning an empty slice, because an empty
    slice makes every `not in` assertion vacuously true.
    """
    lines = workflow.splitlines(keepends=True)
    start = None
    for i, line in enumerate(lines):
        if line.rstrip("\n") == f"  {job}:":
            start = i
            break
    assert start is not None, f"job '  {job}:' not found in workflow"

    end = len(lines)
    for i in range(start + 1, len(lines)):
        if re.fullmatch(r"  [A-Za-z_][A-Za-z0-9_-]*:", lines[i].rstrip("\n")):
            end = i
            break
    return "".join(lines[start:end])


def _step_block(block: str, step_name: str) -> str:
    """Slice one `- name: <step_name>` step out of an already-sliced job."""
    marker = f"- name: {step_name}"
    start = block.find(marker)
    assert start != -1, f"step '{marker}' not found in the given block"

    end = block.find("- name:", start + len(marker))
    return block[start:] if end == -1 else block[start:end]


# Only templates that exist as of this task. Tasks 6 and 7 append to this list
# as they add templates, so every commit leaves the suite green.
ALL_TEMPLATE_NAMES = [
    "bootstrap-gcp.sh",
    "review_tag.py",
    "sync-github-secrets.sh",
]

ALL_TEMPLATE_NAMES += ["integration-tag.yml", "deploy-integration.yml"]

ALL_TEMPLATE_NAMES += ["deploy.yml"]

ALL_TEMPLATE_NAMES += ["nginx.conf", "entrypoint.sh", "Dockerfile.combined"]


@pytest.mark.parametrize("name", ALL_TEMPLATE_NAMES)
def test_no_template_creates_a_service_account_key(name: str):
    # Invariant 12: WIF only. A key file in a repo or on a laptop is a permanent,
    # unrevoked credential the moment it leaks.
    assert "service-accounts keys create" not in _template(name)


def test_wif_condition_pins_repository_and_branch_refs():
    # Invariant 4: widening the condition to tag refs hands a federated token to
    # anyone who can push a tag.
    script = _template("bootstrap-gcp.sh")
    assert "assertion.repository ==" in script
    assert "refs/heads/" in script
    assert "refs/tags/" not in script


def test_describe_resource_aborts_on_empty_error():
    # Invariant 6: fail-closed on unrecognized errors. The function must abort
    # (call exit) when describe fails with unrecognized output — execution must not continue.
    describe_func = _extract_describe_resource()

    bash_code = f"""
DESCRIBE_OUTPUT=""
{describe_func}
test_stub() {{ return 1; }}
describe_resource test_stub 2>/dev/null && rc=0 || rc=$?
echo "CONTINUED rc=$rc"
"""

    with tempfile.NamedTemporaryFile(mode="w", suffix=".sh", delete=False) as f:
        f.write(bash_code)
        f.flush()
        temp_path = f.name

    try:
        result = subprocess.run(
            ["bash", temp_path], capture_output=True, text=True, timeout=5, check=False
        )
        # Abort means the script never reaches echo "CONTINUED"
        assert "CONTINUED" not in result.stdout, (
            f"describe_resource should abort, not return; stdout: {result.stdout}"
        )
        assert result.returncode != 0, f"expected non-zero exit from abort, got {result.returncode}"
    finally:
        Path(temp_path).unlink()


def test_describe_resource_aborts_on_permission_denied():
    # Invariant 6: fail-closed on permission errors — these cannot be mistaken
    # for resource absence. Execution must not continue.
    describe_func = _extract_describe_resource()

    bash_code = f"""
DESCRIBE_OUTPUT=""
{describe_func}
test_stub() {{ echo "ERROR: permission denied" >&2; return 1; }}
describe_resource test_stub 2>/dev/null && rc=0 || rc=$?
echo "CONTINUED rc=$rc"
"""

    with tempfile.NamedTemporaryFile(mode="w", suffix=".sh", delete=False) as f:
        f.write(bash_code)
        f.flush()
        temp_path = f.name

    try:
        result = subprocess.run(
            ["bash", temp_path], capture_output=True, text=True, timeout=5, check=False
        )
        # Abort means the script never reaches echo "CONTINUED"
        assert "CONTINUED" not in result.stdout, (
            f"describe_resource should abort on permission error; stdout: {result.stdout}"
        )
        assert result.returncode != 0, f"expected non-zero exit from abort, got {result.returncode}"
    finally:
        Path(temp_path).unlink()


def test_describe_resource_aborts_on_invalid_grant():
    # Invariant 6 + Important 2: fail-closed on auth failures. The string
    # "invalid_grant: Not found or already used" contains "not found" but is an
    # authentication failure, not resource absence. Without the auth-failure guard,
    # this would wrongly return 1 (reporting absent). Must abort instead.
    describe_func = _extract_describe_resource()

    bash_code = f"""
DESCRIBE_OUTPUT=""
{describe_func}
test_stub() {{ echo "invalid_grant: Not found or already used" >&2; return 1; }}
describe_resource test_stub 2>/dev/null && rc=0 || rc=$?
echo "CONTINUED rc=$rc"
"""

    with tempfile.NamedTemporaryFile(mode="w", suffix=".sh", delete=False) as f:
        f.write(bash_code)
        f.flush()
        temp_path = f.name

    try:
        result = subprocess.run(
            ["bash", temp_path], capture_output=True, text=True, timeout=5, check=False
        )
        # Without auth guard, "invalid_grant: Not found..." matches the not-found branch
        # and returns 1, causing "CONTINUED rc=1" to appear. The auth guard must abort.
        assert "CONTINUED" not in result.stdout, (
            f"auth failure guard missing; invalid_grant wrongly classified as absent; "
            f"stdout: {result.stdout}"
        )
        assert result.returncode != 0, f"expected non-zero exit from abort, got {result.returncode}"
    finally:
        Path(temp_path).unlink()


def test_describe_resource_returns_absent_on_genuine_not_found():
    # Invariant 6: recognize genuine resource-not-found and report it as absent
    # (return 1) without aborting. Execution MUST continue so the bootstrap can
    # proceed to create the resource. This also catches if someone changes the
    # absent return to exit.
    describe_func = _extract_describe_resource()

    bash_code = f"""
DESCRIBE_OUTPUT=""
{describe_func}
test_stub() {{ echo "ERROR: Resource 'x' was not found" >&2; return 1; }}
describe_resource test_stub 2>/dev/null && rc=0 || rc=$?
echo "CONTINUED rc=$rc"
"""

    with tempfile.NamedTemporaryFile(mode="w", suffix=".sh", delete=False) as f:
        f.write(bash_code)
        f.flush()
        temp_path = f.name

    try:
        result = subprocess.run(
            ["bash", temp_path], capture_output=True, text=True, timeout=5, check=False
        )
        # For genuine not-found, describe_resource must return 1 (not exit).
        # This means execution continues and we see "CONTINUED rc=1".
        assert "CONTINUED rc=1" in result.stdout, (
            f"describe_resource must return 1 for genuine not-found, not abort; "
            f"stdout: {result.stdout}"
        )
        # The script exits with 0 because the echo succeeded
        assert result.returncode == 0, f"expected exit 0 (echo succeeded), got {result.returncode}"
    finally:
        Path(temp_path).unlink()


def test_bootstrap_grants_no_project_level_role_to_a_runtime_account():
    # Invariant 8: runtime identities get resource-scoped bindings only.
    #
    # The member is on a continuation line, not on the line naming the command,
    # so the whole invocation has to be reassembled before it can be inspected.
    # Checking only the command line would assert nothing.
    script = _template("bootstrap-gcp.sh")
    lines = script.splitlines()
    invocations = []
    for index, line in enumerate(lines):
        if "projects add-iam-policy-binding" not in line:
            continue
        invocation = [line]
        cursor = index
        while lines[cursor].rstrip().endswith("\\") and cursor + 1 < len(lines):
            cursor += 1
            invocation.append(lines[cursor])
        invocations.append("\n".join(invocation))

    assert invocations, "bootstrap-gcp.sh grants no project-level role at all"
    for invocation in invocations:
        assert "RUNTIME" not in invocation, invocation
        assert "runtime" not in invocation, invocation
        assert "DEPLOY_SA_EMAIL" in invocation, invocation


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


def test_entrypoint_uses_bash_for_wait_n():
    # `wait -n` is a bashism. Under dash it fails at runtime, inside a container,
    # on the first deploy -- the most expensive place to discover it.
    entrypoint = _template("entrypoint.sh")
    assert entrypoint.startswith("#!/usr/bin/env bash")
    assert "wait -n" in entrypoint


def test_entrypoint_exits_when_any_child_exits():
    # A supervisor that restarts children in place keeps a broken revision
    # serving traffic; Cloud Run must be allowed to replace it.
    entrypoint = _template("entrypoint.sh")
    assert "trap" in entrypoint
    assert re.search(r"wait -n\s*\n\s*exit", entrypoint)


def test_envsubst_is_restricted_to_port():
    # Unrestricted envsubst eats nginx's own $host and $remote_addr, and the
    # resulting config is silently wrong rather than broken.
    entrypoint = _template("entrypoint.sh")
    assert "envsubst '${PORT}'" in entrypoint


def test_nginx_listens_on_the_cloud_run_port():
    conf = _template("nginx.conf")
    assert "listen       ${PORT}" in conf or "listen ${PORT}" in conf


def test_nginx_forwards_the_proxy_headers():
    # Without X-Forwarded-Proto the apps generate http:// URLs behind Cloud Run's
    # TLS terminator, which breaks redirects and cookies.
    conf = _template("nginx.conf")
    assert "X-Forwarded-Proto" in conf
    assert "X-Forwarded-For" in conf


def test_envsubst_restriction_actually_works():
    # Behavioral test: extract the actual envsubst command from entrypoint.sh,
    # run it on nginx.conf with PORT=9999, and verify the restriction is effective.
    import shutil

    if shutil.which("envsubst") is None:
        pytest.skip("envsubst not installed")

    # Extract the envsubst command from entrypoint.sh
    entrypoint = _template("entrypoint.sh")
    lines = entrypoint.split("\n")

    # Find the line containing envsubst
    envsubst_idx = None
    for i, line in enumerate(lines):
        if "envsubst" in line and not line.strip().startswith("#"):
            envsubst_idx = i
            break

    assert envsubst_idx is not None, "envsubst command not found in entrypoint.sh"

    # Collect the full command (handles backslash continuation)
    command_lines = [lines[envsubst_idx]]
    idx = envsubst_idx
    while idx < len(lines) - 1 and lines[idx].rstrip().endswith("\\"):
        idx += 1
        command_lines.append(lines[idx])

    full_command = " ".join(line.rstrip().rstrip("\\").strip() for line in command_lines)

    # Extract the envsubst portion: envsubst '${PORT}' or similar
    envsubst_match = re.search(r"(envsubst\s+'[^']*')", full_command)
    assert envsubst_match is not None, f"could not extract envsubst from: {full_command}"
    envsubst_cmd = envsubst_match.group(1)

    # Load nginx.conf template
    conf = _template("nginx.conf")
    with tempfile.NamedTemporaryFile(mode="w", suffix=".conf", delete=False) as f:
        f.write(conf)
        f.flush()
        temp_path = f.name

    try:
        # Run the actual envsubst command from entrypoint.sh
        result = subprocess.run(
            ["bash", "-c", f"{envsubst_cmd} < {temp_path}"],
            capture_output=True,
            text=True,
            timeout=5,
            env={"PORT": "9999"},
            check=True,
        )

        output = result.stdout
        # PORT should be substituted in the listen directive
        assert "listen       9999" in output or "listen 9999" in output, (
            f"PORT=9999 not substituted in listen directive. Output: {output[:200]}"
        )
        # nginx variables should still be present (NOT substituted)
        assert "$host" in output, "$host was consumed by restricted envsubst"
        assert "$remote_addr" in output, "$remote_addr was consumed by restricted envsubst"
        assert "$proxy_add_x_forwarded_for" in output, "$proxy_add_x_forwarded_for was consumed"

        # Now run unrestricted envsubst to prove the restriction matters
        result_unrestricted = subprocess.run(
            ["bash", "-c", f"envsubst < {temp_path}"],
            capture_output=True,
            text=True,
            timeout=5,
            env={"PORT": "9999"},
            check=True,
        )

        unrestricted_output = result_unrestricted.stdout
        # Unrestricted envsubst should destroy nginx's variables (they're undefined)
        assert "$host" not in unrestricted_output, (
            "unrestricted envsubst should destroy $host, but it survived"
        )
        assert "$remote_addr" not in unrestricted_output, (
            "unrestricted envsubst should destroy $remote_addr, but it survived"
        )

    finally:
        Path(temp_path).unlink()


def test_dockerfile_installs_bash_for_wait_n():
    # bash is required for wait -n in entrypoint.sh. Under dash or sh, the
    # container fails at runtime on the first deploy (most expensive discovery).
    dockerfile = _template("Dockerfile.combined")
    assert "bash" in dockerfile, "bash must be installed for wait -n in entrypoint.sh"
    # Verify it's in the apt-get install line, not just mentioned in a comment
    lines = dockerfile.split("\n")
    for line in lines:
        if "apt-get install" in line:
            install_block = []
            idx = lines.index(line)
            # Collect the complete multi-line install command (handles backslash continuation)
            while idx < len(lines):
                install_block.append(lines[idx])
                if not lines[idx].rstrip().endswith("\\"):
                    break
                idx += 1
            install_text = " ".join(install_block)
            assert "bash" in install_text, (
                "bash must be in apt-get install command, not just mentioned elsewhere"
            )
            return
    raise AssertionError("apt-get install line not found in Dockerfile.combined")


def test_dockerfile_installs_gettext_base_for_envsubst():
    # gettext-base provides envsubst, which entrypoint.sh uses to render nginx.conf.
    # Removing it while leaving bash installed fails at container start with
    # "command not found: envsubst" — the most expensive discovery.
    dockerfile = _template("Dockerfile.combined")
    assert "gettext-base" in dockerfile, "gettext-base must be installed for envsubst"
    # Verify it's in the apt-get install line, not just mentioned in a comment
    lines = dockerfile.split("\n")
    for line in lines:
        if "apt-get install" in line:
            install_block = []
            idx = lines.index(line)
            # Collect the complete multi-line install command (handles backslash continuation)
            while idx < len(lines):
                install_block.append(lines[idx])
                if not lines[idx].rstrip().endswith("\\"):
                    break
                idx += 1
            install_text = " ".join(install_block)
            assert "gettext-base" in install_text, (
                "gettext-base must be in apt-get install command, not just mentioned elsewhere"
            )
            return
    raise AssertionError("apt-get install line not found in Dockerfile.combined")


def test_wait_n_returns_on_first_child():
    # Behavioral test: Extract the entrypoint structure and verify wait -n returns
    # on first child death, not blocking for all children. Derived directly from
    # the shipped template, not a hand-written copy.
    entrypoint = _template("entrypoint.sh")

    # Transform the template into a runnable script
    runnable = entrypoint
    # Substitute the two commands: one exits quickly (exit code 7), one sleeps 30s
    runnable = runnable.replace("{{API_COMMAND}}", "bash -c 'sleep 0.2; exit 7'")
    runnable = runnable.replace("{{WEB_COMMAND}}", "sleep 30")
    # Replace envsubst block with a stub (we don't need nginx for this test)
    # The envsubst block is multi-line with backslash continuation
    lines = runnable.split("\n")
    new_lines = []
    skip_until_complete = False
    for _i, line in enumerate(lines):
        if skip_until_complete:
            if not line.rstrip().endswith("\\"):
                skip_until_complete = False
            continue
        if "envsubst" in line and not line.strip().startswith("#"):
            # Skip this line and any continuation lines
            skip_until_complete = line.rstrip().endswith("\\")
            new_lines.append("# envsubst replaced with stub for test")
            continue
        # Replace nginx line with a stub (it doesn't need to run)
        if "nginx -g 'daemon off;'" in line:
            new_lines.append("# nginx stub for test")
            continue
        new_lines.append(line)
    runnable = "\n".join(new_lines)

    with tempfile.NamedTemporaryFile(mode="w", suffix=".sh", delete=False) as f:
        f.write(runnable)
        f.flush()
        temp_path = f.name

    try:
        # wait -n should return in ~0.2s when API_COMMAND exits
        # plain wait would block for 30s and exceed the timeout
        # Use 10s timeout with 30s sleep to get a clear discrimination
        try:
            subprocess.run(
                ["bash", temp_path],
                timeout=10,
                check=False,
                start_new_session=True,  # Isolate process group
            )
            # If we reach here, wait -n returned promptly
            # The exit code varies due to signal handling but that's OK - we're testing
            # that the script returns quickly, not the exact exit code
        except subprocess.TimeoutExpired as e:
            raise AssertionError(
                "wait -n did not return; script blocked past timeout (mutated to plain wait?)"
            ) from e

    finally:
        Path(temp_path).unlink()

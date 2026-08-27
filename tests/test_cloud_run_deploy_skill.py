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
    Path(__file__).resolve().parent.parent
    / "skills/delivery/cloud-run-continuous-deploy/templates"
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


# Only templates that exist as of this task. Tasks 6 and 7 append to this list
# as they add templates, so every commit leaves the suite green.
ALL_TEMPLATE_NAMES = [
    "bootstrap-gcp.sh",
    "review_tag.py",
    "sync-github-secrets.sh",
]

ALL_TEMPLATE_NAMES += ["integration-tag.yml", "deploy-integration.yml"]

ALL_TEMPLATE_NAMES += ["deploy.yml"]


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


def test_untrusted_head_checkout_drops_credentials():
    # Invariant 5: a persisted token in a checkout of attacker-controlled code is
    # a token in attacker-controlled code.
    workflow = _template("deploy-integration.yml")
    assert "persist-credentials: false" in workflow


def test_integration_concurrency_does_not_cancel():
    # Invariant 11: cancelling mid-reset leaves a half-migrated database.
    workflow = _template("deploy-integration.yml")
    assert "cancel-in-progress: false" in workflow


def test_reset_requires_both_flags():
    # Invariant 10, the two flags. The marker is asserted separately.
    workflow = _template("deploy-integration.yml")
    # Extract only the reset step to ensure flags are in the right place
    reset_step_start = workflow.find('- name: Reset, migrate and seed the integration database')
    assert reset_step_start != -1, "Reset step not found"
    reset_step_end = workflow.find('- name:', reset_step_start + 1)
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
        if 'IMAGE="' in line and '${AR_REGION}-docker.pkg.dev' in line:
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

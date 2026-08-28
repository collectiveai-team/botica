"""Invariants over the GCP bootstrap script.

These exist because the skill's value is a dozen security invariants, and prose
does not fail CI. A template edit that drops one fails here instead of in a
production deploy. Every assertion names the invariant from the spec.
"""

from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path

import pytest
from cloud_run_support import ALL_TEMPLATE_NAMES, _extract_describe_resource, _template


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

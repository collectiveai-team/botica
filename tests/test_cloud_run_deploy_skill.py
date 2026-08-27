"""Static invariant tests over the shipped templates.

These exist because the skill's value is a dozen security invariants, and prose
does not fail CI. A template edit that drops one fails here instead of in a
production deploy. Every assertion names the invariant from the spec.
"""

from __future__ import annotations

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
    # when describe fails with unrecognized output.
    describe_func = _extract_describe_resource()

    bash_code = f"""
set -euo pipefail
DESCRIBE_OUTPUT=""
{describe_func}
test_stub() {{ return 1; }}
describe_resource test_stub 2>/dev/null || exit_code=$?
exit "${{exit_code:-0}}"
"""

    with tempfile.NamedTemporaryFile(mode="w", suffix=".sh", delete=False) as f:
        f.write(bash_code)
        f.flush()
        temp_path = f.name

    try:
        result = subprocess.run(
            ["bash", temp_path], capture_output=True, text=True, timeout=5, check=False
        )
        assert result.returncode == 1, f"expected exit 1, got {result.returncode}"
    finally:
        Path(temp_path).unlink()


def test_describe_resource_aborts_on_permission_denied():
    # Invariant 6: fail-closed on permission errors — these cannot be mistaken
    # for resource absence.
    describe_func = _extract_describe_resource()

    bash_code = f"""
set -euo pipefail
DESCRIBE_OUTPUT=""
{describe_func}
test_stub() {{ echo "ERROR: permission denied" >&2; return 1; }}
describe_resource test_stub 2>/dev/null || exit_code=$?
exit "${{exit_code:-0}}"
"""

    with tempfile.NamedTemporaryFile(mode="w", suffix=".sh", delete=False) as f:
        f.write(bash_code)
        f.flush()
        temp_path = f.name

    try:
        result = subprocess.run(
            ["bash", temp_path], capture_output=True, text=True, timeout=5, check=False
        )
        assert result.returncode == 1, f"expected exit 1, got {result.returncode}"
    finally:
        Path(temp_path).unlink()


def test_describe_resource_aborts_on_invalid_grant():
    # Invariant 6 + Important 2: fail-closed on auth failures. The string
    # "invalid_grant: Not found or already used" contains "not found" but is an
    # authentication failure, not resource absence. This guards the regression
    # where auth errors were misclassified as absent.
    describe_func = _extract_describe_resource()

    bash_code = f"""
set -euo pipefail
DESCRIBE_OUTPUT=""
{describe_func}
test_stub() {{ echo "invalid_grant: Not found or already used" >&2; return 1; }}
describe_resource test_stub 2>/dev/null || exit_code=$?
exit "${{exit_code:-0}}"
"""

    with tempfile.NamedTemporaryFile(mode="w", suffix=".sh", delete=False) as f:
        f.write(bash_code)
        f.flush()
        temp_path = f.name

    try:
        result = subprocess.run(
            ["bash", temp_path], capture_output=True, text=True, timeout=5, check=False
        )
        assert result.returncode == 1, f"expected exit 1, got {result.returncode}"
    finally:
        Path(temp_path).unlink()


def test_describe_resource_returns_absent_on_genuine_not_found():
    # Invariant 6: recognize genuine resource-not-found and report it as absent
    # (return 1) without aborting (not exiting the whole script).
    describe_func = _extract_describe_resource()

    bash_code = f"""
set -euo pipefail
DESCRIBE_OUTPUT=""
{describe_func}
test_stub() {{ echo "ERROR: Resource 'x' was not found" >&2; return 1; }}
describe_resource test_stub 2>/dev/null || exit_code=$?
# describe_resource returned (did not call exit), so we get its return value
exit "${{exit_code:-0}}"
"""

    with tempfile.NamedTemporaryFile(mode="w", suffix=".sh", delete=False) as f:
        f.write(bash_code)
        f.flush()
        temp_path = f.name

    try:
        result = subprocess.run(
            ["bash", temp_path], capture_output=True, text=True, timeout=5, check=False
        )
        assert (
            result.returncode == 1
        ), f"expected return 1 (not abort), got {result.returncode}"
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

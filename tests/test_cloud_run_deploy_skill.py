"""Static invariant tests over the shipped templates.

These exist because the skill's value is a dozen security invariants, and prose
does not fail CI. A template edit that drops one fails here instead of in a
production deploy. Every assertion names the invariant from the spec.
"""

from __future__ import annotations

from pathlib import Path

import pytest

TEMPLATES = (
    Path(__file__).resolve().parent.parent
    / "skills/delivery/cloud-run-continuous-deploy/templates"
)


def _template(name: str) -> str:
    return (TEMPLATES / name).read_text(encoding="utf-8")


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


def test_bootstrap_fails_closed_on_ambiguous_inspection():
    # Invariant 6: treating an ambiguous describe error as "absent" is how a
    # bootstrap recreates or resets a live resource.
    script = _template("bootstrap-gcp.sh")
    assert "refusing to treat it as absent" in script


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

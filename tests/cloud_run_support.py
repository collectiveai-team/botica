"""Shared helpers for the cloud-run-continuous-deploy invariant suites.

The suites live in three files because the combined module outgrew the
file-size guard (CES-71). They share these helpers rather than each carrying a
copy, which would also trip the duplication gate (CES-118).

The slicing helpers all raise on a missing anchor instead of returning an empty
slice: an empty slice makes every `not in` assertion vacuously true, which is
the exact defect several of these tests were written to close.
"""

from __future__ import annotations

import re
from pathlib import Path

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

"""Label changes invalidate stale risk classifications."""

import importlib.util
import subprocess
import sys
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "review_labels", Path(__file__).resolve().parents[1] / "scripts" / "review_labels.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
labels = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = labels
_SPEC.loader.exec_module(labels)


def test_incomplete_risk_removes_low_label_without_requesting_human_review():
    report = {"status": "incomplete", "effective_tier": None}
    result = labels.label_decision(report, ["risk:low", "area:periphery", "bug"])
    assert result.remove == ["area:periphery", "risk:low"]
    assert result.add == ["risk:incomplete"]


def test_complete_risk_replaces_labels_from_both_assessments():
    report = {
        "status": "complete",
        "effective_tier": "high",
        "branch": {"comparisons": [{"areas": ["critical"]}]},
        "integration": {"comparisons": [{"areas": ["core"]}]},
    }
    result = labels.label_decision(report, ["risk:low", "area:periphery"])
    assert result.add == ["area:core", "area:critical", "risk:high"]
    assert result.remove == ["area:periphery", "risk:low"]


def test_workflow_invalidates_before_checkout_build_or_fetch_and_checks_both_shas():
    workflow = (
        Path(__file__).resolve().parents[1] / ".github/workflows/review-labels.yml"
    ).read_text()
    invalidate = workflow.index("Invalidate previous classification")
    for operation in ["uses: actions/checkout", "cargo build", "git -c core.hooksPath"]:
        assert invalidate < workflow.index(operation)
    assert "ready_for_review, edited" in workflow
    assert "branches: [main]" in workflow
    assert 'test "$current" = "$BASE_SHA:$HEAD_SHA" || exit 0' in workflow
    assert 'if test "$current" != "$BASE_SHA:$HEAD_SHA"' in workflow
    assert "gh api --paginate" in workflow


def _step(name: str) -> str:
    workflow = (
        Path(__file__).resolve().parents[1] / ".github/workflows/review-labels.yml"
    ).read_text()
    section = workflow.split(f"- name: {name}\n", 1)[1]
    body = section.split("        run: |\n", 1)[1]
    lines = []
    for line in body.splitlines():
        if line.strip() and not line.startswith("          "):
            break
        lines.append(line[10:])
    return "\n".join(lines)


def test_query_failure_stops_before_publishing_incomplete_as_success():
    fake = """gh() {
      if test "$1 $2" = "pr view"; then return 42; fi
      echo MUTATION
    }
    PR_NUMBER=1
    """
    result = subprocess.run(  # botica: ignore[test-no-subprocess] -- trusted workflow, fake gh
        [
            "bash",
            "-e",
            "-o",
            "pipefail",
            "-c",
            fake + _step("Invalidate previous classification before fallible work"),
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 42
    assert "MUTATION" not in result.stdout


def test_destination_invalidation_attempts_remaining_prs_after_one_failure():
    fake = """gh() {
      if test "$1" = api; then printf '1\\n2\\n'; return; fi
      if test "$1 $2 $3" = "pr view 1"; then return 42; fi
      if test "$1 $2" = "pr view"; then echo risk:low; return; fi
      echo "MUTATION $*"
    }
    GH_REPO=example/repo
    """
    result = subprocess.run(  # botica: ignore[test-no-subprocess] -- trusted workflow, fake gh
        [
            "bash",
            "-e",
            "-o",
            "pipefail",
            "-c",
            fake + _step("Invalidate classifications whose destination advanced"),
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 1
    assert "Destination invalidation failed for PR 1" in result.stdout
    assert "pr edit 2 --remove-label risk:low" in result.stdout
    assert "pr edit 2 --add-label risk:incomplete" in result.stdout

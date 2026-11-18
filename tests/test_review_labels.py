"""Label changes invalidate stale risk classifications."""

import importlib.util
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

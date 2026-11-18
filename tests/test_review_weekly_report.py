"""Weekly diagnostics deduplicate stable issue identities before a global cap."""

import importlib.util
import sys
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "review_weekly_report",
    Path(__file__).resolve().parents[1] / "scripts" / "review_weekly_report.py",
)
assert _SPEC is not None
assert _SPEC.loader is not None
weekly = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = weekly
_SPEC.loader.exec_module(weekly)


def test_all_diagnostic_sections_share_one_twenty_item_budget():
    report = "## Caught before the commit\n" + "".join(f"- `caught-{n}`: 1\n" for n in range(25))
    report += "## Rules with many suppressions\n" + "".join(
        f"- `noisy-{n}`: 6\n" for n in range(25)
    )
    result = weekly.format_report(report, [], "Weekly review report")
    assert result.body.count("<!-- botica:history-item:") == 20
    assert result.actionable == 25
    assert result.omitted == 30
    assert "`noisy-0`" in result.body
    assert "`caught-0`" not in result.body


def test_issue_deduplication_precedes_the_cap_and_is_stable_when_counts_change():
    report = "## Rules with many suppressions\n- `noisy`: 6 suppressed\n"
    first = weekly.format_report(report, [], "Weekly review report")
    opened = weekly.OpenIssue(number=10, title="Fix noisy rule", body=first.body)
    repeat = weekly.format_report(
        report.replace("6 suppressed", "8 suppressed"), [opened], "Weekly review report"
    )
    assert repeat.actionable == 0
    assert repeat.duplicates == 1
    assert "<!-- botica:history-item:" not in repeat.body
    own = weekly.OpenIssue(number=20, title="Weekly review report", body=first.body)
    update = weekly.format_report(report, [own], "Weekly review report")
    assert update.actionable == 1
    assert update.issue_number == 20

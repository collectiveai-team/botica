"""Prepare one weekly issue with stable diagnostic identities and a global item budget."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from dataclasses import asdict, dataclass

SECTIONS = {
    "History coverage gaps": -1,
    "Reached CI": 0,
    "Rules with many suppressions": 1,
    "Findings of the roles that refute did not confirm": 2,
    "Rules with no findings": 3,
    "Caught before the commit": 4,
}


@dataclass
class OpenIssue:
    """An open issue returned by GitHub, excluding pull requests."""

    number: int
    title: str
    body: str


@dataclass
class WeeklyReport:
    """The issue body and the decision of whether it contains new actionable diagnostics."""

    body: str
    actionable: int
    omitted: int
    duplicates: int
    issue_number: int | None


@dataclass
class _Item:
    position: int
    heading: str
    marker: str

    @property
    def actionable(self) -> bool:
        return self.heading != "Caught before the commit"


def _items(lines: list[str]) -> list[_Item]:
    result = []
    heading = ""
    for position, line in enumerate(lines):
        if line.startswith("## "):
            heading = line[3:].strip()
        match = re.match(r"^- `([^`]+)`:", line)
        if heading not in SECTIONS or match is None:
            continue
        identity = hashlib.blake2s(f"{heading}\0{match[1]}".encode(), digest_size=16).hexdigest()
        result.append(_Item(position, heading, f"<!-- botica:history-item:{identity} -->"))
    return result


def _retained(items: list[_Item], issues: list[OpenIssue], title: str) -> tuple[list[_Item], int]:
    existing = {
        marker
        for issue in issues
        if issue.title != title
        for marker in re.findall(r"<!-- botica:history-item:[0-9a-f]{32} -->", issue.body)
    }
    kept = [item for item in items if item.marker not in existing]
    return kept, len(items) - len(kept)


def format_report(
    markdown: str, issues: list[OpenIssue], title: str, maximum: int = 20
) -> WeeklyReport:
    """Deduplicate diagnoses, prioritize actionable ones, then cap across every section."""
    if not 1 <= maximum <= 20:
        raise ValueError("The weekly report permits between 1 and 20 items")
    lines = markdown.splitlines()
    all_items = _items(lines)
    kept, duplicates = _retained(all_items, issues, title)
    ordered = sorted(kept, key=lambda item: (SECTIONS[item.heading], item.position))
    selected = {item.position: item for item in ordered[:maximum]}
    item_positions = {item.position for item in all_items}
    output = []
    for position, line in enumerate(lines):
        if position not in item_positions:
            output.append(line)
        elif position in selected:
            output.append(f"{line} {selected[position].marker}")
    omitted = max(0, len(kept) - maximum)
    output.append(
        f"\nShown {len(selected)} diagnostics; omitted {omitted} beyond the global limit; "
        f"excluded {duplicates} already tracked in other open issues."
    )
    output.append(
        "Markers identify the diagnostic category and rule, not an individual finding fingerprint. "
        "Issues without these markers cannot be deduplicated with certainty."
    )
    own = [issue.number for issue in issues if issue.title == title]
    return WeeklyReport(
        body="\n".join(output) + "\n",
        actionable=sum(item.actionable for item in kept),
        omitted=omitted,
        duplicates=duplicates,
        issue_number=min(own) if own else None,
    )


if __name__ == "__main__":
    encoded = sys.stdin.buffer.read(8 * 1024 * 1024 + 1)
    if len(encoded) > 8 * 1024 * 1024:
        raise ValueError("Weekly report inputs exceed 8 MiB")
    request = json.loads(encoded)
    issues = [
        OpenIssue(number=int(issue["number"]), title=issue["title"], body=issue.get("body") or "")
        for issue in request["issues"]
        if "pull_request" not in issue
    ]
    markdown = request["report"]
    coverage = request.get("coverage") or {"status": "complete"}
    if coverage.get("status") != "complete":
        markdown = (
            "## History coverage gaps\n\n"
            "- `history-artifact-coverage`: INCOMPLETE. Source coverage: "
            f"{json.dumps(coverage)}\n\n" + markdown
        )
    report = format_report(markdown, issues, request["title"], int(request["maximum"]))
    report.body = f"{request['introduction']}\n\n{report.body}\nRun: {request['run_url']}\n"
    if coverage.get("status") != "complete":
        report.body = (
            "INCOMPLETE HISTORY: the diagnostics cover only the available evidence.\n\n"
            + report.body
        )
    if len(report.body.encode()) > 60_000:
        raise ValueError("Weekly report exceeds the issue body bound; no report was published")
    print(json.dumps(asdict(report)))

"""Compute label replacement from risk evidence without running reviewed code."""

from __future__ import annotations

import json
import sys
from dataclasses import asdict, dataclass
from typing import TypeAlias

Json: TypeAlias = str | int | float | bool | list["Json"] | dict[str, "Json"] | None


@dataclass
class LabelDecision:
    """Botica-owned label replacements."""

    remove: list[str]
    add: list[str]


def _areas(assessment: Json) -> set[str]:
    if not isinstance(assessment, dict):
        return set()
    comparisons = assessment.get("comparisons")
    if not isinstance(comparisons, list):
        return set()
    result: set[str] = set()
    for comparison in comparisons:
        if not isinstance(comparison, dict):
            continue
        areas = comparison.get("areas")
        if isinstance(areas, list):
            result.update(
                f"area:{area}"
                for area in areas
                if isinstance(area, str) and area in {"critical", "platform", "core", "periphery"}
            )
    return result


def label_decision(report: dict[str, Json], existing: list[str]) -> LabelDecision:
    """Replace botica-owned labels, including invalidation on incomplete evidence."""
    desired = {"risk:incomplete"}
    if report.get("status") == "complete" and report.get("effective_tier") in {
        "low",
        "medium",
        "high",
    }:
        desired = {f"risk:{report['effective_tier']}"}
        for name in ("branch", "integration"):
            desired.update(_areas(report.get(name)))
    owned = {label for label in existing if label.startswith(("risk:", "area:"))}
    return LabelDecision(remove=sorted(owned - desired), add=sorted(desired - set(existing)))


if __name__ == "__main__":
    request = json.load(sys.stdin)
    print(json.dumps(asdict(label_decision(request["risk"], request["labels"]))))

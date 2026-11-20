"""Write the `covered.expected_block` of each stale role into its REVIEW.md.

A role binds the blake3 of the files it covers (`website/docs/guide/review-plan.md`). A change to
one of them, such as the pull request of the workflow `pins`, leaves the block stale, and
`botica review plan` on the destination returns `incomplete`. This script reads the JSON of
`botica review check --format json` and replaces each stale block. It fails on a role whose
coverage is unverified, since no block makes that one current.

Usage: `python scripts/refresh_review_coverage.py check.json`
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import TypedDict


class Covered(TypedDict):
    status: str
    expected_block: str | None


class Definition(TypedDict):
    path: str


class Role(TypedDict):
    role: str
    definition: Definition
    covered: Covered


class Report(TypedDict):
    roles: list[Role]


BLOCK = re.compile(r"<!-- botica:covered -->.*?<!-- /botica:covered -->", re.S)


def refresh(report: Report, root: Path) -> list[str]:
    """Rewrite the stale blocks under `root`; return the roles that changed."""
    changed = []
    for role in report["roles"]:
        covered = role["covered"]
        status = covered["status"]
        if status == "current":
            continue
        name = role["role"]
        if status != "stale" or not covered.get("expected_block"):
            raise SystemExit(f"the coverage of {name} is {status}; fix it by hand")
        path = root / role["definition"]["path"]
        text = path.read_text()
        block = covered["expected_block"] or ""
        new, count = BLOCK.subn(lambda _, block=block: block, text, count=1)
        if count != 1:
            raise SystemExit(f"{path} has no coverage block")
        path.write_text(new)
        changed.append(name)
    return changed


def main() -> None:
    report: Report = json.loads(Path(sys.argv[1]).read_text())
    for name in refresh(report, Path.cwd()):
        print(f"refreshed the coverage block of {name}")


if __name__ == "__main__":
    main()

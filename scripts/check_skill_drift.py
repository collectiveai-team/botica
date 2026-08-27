"""Report drift between the curated upstream skill catalog and mattpocock/skills.

`npx skills add` skips unknown skill names without failing, so an upstream rename
lands as a silently missing skill in every scaffolded repo. This checks the
catalog in `scaffolding.skills` against the pinned tag (always) and against
the newest upstream tag (with --check-upstream, for the scheduled run).

Run: uv run python scripts/check_skill_drift.py [--check-upstream]
"""

from __future__ import annotations

import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

import cyclopts

from scaffolding.skills import MATTPOCOCK_REF, MATTPOCOCK_REPO, MATTPOCOCK_SKILLS

CLONE_URL = f"https://github.com/{MATTPOCOCK_REPO}.git"
SEMVER_TAG = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")
FRONTMATTER_NAME = re.compile(r"^name:\s*(\S+)\s*$", re.MULTILINE)

app = cyclopts.App(name="check-skill-drift", help=__doc__)


@dataclass
class DriftReport:
    """Outcome of comparing the curated catalog against one upstream ref."""

    ref: str
    missing: list[str] = field(default_factory=list)
    added: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.missing


def _run(args: list[str], cwd: Path | None = None) -> str:
    return subprocess.run(args, cwd=cwd, check=True, capture_output=True, text=True).stdout


def latest_tag() -> str:
    """Newest semver tag on the upstream remote."""
    refs = _run(["git", "ls-remote", "--tags", "--refs", CLONE_URL])
    tags = [line.rsplit("/", 1)[-1] for line in refs.splitlines()]
    versioned = [(SEMVER_TAG.match(tag), tag) for tag in tags]
    ranked = sorted(
        (tuple(int(g) for g in match.groups()), tag) for match, tag in versioned if match
    )
    if not ranked:
        message = f"no semver tags found on {CLONE_URL}"
        raise RuntimeError(message)
    return ranked[-1][1]


def skill_names_at(ref: str) -> set[str]:
    """Every skill name published by upstream at `ref`."""
    with tempfile.TemporaryDirectory() as tmp:
        _run(["git", "clone", "--depth", "1", "--branch", ref, "--quiet", CLONE_URL, tmp])
        found = set()
        for skill_md in Path(tmp).glob("skills/*/*/SKILL.md"):
            match = FRONTMATTER_NAME.search(skill_md.read_text(encoding="utf-8"))
            found.add(match.group(1) if match else skill_md.parent.name)
        return found


def compare(ref: str) -> DriftReport:
    published = skill_names_at(ref)
    curated = set(MATTPOCOCK_SKILLS)
    return DriftReport(
        ref=ref,
        missing=sorted(curated - published),
        added=sorted(published - curated),
    )


def _describe(report: DriftReport, *, show_added: bool) -> list[str]:
    lines = [f"### `{MATTPOCOCK_REPO}` @ `{report.ref}`"]
    if report.missing:
        lines.append(f"- **Missing from upstream** (renamed or removed): `{report.missing}`")
    else:
        lines.append(f"- All {len(MATTPOCOCK_SKILLS)} curated skills are published here.")
    if show_added and report.added:
        lines.append(f"- Not curated (available to adopt): `{report.added}`")
    return lines


@app.default
def check(*, check_upstream: bool = False) -> None:
    """Validate the pin, and optionally compare it against the newest tag."""
    lines: list[str] = []
    pinned = compare(MATTPOCOCK_REF)
    lines += _describe(pinned, show_added=False)

    upstream_drifted = False
    if check_upstream:
        newest = latest_tag()
        if newest == MATTPOCOCK_REF:
            lines.append(f"\nPin is current: `{MATTPOCOCK_REF}` is the newest tag.")
        else:
            report = compare(newest)
            upstream_drifted = not report.ok
            lines.append(f"\nA newer tag is available: `{MATTPOCOCK_REF}` -> `{newest}`.")
            lines += _describe(report, show_added=True)
            if upstream_drifted:
                lines.append(
                    "\nBumping `MATTPOCOCK_REF` as-is would silently drop the skills listed "
                    "above. Rename them in `MATTPOCOCK_SKILLS` first."
                )

    summary = "\n".join(lines)
    print(summary)
    if not pinned.ok:
        print(f"\nERROR: the pinned ref {MATTPOCOCK_REF} does not publish every curated skill.")
    sys.exit(1 if not pinned.ok or upstream_drifted else 0)


if __name__ == "__main__":
    app()

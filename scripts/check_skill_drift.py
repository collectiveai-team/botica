"""Report drift between the upstream skills of the house baseline and mattpocock/skills.

`npx skills add` skips unknown skill names without failing, so an upstream rename
lands as a silently missing skill in every repository that installs the recipe `base`.
This checks the `[[skills]]` step of `catalog/recipes/base.toml` whose source pins a tag
against that tag (always) and against the newest upstream tag (with --check-upstream,
for the scheduled run).

Run: uv run python scripts/check_skill_drift.py [--check-upstream]
"""

from __future__ import annotations

import re
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

import cyclopts

BASE_RECIPE = Path(__file__).resolve().parents[1] / "catalog" / "recipes" / "base.toml"
SEMVER_TAG = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")
FRONTMATTER_NAME = re.compile(r"^name:\s*(\S+)\s*$", re.MULTILINE)

app = cyclopts.App(name="check-skill-drift", help=__doc__)


@dataclass(frozen=True)
class Catalog:
    """The curated upstream skills: the repository, its pinned tag and the names."""

    repo: str
    ref: str
    names: list[str]

    @property
    def clone_url(self) -> str:
        return f"https://github.com/{self.repo}.git"


def curated_catalog(recipe: Path) -> Catalog:
    """Read the one `[[skills]]` step of `recipe` whose source is `owner/repo#tag`."""
    steps = tomllib.loads(recipe.read_text(encoding="utf-8")).get("skills", [])
    pinned = [step for step in steps if "#" in step["source"]]
    if len(pinned) != 1:
        if not pinned:
            message = f"{recipe}: no [[skills]] step pins a tag"
        else:
            message = f"{recipe}: {len(pinned)} [[skills]] steps pin a tag"
        raise ValueError(message)
    repo, ref = pinned[0]["source"].split("#", 1)
    return Catalog(repo=repo, ref=ref, names=list(pinned[0]["names"]))


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


def latest_tag(catalog: Catalog) -> str:
    """Newest semver tag on the upstream remote."""
    refs = _run(["git", "ls-remote", "--tags", "--refs", catalog.clone_url])
    tags = [line.rsplit("/", 1)[-1] for line in refs.splitlines()]
    versioned = [(SEMVER_TAG.match(tag), tag) for tag in tags]
    ranked = sorted(
        (tuple(int(g) for g in match.groups()), tag) for match, tag in versioned if match
    )
    if not ranked:
        message = f"no semver tags found on {catalog.clone_url}"
        raise RuntimeError(message)
    return ranked[-1][1]


def skill_names_at(catalog: Catalog, ref: str) -> set[str]:
    """Every skill name published by upstream at `ref`."""
    with tempfile.TemporaryDirectory() as tmp:
        _run(["git", "clone", "--depth", "1", "--branch", ref, "--quiet", catalog.clone_url, tmp])
        found = set()
        for skill_md in Path(tmp).glob("skills/*/*/SKILL.md"):
            match = FRONTMATTER_NAME.search(skill_md.read_text(encoding="utf-8"))
            found.add(match.group(1) if match else skill_md.parent.name)
        return found


def compare(catalog: Catalog, ref: str) -> DriftReport:
    published = skill_names_at(catalog, ref)
    curated = set(catalog.names)
    return DriftReport(
        ref=ref,
        missing=sorted(curated - published),
        added=sorted(published - curated),
    )


def _describe(catalog: Catalog, report: DriftReport, *, show_added: bool) -> list[str]:
    lines = [f"### `{catalog.repo}` @ `{report.ref}`"]
    if report.missing:
        lines.append(f"- **Missing from upstream** (renamed or removed): `{report.missing}`")
    else:
        lines.append(f"- All {len(catalog.names)} curated skills are published here.")
    if show_added and report.added:
        lines.append(f"- Not curated (available to adopt): `{report.added}`")
    return lines


@app.default
def check(*, check_upstream: bool = False) -> None:
    """Validate the pin, and optionally compare it against the newest tag."""
    catalog = curated_catalog(BASE_RECIPE)
    lines: list[str] = []
    pinned = compare(catalog, catalog.ref)
    lines += _describe(catalog, pinned, show_added=False)

    upstream_drifted = False
    if check_upstream:
        newest = latest_tag(catalog)
        if newest == catalog.ref:
            lines.append(f"\nPin is current: `{catalog.ref}` is the newest tag.")
        else:
            report = compare(catalog, newest)
            upstream_drifted = not report.ok
            lines.append(f"\nA newer tag is available: `{catalog.ref}` -> `{newest}`.")
            lines += _describe(catalog, report, show_added=True)
            if upstream_drifted:
                lines.append(
                    "\nBumping the tag in `catalog/recipes/base.toml` as-is would silently drop "
                    "the skills listed above. Rename them in its `names` first."
                )

    summary = "\n".join(lines)
    print(summary)
    if not pinned.ok:
        print(f"\nERROR: the pinned ref {catalog.ref} does not publish every curated skill.")
    sys.exit(1 if not pinned.ok or upstream_drifted else 0)


if __name__ == "__main__":
    app()

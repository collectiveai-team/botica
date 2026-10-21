"""Point every use of the mirror of botica at one release.

prek installs botica through the mirror `collectiveai-team/botica-pre-commit`, at the tag that
`rev` names. A release sets that tag in this repository's own `prek.toml`, in the template of the
recipes, in the examples of the docs and of the mirror's README, and sets the version and the
`botica==` pin of the mirror's package. The release workflow runs `bump` twice: on the copy it
pushes to the mirror, and on the branch of the release after the mirror has its tag.
"""

from __future__ import annotations

import re
from pathlib import Path

import cyclopts

app = cyclopts.App(name="bump-mirror-rev", help=__doc__)

VERSION = re.compile(r"^\d+\.\d+\.\d+$")
MIRROR = "https://github.com/collectiveai-team/botica-pre-commit"
# Each file names the mirror on one line and its `rev` on the next.
REV_FILES = (
    "prek.toml",
    "catalog/recipes/files/prek-base.toml",
    "website/docs/guide/prek-and-ci.md",
    "distribution/botica-pre-commit/README.md",
)
REV = re.compile(rf'^(?P<head>\s*repo = "{re.escape(MIRROR)}"\n\s*rev = ")v[^"]*(?P<tail>")', re.M)
PACKAGE = "distribution/botica-pre-commit/pyproject.toml"
PACKAGE_VERSION = re.compile(r'^(?P<head>version = ")[^"]*(?P<tail>")', re.M)
PIN = re.compile(r'(?P<head>"botica==)[^"]*(?P<tail>")')


def _replace(path: Path, pattern: re.Pattern[str], value: str) -> bool:
    """Set `value` between `head` and `tail` of each match; fail when `pattern` is absent."""
    text = path.read_text(encoding="utf-8")
    new, count = pattern.subn(lambda match: f"{match['head']}{value}{match['tail']}", text)
    if count == 0:
        raise SystemExit(f"{path}: no `{pattern.pattern}` to update")
    if new == text:
        return False
    path.write_text(new, encoding="utf-8")
    return True


def bump(root: Path, version: str) -> list[str]:
    """Point the mirror's `rev` and package at `version`; return the files that changed."""
    if not VERSION.match(version):
        raise SystemExit(f"{version!r} is not X.Y.Z")
    changed = [name for name in REV_FILES if _replace(root / name, REV, f"v{version}")]
    package = root / PACKAGE
    version_changed = _replace(package, PACKAGE_VERSION, version)
    pin_changed = _replace(package, PIN, version)
    if version_changed or pin_changed:
        changed.append(PACKAGE)
    return changed


@app.default
def bump_command(version: str) -> None:
    """Point the mirror's `rev` and package at VERSION (X.Y.Z)."""
    for name in bump(Path.cwd(), version):
        print(f"{name}: {version}")


if __name__ == "__main__":
    app()

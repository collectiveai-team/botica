"""Move the default versions of the tools that the templates of the recipes run with uvx or npx.

A parameter of `kind = "version"` of a recipe is the version of a tool that a template runs as
`uvx <package>{{ param.<name>.at }}` or `npx <package>{{ param.<name>.at }}`; Dependabot and
`prek update` do not see it. This script reads the package of each such parameter from the
templates, asks PyPI (uvx) or npm (npx) for its newest stable release that is at least
`--cooldown-days` old, the `exclude-newer` of this repository, and sets the `default` of the
parameter to it, in the recipe and in the row of the parameter in the table of
`website/docs/guide/recipes.md`. A default is never moved to an older version. The weekly workflow
`pins.yml` runs it after `scripts/sync_pins.py`.
"""

from __future__ import annotations

import json
import re
import tomllib
import urllib.request
from collections.abc import Callable
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import TypedDict, cast

import cyclopts

app = cyclopts.App(name="bump-tool-params", help=__doc__)

RECIPES = "catalog/recipes"
GUIDE = "website/docs/guide/recipes.md"
USE = re.compile(
    r"\b(?P<runner>uvx|npx)\b[^\n]*?\s(?P<package>@?[A-Za-z0-9][\w./-]*?)@?"
    r"\{\{ *param\.(?P<name>[a-z0-9-]+)(?:\.at)? *\}\}"
)
STABLE = re.compile(r"^\d+(\.\d+)*$")
REGISTRY = {"uvx": "pypi", "npx": "npm"}

# The newest stable version of a package of a registry published before a moment, or None.
Latest = Callable[[str, str, datetime], "str | None"]


@dataclass(frozen=True)
class Tool:
    recipe: Path
    param: str
    registry: str
    package: str
    default: str


def _as_version(text: str) -> tuple[int, ...]:
    return tuple(int(part) for part in text.split("."))


def tools(root: Path) -> list[Tool]:
    """Return each version parameter of the recipes that a template runs with uvx or npx."""
    used: dict[str, tuple[str, str]] = {}
    for path in sorted((root / RECIPES).rglob("*")):
        if not path.is_file():
            continue
        for match in USE.finditer(path.read_text(encoding="utf-8")):
            used[match["name"]] = (REGISTRY[match["runner"]], match["package"])
    found = []
    for recipe in sorted((root / RECIPES).glob("*.toml")):
        params = tomllib.loads(recipe.read_text(encoding="utf-8")).get("params", {})
        for name, spec in params.items():
            if spec.get("kind") != "version" or name not in used:
                continue
            registry, package = used[name]
            found.append(Tool(recipe, name, registry, package, str(spec.get("default", ""))))
    return found


def _set_default(text: str, param: str, version: str) -> str:
    block = re.compile(
        rf"(?P<head>\[params\.{re.escape(param)}\]\n"
        r'(?:(?!\[)[^\n]*\n)*?default = ")[^"]*(?P<tail>")'
    )
    return block.sub(lambda match: f"{match['head']}{version}{match['tail']}", text, count=1)


def _set_row(text: str, param: str, version: str) -> str:
    row = re.compile(
        rf"^(?P<head>\| `{re.escape(param)}` \|.*\| )`[^`]*`(?P<tail> \|)$", re.MULTILINE
    )
    return row.sub(lambda match: f"{match['head']}`{version}`{match['tail']}", text, count=1)


def bump(root: Path, latest: Latest, now: datetime, cooldown_days: int) -> list[str]:
    """Set each default to the newest release past the cooldown, in its recipe and in the guide.

    Return one `recipe: param old -> new` line per default that moved.
    """
    before = now - timedelta(days=cooldown_days)
    guide = root / GUIDE
    changed = []
    for tool in tools(root):
        newest = latest(tool.registry, tool.package, before)
        if newest is None:
            continue
        if STABLE.match(tool.default) and _as_version(newest) <= _as_version(tool.default):
            continue
        text = tool.recipe.read_text(encoding="utf-8")
        tool.recipe.write_text(_set_default(text, tool.param, newest), encoding="utf-8")
        table = guide.read_text(encoding="utf-8")
        guide.write_text(_set_row(table, tool.param, newest), encoding="utf-8")
        old = tool.default or '""'
        changed.append(
            f"{tool.recipe.relative_to(root).as_posix()}: {tool.param} {old} -> {newest}"
        )
    return changed


class _PyPIFile(TypedDict):
    upload_time_iso_8601: str
    yanked: bool


class _PyPI(TypedDict):
    releases: dict[str, list[_PyPIFile]]


class _Npm(TypedDict):
    time: dict[str, str]
    versions: dict[str, dict[str, object]]


def _get(url: str) -> object:
    with urllib.request.urlopen(url, timeout=30) as response:  # noqa: S310 -- a fixed https URL
        return json.load(response)


def _newest(published: dict[str, datetime], before: datetime) -> str | None:
    stable = [
        version for version, at in published.items() if STABLE.match(version) and at <= before
    ]
    return max(stable, key=_as_version, default=None)


def latest_release(registry: str, package: str, before: datetime) -> str | None:
    """Ask PyPI or npm for the newest stable release of `package` published before `before`."""
    if registry == "pypi":
        data = _get(f"https://pypi.org/pypi/{package}/json")
        pypi = cast("_PyPI", data)  # botica: ignore[no-type-cast] -- the JSON API of PyPI
        published = {
            version: datetime.fromisoformat(files[0]["upload_time_iso_8601"])
            for version, files in pypi["releases"].items()
            if files and not all(file.get("yanked") for file in files)
        }
    else:
        data = _get(f"https://registry.npmjs.org/{package.replace('/', '%2F')}")
        npm = cast("_Npm", data)  # botica: ignore[no-type-cast] -- the registry of npm
        published = {
            version: datetime.fromisoformat(npm["time"][version])
            for version, manifest in npm["versions"].items()
            if version in npm["time"] and "deprecated" not in manifest
        }
    return _newest(published, before)


@app.default
def bump_command(cooldown_days: int = 7) -> None:
    """Move the default versions of the uvx and npx tools of the recipes to their newest release."""
    for line in bump(Path.cwd(), latest_release, datetime.now(UTC), cooldown_days):
        print(line)


if __name__ == "__main__":
    app()

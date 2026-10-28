"""The default versions of the uvx and npx tools of the recipes move to the newest releases.

Asserted through the public seam of `scripts/bump_tool_params.py`: the recipes of a repository and
a registry go in, the lines it changed and the text of the recipes come out (CES-65). The registry
is an in-memory stand-in for PyPI and npm (CES-64).
"""

from __future__ import annotations

import importlib.util
import sys
from datetime import UTC, datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
_SPEC = importlib.util.spec_from_file_location(
    "bump_tool_params", ROOT / "scripts" / "bump_tool_params.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
btp = importlib.util.module_from_spec(_SPEC)
sys.modules["bump_tool_params"] = btp
_SPEC.loader.exec_module(btp)

NOW = datetime(2026, 10, 28, tzinfo=UTC)
RECIPE = """description = "A recipe"

[params.ruff-version]
description = "The version of ruff that the hook runs with uvx (empty: the newest)"
kind = "version"
default = ""

[params.jscpd-version]
description = "The version of jscpd that the hook runs with npx"
kind = "version"
default = "5"

[params.zizmor-version]
description = "The version of zizmor that the workflow runs with uvx"
kind = "version"
default = "2.0.0"

[params.go-version]
description = "The Go that the hook builds with"
kind = "version"
default = "1.24"
"""
TEMPLATE = """[[repos.hooks]]
entry = "uvx ruff{{ param.ruff-version.at }} check"
entry = "npx --yes jscpd{{ param.jscpd-version.at }} ."
run: uvx zizmor{{ param.zizmor-version.at }} --strict-collection .
language_version = "{{ param.go-version }}"
"""


class Registry:
    """Releases by registry and package, each with the moment it was published."""

    def __init__(self, releases: dict[tuple[str, str], dict[str, datetime]]) -> None:
        self.releases = releases
        self.asked: list[tuple[str, str]] = []

    def latest(self, registry: str, package: str, before: datetime) -> str | None:
        self.asked.append((registry, package))
        published = self.releases.get((registry, package), {})
        stable = [
            version
            for version, at in published.items()
            if at <= before and version.replace(".", "").isdigit()
        ]
        return max(stable, key=lambda version: tuple(map(int, version.split("."))), default=None)


def _repository(tmp_path: Path) -> Path:
    (tmp_path / "catalog/recipes/files").mkdir(parents=True)
    (tmp_path / "catalog/recipes/demo.toml").write_text(RECIPE, encoding="utf-8")
    (tmp_path / "catalog/recipes/files/hooks.toml").write_text(TEMPLATE, encoding="utf-8")
    return tmp_path


def _days_ago(days: int) -> datetime:
    return NOW - timedelta(days=days)


def test_each_default_moves_to_the_newest_stable_release_past_the_cooldown(tmp_path: Path):
    root = _repository(tmp_path)
    registry = Registry(
        {
            ("pypi", "ruff"): {
                "0.17.1": _days_ago(30),
                "0.17.2": _days_ago(10),
                "0.18.0": _days_ago(2),
                "0.18.0rc1": _days_ago(20),
            },
            ("npm", "jscpd"): {"5.0.3": _days_ago(40), "5.1.0": _days_ago(8)},
            ("pypi", "zizmor"): {"1.9.0": _days_ago(90)},
        }
    )

    changed = btp.bump(root, registry.latest, NOW, cooldown_days=7)

    assert changed == [
        'catalog/recipes/demo.toml: ruff-version "" -> 0.17.2',
        "catalog/recipes/demo.toml: jscpd-version 5 -> 5.1.0",
    ]
    text = (root / "catalog/recipes/demo.toml").read_text(encoding="utf-8")
    assert '[params.ruff-version]\ndescription = "The version of ruff' in text
    assert 'kind = "version"\ndefault = "0.17.2"' in text
    assert 'kind = "version"\ndefault = "5.1.0"' in text
    # an older release never replaces a newer default
    assert 'kind = "version"\ndefault = "2.0.0"' in text
    # a version that no template runs with uvx or npx is not asked for
    assert 'default = "1.24"' in text
    assert sorted(registry.asked) == [("npm", "jscpd"), ("pypi", "ruff"), ("pypi", "zizmor")]


def test_a_package_with_no_release_past_the_cooldown_keeps_its_default(tmp_path: Path):
    root = _repository(tmp_path)
    registry = Registry({("pypi", "ruff"): {"0.17.2": _days_ago(1)}})

    assert btp.bump(root, registry.latest, NOW, cooldown_days=7) == []


def test_the_recipes_of_the_catalog_name_a_package_for_each_uvx_and_npx_tool():
    found = {
        (tool.recipe.name, tool.param): (tool.registry, tool.package) for tool in btp.tools(ROOT)
    }

    assert found[("base.toml", "abide-version")] == ("npm", "@coldtea/abide")
    assert found[("ci.toml", "zizmor-version")] == ("pypi", "zizmor")
    assert found[("ci-python.toml", "prek-version")] == ("pypi", "prek")
    assert ("base-go.toml", "govulncheck-version") not in found

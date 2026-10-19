"""Every rule page of the standards site has a Discussions category to hold its thread.

The category of a page is the one named like its family, taken from `zensical.toml`. A family
that has none falls back to `General`, which works but hides that someone forgot to create it.
"""

from __future__ import annotations

import tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CATALOG = ROOT / "catalog" / "rules"
SITE_CONFIG = ROOT / "website" / "zensical.toml"


@dataclass(frozen=True)
class Giscus:
    categories: dict[str, str]
    fallback_id: str


def _giscus() -> Giscus:
    config = tomllib.loads(SITE_CONFIG.read_text(encoding="utf-8"))["project"]["extra"]["giscus"]
    return Giscus(categories=config["categories"], fallback_id=config["fallback"]["id"])


def _groups_of_the_catalog() -> set[str]:
    """Return the folders that hold rules: one per family, and `botica` for its own rules."""
    return {path.parent.parent.name for path in CATALOG.glob("*/*/rule.toml")}


def test_every_family_of_the_catalog_has_a_discussions_category():
    missing = _groups_of_the_catalog() - set(_giscus().categories)

    assert not missing, f"families with no Discussions category in zensical.toml: {sorted(missing)}"


def test_the_standards_have_a_discussions_category_of_their_own():
    assert "standards" in _giscus().categories


def test_the_ids_of_the_categories_are_those_of_discussions():
    giscus = _giscus()
    ids = [*giscus.categories.values(), giscus.fallback_id]

    assert ids
    assert all(value.startswith("DIC_") for value in ids), ids
    assert len(set(ids)) == len(ids), "two categories share an id"

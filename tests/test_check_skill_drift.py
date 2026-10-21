"""The drift check reads the curated upstream skills from the recipe `base`.

Asserted through the public seam of `scripts/check_skill_drift.py`: a recipe file goes in, the
upstream repository, its pinned tag and the curated names come out (CES-65).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

_SPEC = importlib.util.spec_from_file_location(
    "check_skill_drift", Path(__file__).resolve().parents[1] / "scripts" / "check_skill_drift.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
csd = importlib.util.module_from_spec(_SPEC)
sys.modules["check_skill_drift"] = csd
_SPEC.loader.exec_module(csd)

BASE = Path(__file__).resolve().parents[1] / "catalog" / "recipes" / "base.toml"


def _recipe(tmp_path: Path, text: str) -> Path:
    path = tmp_path / "base.toml"
    path.write_text(text, encoding="utf-8")
    return path


def test_the_catalog_is_the_skills_step_whose_source_pins_a_tag(tmp_path: Path) -> None:
    recipe = _recipe(
        tmp_path,
        '[[skills]]\nsource = "a/local"\nnames = ["x"]\n\n'
        '[[skills]]\nsource = "up/stream#v1.2.3"\nnames = ["b", "a"]\n',
    )

    catalog = csd.curated_catalog(recipe)

    assert (catalog.repo, catalog.ref, catalog.names) == ("up/stream", "v1.2.3", ["b", "a"])


def test_a_recipe_without_a_pinned_skills_step_is_an_error(tmp_path: Path) -> None:
    recipe = _recipe(tmp_path, '[[skills]]\nsource = "a/local"\nnames = ["x"]\n')

    with pytest.raises(ValueError, match="no \\[\\[skills\\]\\] step pins a tag"):
        csd.curated_catalog(recipe)


def test_two_pinned_skills_steps_are_an_error(tmp_path: Path) -> None:
    recipe = _recipe(
        tmp_path,
        '[[skills]]\nsource = "a/b#v1.0.0"\nnames = ["x"]\n\n'
        '[[skills]]\nsource = "c/d#v2.0.0"\nnames = ["y"]\n',
    )

    with pytest.raises(ValueError, match="2 \\[\\[skills\\]\\] steps pin a tag"):
        csd.curated_catalog(recipe)


def test_the_recipe_base_pins_the_skills_of_matt_pocock() -> None:
    catalog = csd.curated_catalog(BASE)

    assert catalog.repo == "mattpocock/skills"
    assert csd.SEMVER_TAG.match(catalog.ref), catalog.ref
    assert "tdd" in catalog.names
    assert len(set(catalog.names)) == len(catalog.names)

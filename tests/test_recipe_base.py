"""`lint/recipes/base.toml` carries what `scaffolding` installs, until `scaffolding` is removed."""

import tomllib
from pathlib import Path

from scaffolding import components, skills

ROOT = Path(__file__).resolve().parent.parent
RECIPES = ROOT / "lint" / "recipes"
TEMPLATES = ROOT / "scaffolding" / "templates"


def _names_from(source: str) -> list[str]:
    base = tomllib.loads((RECIPES / "base.toml").read_text(encoding="utf-8"))
    steps = [step for step in base["skills"] if step["source"] == source]
    assert len(steps) == 1, f"base.toml has {len(steps)} skills steps from {source}"
    return steps[0]["names"]


def test_base_installs_the_mattpocock_skills_that_scaffolding_installs() -> None:
    assert _names_from(skills.MATTPOCOCK_SOURCE) == skills.MATTPOCOCK_SKILLS


def test_base_installs_the_house_skills_that_scaffolding_installs() -> None:
    assert _names_from(skills.SCAFFOLDING_SOURCE) == skills.LOCAL_SKILLS


def test_base_has_the_prek_template_of_scaffolding() -> None:
    assert (RECIPES / "files" / "prek-base.toml").read_text(encoding="utf-8") == (
        TEMPLATES / "prek-generic.toml"
    ).read_text(encoding="utf-8")


def test_base_has_the_standards_index_template_of_scaffolding() -> None:
    assert (RECIPES / "sections" / "standards-index.md").read_text(encoding="utf-8") == (
        TEMPLATES / "standards-index.md"
    ).read_text(encoding="utf-8")


def _gitignore(key: str) -> list[str]:
    base = tomllib.loads((RECIPES / "base.toml").read_text(encoding="utf-8"))
    assert len(base["gitignore"]) == 1
    return base["gitignore"][0][key]


def test_base_ignores_the_entries_that_scaffolding_ignores() -> None:
    assert _gitignore("lines") == components.GITIGNORE_ENTRIES


def test_base_unignores_what_scaffolding_may_unignore() -> None:
    assert _gitignore("unignore") == sorted(skills.UNIGNORE_WHITELIST)

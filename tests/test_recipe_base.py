"""`lint/recipes/base.toml` carries what `scaffolding` installs, until `scaffolding` is removed."""

import re
import tomllib
from pathlib import Path
from typing import Any

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


def _steps(kind: str) -> list[Any]:
    recipe = tomllib.loads((RECIPES / "base-python.toml").read_text(encoding="utf-8"))
    return recipe[kind]


def _section_texts(path: str) -> list[str]:
    return [
        (RECIPES / step["template"]).read_text(encoding="utf-8")
        for step in _steps("sections")
        if step["path"] == path
    ]


def test_base_python_adds_the_python_hooks_of_scaffolding_to_prek() -> None:
    sections = _section_texts("prek.toml")

    assert "\n".join(sections) == (TEMPLATES / "prek-python.toml").read_text(encoding="utf-8")


def test_base_python_has_the_pyproject_of_scaffolding_but_for_the_dependency_groups() -> None:
    file_template = (RECIPES / _steps("files")[0]["template"]).read_text(encoding="utf-8")
    recipe = tomllib.loads("\n".join([file_template, *_section_texts("pyproject.toml")]))
    scaffolded = tomllib.loads((TEMPLATES / "pyproject-template.toml").read_text(encoding="utf-8"))
    scaffolded.pop("dependency-groups")

    assert recipe == scaffolded


def test_base_python_adds_the_dev_packages_that_scaffolding_declares() -> None:
    scaffolded = tomllib.loads((TEMPLATES / "pyproject-template.toml").read_text(encoding="utf-8"))
    dev = scaffolded["dependency-groups"]["dev"]
    declared = {re.split(r"[<>=!~ ;\[]", name, maxsplit=1)[0] for name in dev}
    tools = {package for tool in _steps("tools") for package in tool["packages"]}

    assert declared <= tools


def test_base_has_the_workspace_defaults_section_of_scaffolding() -> None:
    assert (RECIPES / "sections" / "agents-workspace-defaults.md").read_text(encoding="utf-8") == (
        TEMPLATES / "agents-workspace-defaults.md"
    ).read_text(encoding="utf-8")


def test_the_recipes_of_the_agents_have_the_templates_of_scaffolding() -> None:
    assert (RECIPES / "files" / "opencode.jsonc").read_text(encoding="utf-8") == (
        TEMPLATES / "opencode-template.jsonc"
    ).read_text(encoding="utf-8")
    assert (RECIPES / "files" / "claude-settings.json").read_text(encoding="utf-8") == (
        TEMPLATES / "claude-settings-template.json"
    ).read_text(encoding="utf-8")


def _github_templates() -> list[tuple[str, str]]:
    return [
        (path.relative_to(RECIPES / "files").as_posix(), path.read_text(encoding="utf-8"))
        for path in sorted((RECIPES / "files" / "github").rglob("*.yml"))
    ]


ONLY_IN_RECIPES = {
    "github/workflows/dependabot-automerge-label.yml",
    "github/workflows/dependabot-automerge-run.yml",
    "github/workflows/dependabot-automerge.yml",
}


def test_the_ci_templates_are_the_workflows_of_scaffolding_and_the_ones_it_lacks() -> None:
    scaffolded = [
        (path.relative_to(TEMPLATES).as_posix(), path.read_text(encoding="utf-8"))
        for path in sorted((TEMPLATES / "github").rglob("*.yml"))
    ]
    recipes = _github_templates()

    assert [item for item in recipes if item[0] not in ONLY_IN_RECIPES] == scaffolded
    assert {name for name, _ in recipes} - {name for name, _ in scaffolded} == ONLY_IN_RECIPES


def test_every_ci_template_is_used_by_one_recipe_at_its_github_path() -> None:
    steps = [
        step
        for recipe in sorted(RECIPES.glob("*.toml"))
        for step in tomllib.loads(recipe.read_text(encoding="utf-8")).get("files", [])
        if step["template"].startswith("files/github/")
    ]
    used = [(step["template"].removeprefix("files/"), step["path"]) for step in steps]

    assert sorted(template for template, _ in used) == [name for name, _ in _github_templates()]
    assert all(path == f".{template}" for template, path in used)

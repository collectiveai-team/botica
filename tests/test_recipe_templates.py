"""Every CI template under `catalog/recipes/files/github/` is a step of one recipe, at its path."""

import tomllib
from pathlib import Path

RECIPES = Path(__file__).resolve().parent.parent / "catalog" / "recipes"


def _github_templates() -> list[str]:
    return [
        path.relative_to(RECIPES / "files").as_posix()
        for path in sorted((RECIPES / "files" / "github").rglob("*"))
        if path.is_file()
    ]


def test_every_ci_template_is_used_by_one_recipe_at_its_github_path() -> None:
    steps = [
        step
        for recipe in sorted(RECIPES.glob("*.toml"))
        for step in tomllib.loads(recipe.read_text(encoding="utf-8")).get("files", [])
        if step["template"].startswith("files/github/")
    ]
    used = [(step["template"].removeprefix("files/"), step["path"]) for step in steps]

    assert sorted(template for template, _ in used) == _github_templates()
    assert all(path == f".{template}" for template, path in used)

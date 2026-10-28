"""A project takes the version of the mirror of botica in every place that names botica.

Asserted through the public seam of the script that the recipe `ci-botica-update` installs,
`catalog/recipes/files/github/scripts/botica_update.py`: the files of a project go in, the version,
the files it changed and their text come out (CES-65).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "catalog" / "recipes" / "files" / "github" / "scripts" / "botica_update.py"
_SPEC = importlib.util.spec_from_file_location("botica_update", SCRIPT)
assert _SPEC is not None
assert _SPEC.loader is not None
bu = importlib.util.module_from_spec(_SPEC)
sys.modules["botica_update"] = bu
# The folder of the templates is embedded in botica and listed by test_recipe_templates.py, so the
# import leaves no __pycache__ in it.
_WRITES_BYTECODE = sys.dont_write_bytecode
sys.dont_write_bytecode = True
_SPEC.loader.exec_module(bu)
sys.dont_write_bytecode = _WRITES_BYTECODE

PREK = """[[repos]]
repo = "local"

[[repos.hooks]]
id = "agents-conventional-commits"
# uvx botica@0.15.0 was the first with --commit-msg
entry = "uvx botica@0.16.1 lint --select conventional-commits --commit-msg"

[[repos]]
repo = "https://github.com/collectiveai-team/botica-pre-commit"
rev = "v0.17.1"
hooks = [{ id = "botica-lint" }]
"""
PRE_COMMIT = """repos:
  - repo: https://github.com/astral-sh/ruff-pre-commit
    rev: v0.15.0
  - repo: https://github.com/collectiveai-team/botica-pre-commit
    rev: "v0.16.1"
    hooks:
      - id: botica-lint
  - repo: local
    hooks:
      - id: conventional
        entry: uvx botica@0.16.1 lint --select conventional-commits --commit-msg
"""
WORKFLOW = """jobs:
  a:
    steps:
      - uses: someone/botica@v1
      - run: uvx botica@0.16.1 rules export --format abide --check
      - run: uvx my-botica@0.1.0 && uvx botica@latest lint && uvx botica@0.16.1.
"""
BOTICA_TOML = """select = ["no-utils"]

[recipes]
installed = ["base"]

[recipes.params]
botica-version = "0.16.1"
jscpd-version = "5"
"""
PYPROJECT = """[project]
name = "demo"
dependencies = ["botica==0.16.1"]

[tool.other.recipes.params]
botica-version = "0.16.1"

[tool.botica.recipes]
params = { cooldown-days = 7, botica-version = "0.16.1" }
"""


def write(root: Path, name: str, text: str) -> None:
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def read(root: Path, name: str) -> str:
    return (root / name).read_text(encoding="utf-8")


@pytest.fixture
def project(tmp_path: Path) -> Path:
    write(tmp_path, "prek.toml", PREK)
    write(tmp_path, ".pre-commit-config.yaml", PRE_COMMIT)
    write(tmp_path, ".github/workflows/abide.yml", WORKFLOW)
    write(tmp_path, ".github/workflows/other.yaml", "on: push\n")
    write(tmp_path, "botica.toml", BOTICA_TOML)
    write(tmp_path, "pyproject.toml", PYPROJECT)
    return tmp_path


def test_every_place_takes_the_version_of_the_mirror_in_prek_toml(project: Path):
    alignment = bu.align(project)

    assert alignment.version == "0.17.1"
    assert alignment.changed == [
        "prek.toml",
        ".pre-commit-config.yaml",
        ".github/workflows/abide.yml",
        "botica.toml",
        "pyproject.toml",
    ]
    assert read(project, "prek.toml") == PREK.replace(
        "uvx botica@0.16.1 lint", "uvx botica@0.17.1 lint"
    )
    assert read(project, ".pre-commit-config.yaml") == PRE_COMMIT.replace(
        '"v0.16.1"', '"v0.17.1"'
    ).replace("botica@0.16.1", "botica@0.17.1")
    assert read(project, ".github/workflows/abide.yml") == WORKFLOW.replace(
        "botica@0.16.1", "botica@0.17.1"
    )
    assert read(project, "botica.toml") == BOTICA_TOML.replace("0.16.1", "0.17.1")
    assert read(project, "pyproject.toml") == PYPROJECT.replace(
        'cooldown-days = 7, botica-version = "0.16.1"',
        'cooldown-days = 7, botica-version = "0.17.1"',
    )


def test_a_second_run_changes_nothing(project: Path):
    bu.align(project)

    assert bu.align(project).changed == []


def test_without_prek_toml_the_rev_of_pre_commit_is_the_version(project: Path):
    (project / "prek.toml").unlink()

    alignment = bu.align(project)

    assert alignment.version == "0.16.1"
    assert alignment.changed == []


def test_a_rev_that_is_not_a_version_stops_the_run(project: Path):
    write(project, "prek.toml", PREK.replace('rev = "v0.17.1"', 'rev = "3c31ac5f"'))

    with pytest.raises(SystemExit, match="is not a version"):
        bu.align(project)


def test_a_project_without_the_mirror_stops_the_run(tmp_path: Path):
    write(tmp_path, "prek.toml", "repos = []\n")
    write(tmp_path, ".pre-commit-config.yaml", "repos:\n  - repo: local\n    rev: v1.0.0\n")

    with pytest.raises(SystemExit, match="no `rev` of"):
        bu.align(tmp_path)

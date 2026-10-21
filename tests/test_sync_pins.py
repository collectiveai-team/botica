"""The templates of the recipes take the pins of this repository.

Asserted through the public seam of `scripts/sync_pins.py`: the files of a repository go in, the
files it changed and their text come out (CES-65).
"""

from __future__ import annotations

import importlib.util
import shutil
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
_SPEC = importlib.util.spec_from_file_location("sync_pins", ROOT / "scripts" / "sync_pins.py")
assert _SPEC is not None
assert _SPEC.loader is not None
sp = importlib.util.module_from_spec(_SPEC)
sys.modules["sync_pins"] = sp
_SPEC.loader.exec_module(sp)

PREK = '[[repos]]\nrepo = "https://github.com/astral-sh/uv-pre-commit"\nrev = "0.14.0"\n'
OWN = """jobs:
  a:
    steps:
      - uses: actions/checkout@v7
      - uses: astral-sh/setup-uv@v10.3.0
        with:
          version: "0.14.0"
      - uses: pypa/gh-action-pypi-publish@v1.14.2
      - uses: ./.github/actions/local
  b:
    steps:
      - uses: astral-sh/setup-uv@v10.1.0
        with:
          version: "0.14.0"
"""
TEMPLATE = """jobs:
  test:
    steps:
      - uses: actions/checkout@v6
      - name: Install uv
        # a comment
        uses: astral-sh/setup-uv@v10.0.1
        with:
          version: "0.12.3"
          enable-cache: true
      - uses: pypa/gh-action-pypi-publish@release/v1
      - uses: docker/login-action@v4
"""


def write(root: Path, name: str, text: str) -> None:
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


@pytest.fixture
def repository(tmp_path: Path) -> Path:
    write(tmp_path, "prek.toml", PREK)
    write(tmp_path, ".github/workflows/own.yml", OWN)
    write(tmp_path, "catalog/recipes/files/github/workflows/tests.yml", TEMPLATE)
    return tmp_path


def test_a_template_takes_the_highest_pin_of_each_action_and_the_uv_of_prek(repository: Path):
    changed = sp.sync(repository)

    assert changed == [
        ".github/workflows/own.yml",
        "catalog/recipes/files/github/workflows/tests.yml",
    ]
    template = (repository / "catalog/recipes/files/github/workflows/tests.yml").read_text()
    assert "actions/checkout@v7\n" in template
    assert "astral-sh/setup-uv@v10.3.0\n" in template
    assert 'version: "0.14.0"\n          enable-cache: true' in template
    assert "pypa/gh-action-pypi-publish@v1.14.2\n" in template
    assert "docker/login-action@v4\n" in template
    own = (repository / ".github/workflows/own.yml").read_text()
    assert own.count("astral-sh/setup-uv@v10.3.0") == 2
    assert "./.github/actions/local" in own


def test_a_second_run_changes_nothing(repository: Path):
    sp.sync(repository)

    assert sp.sync(repository) == []


def test_a_prek_toml_without_the_uv_hook_stops_the_run(repository: Path):
    write(repository, "prek.toml", "repos = []\n")

    with pytest.raises(SystemExit, match="uv-pre-commit"):
        sp.sync(repository)


def test_this_repository_is_in_sync(tmp_path: Path):
    shutil.copyfile(ROOT / "prek.toml", tmp_path / "prek.toml")
    shutil.copytree(ROOT / ".github/workflows", tmp_path / ".github/workflows")
    shutil.copytree(ROOT / "catalog/recipes/files", tmp_path / "catalog/recipes/files")

    assert sp.sync(tmp_path) == []

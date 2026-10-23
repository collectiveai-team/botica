"""A release points every use of the mirror of botica at its own version.

Asserted through the public seam of `scripts/bump_mirror_rev.py`: the files of the repository go
in, the files it changed and their text come out (CES-65).
"""

from __future__ import annotations

import importlib.util
import shutil
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
_SPEC = importlib.util.spec_from_file_location(
    "bump_mirror_rev", ROOT / "scripts" / "bump_mirror_rev.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
bmr = importlib.util.module_from_spec(_SPEC)
sys.modules["bump_mirror_rev"] = bmr
_SPEC.loader.exec_module(bmr)

OTHER_HOOK = '[[repos]]\nrepo = "https://github.com/astral-sh/ruff-pre-commit"\nrev = "v0.15.22"\n'


def copy_of_the_repository(tmp_path: Path) -> Path:
    for name in (*bmr.REV_FILES, bmr.PACKAGE):
        target = tmp_path / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / name, target)
    return tmp_path


def test_every_file_of_this_repository_names_the_mirror_and_is_moved_to_the_release(tmp_path: Path):
    root = copy_of_the_repository(tmp_path)

    changed = bmr.bump(root, "9.8.7")

    assert sorted(changed) == sorted([*bmr.REV_FILES, bmr.PACKAGE])
    for name in bmr.REV_FILES:
        text = (root / name).read_text(encoding="utf-8")
        assert 'botica-pre-commit"\n' in text
        assert 'rev = "v9.8.7"' in text, name
    package = (root / bmr.PACKAGE).read_text(encoding="utf-8")
    assert 'version = "9.8.7"' in package
    assert '"botica==9.8.7"' in package


def test_a_second_run_with_the_same_version_changes_nothing(tmp_path: Path):
    root = copy_of_the_repository(tmp_path)
    bmr.bump(root, "9.8.7")

    assert bmr.bump(root, "9.8.7") == []


def test_only_the_rev_of_the_mirror_moves(tmp_path: Path):
    root = copy_of_the_repository(tmp_path)
    doc = root / "website/docs/guide/prek-and-ci.md"
    doc.write_text(OTHER_HOOK + doc.read_text(encoding="utf-8"), encoding="utf-8")

    bmr.bump(root, "9.8.7")

    assert doc.read_text(encoding="utf-8").startswith(OTHER_HOOK)


def test_a_file_that_no_longer_names_the_mirror_fails_the_release(tmp_path: Path):
    root = copy_of_the_repository(tmp_path)
    (root / "website/docs/guide/prek-and-ci.md").write_text("# prek\n", encoding="utf-8")

    with pytest.raises(SystemExit, match=r"prek-and-ci\.md"):
        bmr.bump(root, "9.8.7")


def test_a_version_that_is_not_x_y_z_is_refused(tmp_path: Path):
    root = copy_of_the_repository(tmp_path)

    with pytest.raises(SystemExit, match=r"X\.Y\.Z"):
        bmr.bump(root, "v9.8.7")

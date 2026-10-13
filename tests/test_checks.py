"""`check` gating that keys off prek config contents."""

from __future__ import annotations

import subprocess
from typing import TYPE_CHECKING

import pytest

from scaffolding.checks import run_checks

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)
    return tmp_path


def test_no_check_demands_the_files_of_an_ast_grep_hook(repo: Path):
    """The scaffolder installs no ast-grep hook, so it does not ask for its config either."""
    (repo / "prek.toml").write_text(
        '[[repos]]\nrepo = "local"\n\n'
        '[[repos.hooks]]\nid = "ast-grep"\nentry = "ast-grep scan"\nlanguage = "system"\n',
        encoding="utf-8",
    )

    names = [check.name for check in run_checks(repo)]

    assert "ast-grep config" not in names

"""Tier-0 fact detection: git working-tree root discovery."""

from __future__ import annotations

import subprocess
from typing import TYPE_CHECKING

import pytest

from scaffolding.facts import detect, is_git_root

if TYPE_CHECKING:
    from pathlib import Path


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)
    return tmp_path


def test_is_git_root_accepts_ordinary_checkout(repo: Path):
    assert is_git_root(repo)
    assert detect(repo, probe_visibility=False).is_git_repo


def test_is_git_root_accepts_linked_worktree(repo: Path, tmp_path: Path):
    """A linked worktree is a valid install target.

    `(root / ".git").is_dir()` is False there — a linked worktree's `.git` is a FILE
    containing `gitdir: ...` — which made `install` abort with "Not a git repository
    root (no .git/). cd into the repo first." in a directory that *was* the root.
    """
    subprocess.run(["git", "config", "user.email", "t@example.invalid"], cwd=repo, check=True)
    subprocess.run(["git", "config", "user.name", "t"], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-q", "--allow-empty", "-m", "init"], cwd=repo, check=True)

    worktree = tmp_path.parent / f"{tmp_path.name}-wt"
    subprocess.run(
        ["git", "worktree", "add", "-q", "-b", "wt", str(worktree)], cwd=repo, check=True
    )

    assert (worktree / ".git").is_file(), "precondition: a worktree's .git is a file"
    assert is_git_root(worktree)
    assert detect(worktree, probe_visibility=False).is_git_repo


def test_is_git_root_rejects_subdirectory(repo: Path):
    """Root-only, still: `install` writes to cwd, so a subdirectory must stay rejected."""
    sub = repo / "pkg"
    sub.mkdir()

    assert not is_git_root(sub)
    assert not detect(sub, probe_visibility=False).is_git_repo


def test_is_git_root_rejects_non_repo(tmp_path: Path):
    plain = tmp_path / "plain"
    plain.mkdir()

    assert not is_git_root(plain)

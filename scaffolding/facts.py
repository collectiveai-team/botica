"""Tier-0 facts: deterministic repo/environment detection (never asked)."""

from __future__ import annotations

import contextlib
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Facts:
    cwd: str
    is_git_repo: bool
    is_python: bool
    has_dockerfile: bool
    visibility: str  # public | private | internal | unknown
    has_npx: bool
    has_gh: bool
    has_varlock: bool
    has_uv: bool
    has_curl: bool


def _has(cmd: str) -> bool:
    return shutil.which(cmd) is not None


def repo_visibility() -> str:
    if not _has("gh"):
        return "unknown"
    with contextlib.suppress(OSError, subprocess.SubprocessError):
        out = subprocess.run(
            ["gh", "repo", "view", "--json", "visibility", "-q", ".visibility"],
            capture_output=True,
            text=True,
            timeout=15,
            check=False,
        )
        if out.returncode == 0:
            return out.stdout.strip().lower() or "unknown"
    return "unknown"


def _git_ok(root: Path, args: list[str]) -> bool:
    """Return True when `git <args>` exits 0; any failure to run git at all is False."""
    try:
        out = subprocess.run(
            ["git", *args], cwd=root, capture_output=True, text=True, timeout=10, check=False
        )
    except (OSError, subprocess.SubprocessError):
        return False
    return out.returncode == 0


def gitignored(root: Path, rel: str) -> bool:
    return _git_ok(root, ["check-ignore", "-q", rel])


def git_tracked(root: Path, rel: str) -> bool:
    return _git_ok(root, ["ls-files", "--error-unmatch", rel])


def is_git_root(root: Path) -> bool:
    """Return True when `root` is the top level of a git working tree.

    Asking git rather than testing `(root / ".git").is_dir()`: that test is False in a
    linked worktree, where `.git` is a FILE containing `gitdir: ...`. It is also True for
    a directory that merely *contains* a repo checkout. `rev-parse --show-toplevel`
    resolves both cases, and comparing its answer to `root` keeps a subdirectory of the
    repo rejected — which is what the `install` error message tells the user to fix.
    """
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=root,
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return False
    if out.returncode != 0 or not out.stdout.strip():
        return False
    try:
        return Path(out.stdout.strip()).resolve() == root.resolve()
    except OSError:
        return False


def is_python_repo(root: Path) -> bool:
    if (root / "pyproject.toml").exists():
        return True
    return any(root.glob("*.py"))


def detect(root: Path | None = None, *, probe_visibility: bool = True) -> Facts:
    root = root or Path.cwd()
    return Facts(
        cwd=str(root),
        is_git_repo=is_git_root(root),
        is_python=is_python_repo(root),
        has_dockerfile=(root / "Dockerfile").exists(),
        visibility=repo_visibility() if probe_visibility else "unknown",
        has_npx=_has("npx"),
        has_gh=_has("gh"),
        has_varlock=_has("varlock"),
        has_uv=_has("uv"),
        has_curl=_has("curl"),
    )

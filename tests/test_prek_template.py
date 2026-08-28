"""Contract tests for the shipped prek hook templates."""

from __future__ import annotations

import shlex
import tomllib

import pytest

from scaffolding.templates_registry import template_text

# Generated, tracked, and highly repetitive. `.gitignore` does not exclude a lockfile
# (it is committed on purpose), so jscpd reads it and its size can dominate the score.
LOCKFILES = (
    "uv.lock",
    "poetry.lock",
    "Pipfile.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "Cargo.lock",
    "go.sum",
    "composer.lock",
    "Gemfile.lock",
)


def _hook_entry(template: str, hook_id: str) -> str:
    """Return the `entry` command of a hook declared in a shipped prek template."""
    config = tomllib.loads(template_text(template))
    for repo in config.get("repos", []):
        for hook in repo.get("hooks", []):
            if hook.get("id") == hook_id:
                return hook["entry"]
    pytest.fail(f"{template} declares no {hook_id!r} hook")


def _ignore_arg(entry: str) -> str:
    """Return the value passed to jscpd's `--ignore`."""
    parts = shlex.split(entry)
    assert "--ignore" in parts, f"jscpd entry has no --ignore: {entry!r}"
    return parts[parts.index("--ignore") + 1]


def test_jscpd_ignores_generated_lockfiles():
    ignored = _ignore_arg(_hook_entry("prek-generic.toml", "jscpd"))
    patterns = {p.strip() for p in ignored.split(",")}

    for lockfile in LOCKFILES:
        assert f"**/{lockfile}" in patterns, f"jscpd would score {lockfile} as source"


def test_jscpd_does_not_combine_threshold_with_exit_code():
    """`--exit-code` fails on ANY clone, which makes `--threshold` inert."""
    entry = _hook_entry("prek-generic.toml", "jscpd")

    assert "--threshold" in entry
    assert "--exit-code" not in entry

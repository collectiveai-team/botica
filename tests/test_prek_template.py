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

# The native rules the botica hook runs: CES-71, CES-63 and the layout rules (was `repo-shape`).
BOTICA_RULES = {
    "file-size-guard",
    "no-utils",
    "no-notebooks",
    "no-artifacts-in-package",
    "no-tests-in-package",
}


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


def test_botica_hook_runs_the_native_structure_rules():
    entry = shlex.split(_hook_entry("prek-generic.toml", "botica"))

    assert entry[:3] == ["botica", "lint", "--no-astgrep"]
    assert "--select" in entry, f"botica would run every rule it knows: {entry!r}"
    assert set(entry[entry.index("--select") + 1].split(",")) == BOTICA_RULES


def test_the_botica_hook_is_declared_once_so_the_joined_config_has_no_duplicate_id():
    """A Python repo gets the generic and the Python templates joined into one `prek.toml`."""
    joined = tomllib.loads(
        template_text("prek-generic.toml") + "\n" + template_text("prek-python.toml")
    )
    ids = [hook["id"] for repo in joined["repos"] for hook in repo.get("hooks", [])]

    assert ids.count("botica") == 1

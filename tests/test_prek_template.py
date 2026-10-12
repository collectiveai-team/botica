"""Contract tests for the shipped prek hook templates."""

from __future__ import annotations

import re
import shlex
import tomllib

import pytest

from scaffolding.templates_registry import template_text

BOTICA_MIRROR = "https://github.com/collectiveai-team/botica-pre-commit"

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


def _hook_setting(template: str, hook_id: str, key: str) -> object:
    """Return one setting of a hook declared in a shipped prek template (`None` if unset)."""
    config = tomllib.loads(template_text(template))
    for repo in config.get("repos", []):
        for hook in repo.get("hooks", []):
            if hook.get("id") == hook_id:
                return hook.get(key)
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


def _hook_source(template: str, hook_id: str) -> tuple[str, str]:
    """Return the `(repo, rev)` of the `[[repos]]` table that declares the hook."""
    config = tomllib.loads(template_text(template))
    for repo in config.get("repos", []):
        if any(hook.get("id") == hook_id for hook in repo.get("hooks", [])):
            return repo["repo"], repo.get("rev", "")
    pytest.fail(f"{template} declares no {hook_id!r} hook")


def test_botica_hook_comes_from_the_mirror_at_a_release_tag():
    """The hook is installed from the mirror, so a target needs no `botica` on its PATH."""
    repo, rev = _hook_source("prek-generic.toml", "botica")

    assert repo == BOTICA_MIRROR
    assert re.fullmatch(r"v\d+\.\d+\.\d+", rev), f"not a release tag: {rev!r}"


def test_botica_hook_runs_the_native_structure_rules():
    args = _hook_setting("prek-generic.toml", "botica", "args")

    assert isinstance(args, list), "the mirror's entry is fixed, so the rules go in `args`"
    assert "--no-astgrep" in args
    assert "--select" in args, f"botica would run every rule it knows: {args!r}"
    assert set(args[args.index("--select") + 1].split(",")) == BOTICA_RULES


def test_botica_hook_does_not_replace_the_entry_of_the_mirror():
    assert _hook_setting("prek-generic.toml", "botica", "entry") is None


def test_the_botica_hook_is_declared_once_so_the_joined_config_has_no_duplicate_id():
    """A Python repo gets the generic and the Python templates joined into one `prek.toml`."""
    joined = tomllib.loads(
        template_text("prek-generic.toml") + "\n" + template_text("prek-python.toml")
    )
    ids = [hook["id"] for repo in joined["repos"] for hook in repo.get("hooks", [])]

    assert ids.count("botica") == 1

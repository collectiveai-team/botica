"""Contract tests for the shipped prek hook templates."""

from __future__ import annotations

import re
import shlex
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

from scaffolding.templates_registry import template_text

if TYPE_CHECKING:
    from collections.abc import Mapping

BOTICA_MIRROR = "https://github.com/collectiveai-team/botica-pre-commit"
MIRROR_HOOKS = (
    Path(__file__).resolve().parent.parent
    / "lint/distribution/botica-pre-commit/.pre-commit-hooks.yaml"
)

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

# The rules the botica hook runs: CES-71, CES-63 and the layout rules (was `repo-shape`), and the
# syntax rules that the `ast-grep` hook used to run.
BOTICA_RULES = {
    "file-size-guard",
    "no-utils",
    "no-notebooks",
    "no-artifacts-in-package",
    "no-tests-in-package",
    "api-schemas-extra-forbid",
    "arch-database-package",
    "cli-typed-framework",
    "log-get-logger",
    "log-no-print",
    "no-dict-alias",
    "no-dict-call-return",
    "no-dict-literal-return",
    "no-dict-return-annotation",
    "settings-module",
}


@dataclass(frozen=True)
class DeclaredHook:
    """A hook as a shipped prek template declares it, with the `[[repos]]` table it sits in."""

    repo: str
    rev: str
    settings: Mapping[str, object]


def _declared_hook(template: str, hook_id: str) -> DeclaredHook:
    config = tomllib.loads(template_text(template))
    for repo in config.get("repos", []):
        for hook in repo.get("hooks", []):
            if hook.get("id") == hook_id:
                return DeclaredHook(repo["repo"], repo.get("rev", ""), hook)
    pytest.fail(f"{template} declares no {hook_id!r} hook")


def _hook_entry(template: str, hook_id: str) -> str:
    """Return the `entry` command of a hook declared in a shipped prek template."""
    return str(_declared_hook(template, hook_id).settings["entry"])


def _hook_setting(template: str, hook_id: str, key: str) -> object:
    """Return one setting of a hook declared in a shipped prek template (`None` if unset)."""
    return _declared_hook(template, hook_id).settings.get(key)


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


def test_botica_hook_comes_from_the_mirror_at_a_release_tag():
    """The hook is installed from the mirror, so a target needs no `botica` on its PATH."""
    hook = _declared_hook("prek-generic.toml", "botica")

    assert hook.repo == BOTICA_MIRROR
    assert re.fullmatch(r"v\d+\.\d+\.\d+", hook.rev), f"not a release tag: {hook.rev!r}"


def test_botica_hook_runs_the_rules_the_templates_ship():
    args = _hook_setting("prek-generic.toml", "botica", "args")

    assert isinstance(args, list), "the mirror's entry is fixed, so the rules go in `args`"
    assert "--no-astgrep" not in args, "the syntax rules are what the ast-grep hook used to run"
    assert "--select" in args, f"botica would run every rule it knows: {args!r}"
    assert set(args[args.index("--select") + 1].split(",")) == BOTICA_RULES


def _mirror_hook_block(hook_id: str) -> str:
    """Return the text of one hook in the mirror's `.pre-commit-hooks.yaml` (no YAML parser)."""
    text = MIRROR_HOOKS.read_text(encoding="utf-8")
    match = re.search(rf"^- id: {re.escape(hook_id)}$(.*?)(?=^- id: |\Z)", text, re.M | re.S)
    assert match, f"the mirror declares no {hook_id!r} hook"
    return match.group(1)


@pytest.mark.parametrize("hook_id", ["botica", "botica-project"])
def test_mirror_hooks_show_their_output_when_they_pass(hook_id: str):
    """Prek hides the output of a passing hook, and a size warning does not fail the hook."""
    assert re.search(r"^  verbose: true$", _mirror_hook_block(hook_id), re.M)


def test_mirror_file_hook_forces_the_exclusions_on_the_files_prek_names():
    """Prek names every file, and botica lints a named file even if `extend-exclude` has it."""
    assert "--force-exclude" in _mirror_hook_block("botica")


def test_botica_hook_does_not_replace_the_entry_of_the_mirror():
    assert _hook_setting("prek-generic.toml", "botica", "entry") is None


def test_the_botica_hook_is_declared_once_so_the_joined_config_has_no_duplicate_id():
    """A Python repo gets the generic and the Python templates joined into one `prek.toml`."""
    joined = tomllib.loads(
        template_text("prek-generic.toml") + "\n" + template_text("prek-python.toml")
    )
    ids = [hook["id"] for repo in joined["repos"] for hook in repo.get("hooks", [])]

    assert ids.count("botica") == 1


@pytest.mark.parametrize("template", ["prek-generic.toml", "prek-python.toml"])
def test_no_template_ships_an_ast_grep_hook(template: str):
    """The syntax rules run in the botica hook; the `ast-grep` hook and its config are retired."""
    config = tomllib.loads(template_text(template))
    ids = [hook["id"] for repo in config.get("repos", []) for hook in repo.get("hooks", [])]

    assert "ast-grep" not in ids

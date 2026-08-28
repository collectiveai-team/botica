"""`check` gating that keys off prek config contents."""

from __future__ import annotations

import subprocess
from typing import TYPE_CHECKING

import pytest

from scaffolding.checks import CheckResult, run_checks

if TYPE_CHECKING:
    from pathlib import Path

ASTGREP_CHECK = "ast-grep config"


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)
    return tmp_path


def _write_prek(repo: Path, body: str) -> None:
    (repo / "prek.toml").write_text(body, encoding="utf-8")


def _astgrep_result(repo: Path) -> CheckResult | None:
    """Return the ast-grep check, or None when it opted out of running at all."""
    return next((c for c in run_checks(repo) if c.name == ASTGREP_CHECK), None)


def test_astgrep_check_skipped_when_only_named_in_a_comment(repo: Path):
    """The generic prek.toml names ast-grep in a jscpd comment and ships no such hook.

    Substring-matching the raw file made every repo on the generic hook set fail this
    check, demanding an sgconfig.yml and rule dir it had no hook to run.
    """
    _write_prek(
        repo,
        "# jscpd: complementary to ast-grep (which matches a single declared pattern)\n"
        '[[repos]]\nrepo = "local"\n\n'
        '[[repos.hooks]]\nid = "jscpd"\nentry = "jscpd ."\nlanguage = "system"\n',
    )

    assert _astgrep_result(repo) is None


def test_astgrep_check_skipped_without_prek_config(repo: Path):
    assert _astgrep_result(repo) is None


def test_astgrep_check_skipped_when_prek_config_is_unparseable(repo: Path):
    """Malformed TOML must not resurrect the false positive via a text fallback."""
    _write_prek(repo, "this is not = = toml\n# ast-grep\n")

    assert _astgrep_result(repo) is None


def test_astgrep_check_fails_on_a_real_hook_without_config(repo: Path):
    _write_prek(
        repo,
        '[[repos]]\nrepo = "local"\n\n'
        '[[repos.hooks]]\nid = "ast-grep"\nentry = "ast-grep scan"\nlanguage = "system"\n',
    )

    result = _astgrep_result(repo)

    assert result is not None
    assert not result.ok


def test_astgrep_check_passes_with_sgconfig_and_rules(repo: Path):
    _write_prek(
        repo,
        '[[repos]]\nrepo = "local"\n'
        'hooks = [{ id = "ast-grep", entry = "ast-grep scan", language = "system" }]\n',
    )
    (repo / "sgconfig.yml").write_text("ruleDirs:\n  - ast-grep/rules\n", encoding="utf-8")
    rules = repo / "ast-grep" / "rules"
    rules.mkdir(parents=True)
    (rules / "example.yml").write_text(
        "id: example\nlanguage: python\nrule:\n  pattern: print($$$A)\n", encoding="utf-8"
    )

    result = _astgrep_result(repo)

    assert result is not None
    assert result.ok

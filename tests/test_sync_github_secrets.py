"""The sync script is the one place a secret value and the agent nearly meet.

Its tests therefore pin two things: the environment-suffix rule that stops a
missing integration secret falling back to a repository-scoped one, and the
absolute silence of the script about values.
"""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import pytest

SCRIPT = (
    Path(__file__).resolve().parent.parent
    / "skills/delivery/cloud-run-continuous-deploy/templates/sync-github-secrets.sh"
)

SENSITIVE = {"config": {"DATABASE_URL": {"value": "REDACTED"}, "API_KEY": {"value": "REDACTED"}}}


@pytest.fixture
def fake_bin(tmp_path: Path) -> tuple[Path, Path]:
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    calls = tmp_path / "gh-calls.txt"

    (bin_dir / "gh").write_text(
        "#!/usr/bin/env bash\n"
        'if [[ "$1" == "auth" ]]; then exit 0; fi\n'
        f'printf "%s\\n" "$*" >> {json.dumps(str(calls))}\n'
        "cat > /dev/null\n",
        encoding="utf-8",
    )
    (bin_dir / "varlock").write_text(
        "#!/usr/bin/env bash\n" f"printf '%s' {json.dumps(json.dumps(SENSITIVE))}\n",
        encoding="utf-8",
    )
    for name in ("gh", "varlock"):
        (bin_dir / name).chmod(0o755)
    return bin_dir, calls


def _run(bin_dir: Path, environment: str, values: dict[str, str]):
    env = dict(os.environ, PATH=f"{bin_dir}:{os.environ['PATH']}", **values)
    return subprocess.run(
        ["bash", str(SCRIPT), environment], env=env, capture_output=True, text=True, check=False
    )


def test_dev_uses_bare_secret_names(fake_bin):
    bin_dir, calls = fake_bin
    result = _run(bin_dir, "dev", {"DATABASE_URL": "postgres://x", "API_KEY": "k"})
    assert result.returncode == 0, result.stderr
    recorded = calls.read_text(encoding="utf-8")
    assert "secret set DATABASE_URL --env dev" in recorded
    assert "DATABASE_URL_DEV" not in recorded


def test_integration_suffixes_every_secret_name(fake_bin):
    bin_dir, calls = fake_bin
    result = _run(bin_dir, "integration", {"DATABASE_URL": "postgres://x", "API_KEY": "k"})
    assert result.returncode == 0, result.stderr
    recorded = calls.read_text(encoding="utf-8")
    assert "secret set DATABASE_URL_INTEGRATION --env integration" in recorded
    assert "secret set API_KEY_INTEGRATION --env integration" in recorded


def test_never_prints_a_value(fake_bin):
    bin_dir, _ = fake_bin
    result = _run(bin_dir, "dev", {"DATABASE_URL": "postgres://supersecret", "API_KEY": "k"})
    assert "supersecret" not in result.stdout
    assert "supersecret" not in result.stderr


def test_never_passes_a_value_in_argv(fake_bin):
    bin_dir, calls = fake_bin
    _run(bin_dir, "dev", {"DATABASE_URL": "postgres://supersecret", "API_KEY": "k"})
    assert "supersecret" not in calls.read_text(encoding="utf-8")


def test_fails_when_a_declared_secret_is_unset(fake_bin):
    bin_dir, _ = fake_bin
    result = _run(bin_dir, "integration", {"DATABASE_URL": "postgres://x"})
    assert result.returncode != 0
    assert "API_KEY" in result.stderr


def test_rejects_an_unknown_environment(fake_bin):
    bin_dir, _ = fake_bin
    result = _run(bin_dir, "staging", {"DATABASE_URL": "x", "API_KEY": "k"})
    assert result.returncode != 0
    assert "staging" in result.stderr

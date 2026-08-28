"""The inventory script must classify compose services without executing docker.

`docker compose config` is stubbed with a fake on PATH so the test pins the
classification rules, which is the part that carries judgment, rather than
docker's behavior, which does not need testing here.
"""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import pytest

SCRIPT = (
    Path(__file__).resolve().parent.parent
    / "skills/delivery/cloud-run-continuous-deploy/scripts/inventory-compose.sh"
)

COMPOSE = {
    "services": {
        "api": {"build": {"context": "./backend"}, "ports": ["8000:8000"]},
        "web": {"build": {"context": "./frontend"}, "ports": ["3000:3000"]},
        "worker": {"build": {"context": "./backend"}, "command": "python -m app.worker"},
        "db": {"image": "postgres:16", "volumes": ["pgdata:/var/lib/postgresql/data"]},
        "cache": {"image": "redis:7"},
        # Not a known stateful image, so this one exercises the named-volume
        # branch rather than the image-family lookup.
        "uploads": {"image": "acme/uploads:1", "volumes": ["updata:/data"]},
    }
}


@pytest.fixture
def fake_docker(tmp_path: Path) -> Path:
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    (bin_dir / "docker").write_text(
        f"#!/usr/bin/env bash\ncat {json.dumps(str(tmp_path / 'compose.json'))}\n",
        encoding="utf-8",
    )
    (bin_dir / "docker").chmod(0o755)
    (tmp_path / "compose.json").write_text(json.dumps(COMPOSE), encoding="utf-8")
    return bin_dir


def _run(tmp_path: Path, bin_dir: Path) -> str:
    env = dict(os.environ, PATH=f"{bin_dir}:{os.environ['PATH']}")
    result = subprocess.run(
        ["bash", str(SCRIPT)],
        cwd=tmp_path,
        env=env,
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout


def test_classifies_every_service(tmp_path: Path, fake_docker: Path):
    out = _run(tmp_path, fake_docker)
    assert "api" in out
    assert "http" in out
    assert "worker" in out
    assert "job" in out
    assert "db" in out
    assert "externalize" in out
    assert "cache" in out


def test_names_a_managed_equivalent_for_stateful_services(tmp_path: Path, fake_docker: Path):
    out = _run(tmp_path, fake_docker)
    assert "Cloud SQL" in out
    assert "Memorystore" in out


def test_flags_named_volumes_as_stateful(tmp_path: Path, fake_docker: Path):
    out = _run(tmp_path, fake_docker)
    assert "named volume" in out


def test_emits_a_worksheet_skeleton(tmp_path: Path, fake_docker: Path):
    out = _run(tmp_path, fake_docker)
    worksheet = out.split("--- worksheet ---", 1)[1]
    assert "topology: single-container" in worksheet
    assert "compose: api" in worksheet
    assert "role: http" in worksheet
    assert "review_tag_pattern:" in worksheet
    # Every key a template placeholder needs an answer for must appear here, or
    # the operator is never asked and the placeholder ships unsubstituted.
    # `integration_region` feeds {{INTEGRATION_REGION}} in deploy-integration.yml
    # and `bootstrap-gcp.sh --integration-region`; `reset_entrypoint` feeds
    # {{RESET_ENTRYPOINT}}.
    assert "integration_region:" in worksheet
    assert "reset_entrypoint:" in worksheet


def test_fails_clearly_without_docker(tmp_path: Path):
    empty = tmp_path / "empty"
    empty.mkdir()
    # Create symlinks to bash and python3 so the script can run, but no docker
    (empty / "bash").symlink_to("/bin/bash")
    (empty / "python3").symlink_to("/bin/python3")
    env = dict(os.environ, PATH=str(empty))
    result = subprocess.run(
        ["bash", str(SCRIPT)], cwd=tmp_path, env=env, capture_output=True, text=True, check=False
    )
    assert result.returncode != 0
    assert "docker" in result.stderr

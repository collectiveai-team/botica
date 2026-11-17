"""Consumer envelopes distinguish role selection from static policy validation."""

from __future__ import annotations

import copy
import json
import subprocess
from pathlib import Path
from typing import TypeAlias

import pytest
from jsonschema import Draft7Validator
from test_review_tier_schema import tier_binary as tier_binary

SCHEMA = Path(__file__).resolve().parents[1] / "schema"
ROLE = """---
role: spec
covered: {}
---
<!-- botica:covered -->
<!-- /botica:covered -->
"""
HOOK_ROLE = ROLE.replace("covered: {}", "covered:\n  hooks: [{file: prek.toml, id: lint}]")
Json: TypeAlias = str | int | float | bool | list["Json"] | dict[str, "Json"] | None


def _rewrite(value: Json, tier: bool = False) -> Json:
    if isinstance(value, list):
        return [_rewrite(item, tier) for item in value]
    if not isinstance(value, dict):
        return value
    result = {key: _rewrite(item, tier) for key, item in value.items()}
    reference = result.get("$ref")
    if isinstance(reference, str):
        if reference.startswith("review-tier.schema.json"):
            fragment = reference.removeprefix("review-tier.schema.json").removeprefix("#")
            result["$ref"] = f"#/definitions/tier_contract{fragment}"
        elif tier and reference.startswith("#/"):
            result["$ref"] = f"#/definitions/tier_contract{reference[1:]}"
    return result


def _bundled_schema() -> dict[str, Json]:
    """Bundle the local tier contract for offline standard JSON Schema validation."""
    consumer = _rewrite(json.loads((SCHEMA / "review-consumer.schema.json").read_text()))
    risk = _rewrite(json.loads((SCHEMA / "review-tier.schema.json").read_text()), tier=True)
    assert isinstance(consumer, dict)
    definitions = consumer["definitions"]
    assert isinstance(definitions, dict)
    definitions["tier_contract"] = risk
    return consumer


@pytest.mark.integration
@pytest.mark.parametrize(
    ("command", "path", "role", "expected_exit"),
    [
        ("plan", "docs/start.md", ROLE, 0),
        ("check", "docs/start.md", ROLE, 0),
        ("plan", "service.py", ROLE, 2),
        ("check", "service.py", ROLE, 0),
        ("plan", "docs/start.md", ROLE.replace("covered: {}", "unknown: true"), 2),
        ("check", "docs/start.md", ROLE.replace("covered: {}", ""), 2),
        ("plan", "docs/start.md", HOOK_ROLE, 2),
        ("check", "docs/start.md", HOOK_ROLE, 2),
    ],
)
def test_actual_consumer_reports_obey_the_contract(
    tmp_path: Path, tier_binary: Path, command: str, path: str, role: str, expected_exit: int
):
    def git(*args: str):
        subprocess.run(  # botica: ignore[test-no-subprocess] -- build CLI Git inputs
            ["git", *args], cwd=tmp_path, check=True, capture_output=True
        )

    def write(path: str, content: str):
        target = tmp_path / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")

    write("botica.toml", "[review]\nroles = ['spec']\n")
    write(".github/review/spec/REVIEW.md", role)
    if role == HOOK_ROLE:
        write("prek.toml", "[[repos]]\nrepo = 'local'\nhooks = [{id = 'lint'}]\n")
    git("init", "-b", "main")
    git("config", "user.name", "Consumer schema")
    git("config", "user.email", "schema@example.com")
    git("add", ".")
    git("commit", "-m", "Trusted policy")
    git("switch", "-c", "topic")
    write(path, "Content\n")
    git("add", ".")
    git("commit", "-m", "Head")
    write(".agents/skills/review-spec/SKILL.md", "Installed skill\n")
    output = subprocess.run(  # botica: ignore[test-no-subprocess] -- test the public CLI
        [
            str(tier_binary),
            "review",
            command,
            "--diff",
            "refs/heads/main",
            "--integration-method",
            "merge",
            "--format",
            "json",
        ],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        check=False,
    )
    assert output.returncode == expected_exit, output.stderr
    document = json.loads(output.stdout)
    schema = _bundled_schema()
    Draft7Validator.check_schema(schema)
    validator = Draft7Validator(schema)
    validator.validate(document)
    invalid = copy.deepcopy(document)
    if command == "check" or expected_exit == 2:
        invalid["selected_roles"] = ["spec"]
    else:
        del invalid["inputs"]["roles_digest"]
    assert not validator.is_valid(invalid)
    if command == "check" and expected_exit == 0:
        invalid = copy.deepcopy(document)
        invalid["roles"][0]["skill"] = None
        assert not validator.is_valid(invalid)

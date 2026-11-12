"""The schema of the findings validates the example of `output.md` and rejects a broken file."""

from __future__ import annotations

import json
import re
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

REPO_ROOT = Path(__file__).resolve().parent.parent
SCHEMA = REPO_ROOT / "schema" / "review-findings.schema.json"
OUTPUTS = sorted((REPO_ROOT / "skills" / "review").glob("*/references/output.md"))
README = REPO_ROOT / ".github" / "review" / "README.md"
JSON_BLOCK = re.compile(r"```json\n(.*?)\n```", re.DOTALL)
# `output.md` writes the three values of a field as one placeholder.
PLACEHOLDER = '"yes | no | not-tried"'


def _validator() -> Draft202012Validator:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    return Draft202012Validator(schema)


def _example_text(document: Path) -> str:
    blocks = JSON_BLOCK.findall(document.read_text(encoding="utf-8"))
    assert len(blocks) == 1, f"{document} must hold one json block"
    return blocks[0].replace(PLACEHOLDER, '"yes"')


def _errors(text: str) -> list[str]:
    return [error.message for error in _validator().iter_errors(json.loads(text))]


def _replace(text: str, old: str, new: str) -> str:
    assert old in text, f"the example has no {old}"
    return text.replace(old, new)


@pytest.mark.parametrize("output", OUTPUTS, ids=lambda path: path.parent.parent.name)
def test_the_example_of_output_md_is_valid(output: Path) -> None:
    assert _errors(_example_text(output)) == []


def test_there_is_an_output_file_for_each_role_and_the_panel() -> None:
    assert len(OUTPUTS) == 8


def test_the_example_of_the_readme_is_valid() -> None:
    assert _errors(_example_text(README)) == []


def test_refute_may_add_a_refutation_to_a_finding() -> None:
    text = _replace(_example_text(OUTPUTS[0]), '"fix":', '"refutation": "confirmed", "fix":')

    assert _errors(text) == []


@pytest.mark.parametrize(
    ("old", "new"),
    [
        ('"verdict": "FAIL"', '"verdict": "MAYBE"'),
        ('"severity": "P1"', '"severity": "P4"'),
        ('"line": 42', '"line": 0'),
        ('"line": 42', '"line": "42"'),
        ('"reproduced": "yes"', '"reproduced": "maybe"'),
        ('"fix": "One sentence; no patch."', '"refutation": "wrong"'),
        ('"fix":', '"extra": 1, "fix":'),
        ('"role": "security",', '"role": "security", "extra": 1,'),
        ('"role": "security"', '"role": "Security"'),
        ('"head": "2e12962"', '"head": "main"'),
        (
            '"commands": [{"run": "cargo test -p botica-cli", "result": "pass"}]',
            '"commands": [{"run": "x"}]',
        ),
        ('"fix": "One sentence; no patch."', '"fixed": "One sentence; no patch."'),
        ('"not_checked": ["what it did not review, and why"],', ""),
    ],
)
def test_the_schema_rejects(old: str, new: str) -> None:
    text = _replace(_example_text(OUTPUTS[0]), old, new)

    assert _errors(text) != []

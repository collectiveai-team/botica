"""The schema of the findings validates the example of `output.md` and rejects a broken file."""

from __future__ import annotations

import json
import re
from pathlib import Path

import pytest
from jsonschema import Draft7Validator

REPO_ROOT = Path(__file__).resolve().parent.parent
SCHEMA = REPO_ROOT / "schema" / "review-findings.schema.json"
OUTPUTS = sorted((REPO_ROOT / "skills" / "review").glob("*/references/output.md"))
README = REPO_ROOT / ".github" / "review" / "README.md"
JSON_BLOCK = re.compile(r"```json\n(.*?)\n```", re.DOTALL)
# `output.md` writes the three values of a field as one placeholder.
PLACEHOLDER = '"yes | no | not-tried"'
TOP_LEVEL_KEYS = [
    "role",
    "head",
    "base",
    "verdict",
    "findings",
    "checked",
    "not_checked",
    "commands",
]
FINDING_KEYS = [
    "id",
    "severity",
    "path",
    "line",
    "title",
    "evidence",
    "reproduced",
    "how_to_verify",
    "fix",
]


def _validator() -> Draft7Validator:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    Draft7Validator.check_schema(schema)
    return Draft7Validator(schema)


def _example_text(document: Path) -> str:
    blocks = JSON_BLOCK.findall(document.read_text(encoding="utf-8"))
    assert len(blocks) == 1, f"{document} must hold one json block"
    return blocks[0].replace(PLACEHOLDER, '"yes"')


def _example() -> str:
    return _example_text(OUTPUTS[0])


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


def test_the_output_files_are_copies_of_one_another() -> None:
    texts = {output.read_text(encoding="utf-8") for output in OUTPUTS}

    assert len(texts) == 1


def test_the_example_of_the_readme_is_valid() -> None:
    assert _errors(_example_text(README)) == []


@pytest.mark.parametrize(
    "refutation", ["confirmed", "pre-existing", "unreachable", "not-reproduced", "duplicate"]
)
def test_refute_may_add_a_refutation_to_a_finding(refutation: str) -> None:
    text = _replace(_example(), '"fix":', f'"refutation": "{refutation}", "fix":')

    assert _errors(text) == []


def test_a_finding_may_have_no_line() -> None:
    assert _errors(_replace(_example(), '"line": 42', '"line": null')) == []


@pytest.mark.parametrize("verdict", ["PASS", "FAIL", "INCONCLUSIVE"])
def test_the_three_verdicts_are_valid(verdict: str) -> None:
    assert _errors(_replace(_example(), '"verdict": "FAIL"', f'"verdict": "{verdict}"')) == []


@pytest.mark.parametrize("severity", ["P0", "P1", "P2", "P3"])
def test_the_four_severities_are_valid(severity: str) -> None:
    assert _errors(_replace(_example(), '"severity": "P1"', f'"severity": "{severity}"')) == []


@pytest.mark.parametrize("reproduced", ["yes", "no", "not-tried"])
def test_the_three_values_of_reproduced_are_valid(reproduced: str) -> None:
    text = _replace(_example(), '"reproduced": "yes"', f'"reproduced": "{reproduced}"')

    assert _errors(text) == []


@pytest.mark.parametrize("key", TOP_LEVEL_KEYS)
def test_the_schema_requires_each_top_level_key(key: str) -> None:
    review = json.loads(_example())
    del review[key]

    assert not _validator().is_valid(review)


@pytest.mark.parametrize("key", FINDING_KEYS)
def test_the_schema_requires_each_key_of_a_finding(key: str) -> None:
    review = json.loads(_example())
    del review["findings"][0][key]

    assert not _validator().is_valid(review)


@pytest.mark.parametrize("key", ["run", "result"])
def test_the_schema_requires_each_key_of_a_command(key: str) -> None:
    review = json.loads(_example())
    del review["commands"][0][key]

    assert not _validator().is_valid(review)


@pytest.mark.parametrize("key", ["id", "path", "title", "evidence"])
def test_the_schema_rejects_an_empty_string_in_a_finding(key: str) -> None:
    review = json.loads(_example())
    review["findings"][0][key] = ""

    assert not _validator().is_valid(review)


@pytest.mark.parametrize(
    ("old", "new"),
    [
        ('"verdict": "FAIL"', '"verdict": "MAYBE"'),
        ('"severity": "P1"', '"severity": "P4"'),
        ('"line": 42', '"line": 0'),
        ('"line": 42', '"line": "42"'),
        ('"reproduced": "yes"', '"reproduced": "maybe"'),
        ('"fix":', '"refutation": "wrong", "fix":'),
        ('"fix":', '"extra": 1, "fix":'),
        ('"role": "security",', '"role": "security", "extra": 1,'),
        ('"role": "security"', '"role": "Security"'),
        ('"head": "2e12962"', '"head": "main"'),
        ('"head": "2e12962"', '"head": "2E12962"'),
        ('"base": "main"', '"base": ""'),
        ('"result": "pass"', '"result": "pass", "extra": 1'),
    ],
)
def test_the_schema_rejects(old: str, new: str) -> None:
    assert _errors(_replace(_example(), old, new)) != []

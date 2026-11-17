"""The review-tier contract distinguishes a classification from partial evidence."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft7Validator

SCHEMA = Path(__file__).resolve().parents[1] / "schema" / "review-tier.schema.json"


def validator() -> Draft7Validator:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    Draft7Validator.check_schema(schema)
    return Draft7Validator(schema)


def _assessment(tier: str) -> str:
    return json.dumps(
        {
            "status": "complete",
            "tier": tier,
            "comparisons": [
                {
                    "from": "1" * 40,
                    "to": "2" * 40,
                    "tier": tier,
                    "files": ["docs/start.md"],
                    "areas": ["periphery"],
                    "signals": ["docs-only"],
                }
            ],
        }
    )


def _report() -> str:
    return json.dumps(
        {
            "schema_version": 1,
            "status": "complete",
            "inputs": {
                "base": "1" * 40,
                "head": "2" * 40,
                "integration_method": "merge",
                "botica_version": "0.20.2",
                "config": {"source": "botica.toml", "digest": "a" * 64},
            },
            "branch": json.loads(_assessment("low")),
            "integration": json.loads(_assessment("low")),
            "effective_tier": "low",
        }
    )


def test_an_incomplete_assessment_cannot_advertise_effective_low_risk():
    document = json.loads(_report())
    comparisons: list[object] = []
    document["status"] = "incomplete"
    document["integration"] = {
        "status": "incomplete",
        "reasons": [{"code": "merge-conflict", "message": "src/auth.py conflicts"}],
        "comparisons": comparisons,
    }

    assert not validator().is_valid(document)


@pytest.mark.parametrize(
    ("branch", "integration", "expected"),
    [
        ("low", "low", "low"),
        ("low", "medium", "medium"),
        ("low", "high", "high"),
        ("medium", "low", "medium"),
        ("medium", "medium", "medium"),
        ("medium", "high", "high"),
        ("high", "low", "high"),
        ("high", "medium", "high"),
        ("high", "high", "high"),
    ],
)
def test_the_effective_tier_is_the_maximum_of_both_assessments(
    branch: str, integration: str, expected: str
):
    document = json.loads(_report())
    document["branch"] = json.loads(_assessment(branch))
    document["integration"] = json.loads(_assessment(integration))
    document["effective_tier"] = expected
    contract = validator()

    assert contract.is_valid(document)
    for wrong in {"low", "medium", "high", None} - {expected}:
        document["effective_tier"] = wrong
        assert not contract.is_valid(document), wrong


@pytest.mark.parametrize("missing", [("branch",), ("integration",), ("branch", "integration")])
def test_partial_evidence_has_no_effective_tier_and_cannot_claim_completion(
    missing: tuple[str, ...],
):
    document = json.loads(_report())
    document["status"] = "incomplete"
    document["effective_tier"] = None
    for name in missing:
        document[name] = {
            "status": "incomplete",
            "reasons": [{"code": "missing-history", "message": "Fetch the complete history"}],
            "comparisons": [],
        }
    contract = validator()

    assert contract.is_valid(document)
    document["status"] = "complete"
    assert not contract.is_valid(document)


def test_an_assessment_cannot_hide_a_high_risk_ancestor_comparison():
    document = json.loads(_report())
    second = json.loads(_assessment("high"))["comparisons"][0]
    second["from"] = "3" * 40
    document["branch"]["comparisons"].append(second)

    assert not validator().is_valid(document)
    document["branch"]["tier"] = "high"
    document["effective_tier"] = "high"
    assert validator().is_valid(document)


@pytest.mark.parametrize("name", ["branch", "integration"])
def test_a_complete_assessment_requires_at_least_one_comparison(name: str):
    document = json.loads(_report())
    document[name]["comparisons"] = []

    assert not validator().is_valid(document)


@pytest.mark.parametrize("method", ["squash", "rebase"])
def test_unsupported_integration_methods_require_an_incomplete_result(method: str):
    document = json.loads(_report())
    comparisons: list[object] = []
    document["inputs"]["integration_method"] = method
    contract = validator()

    assert not contract.is_valid(document)
    document["status"] = "incomplete"
    document["effective_tier"] = None
    document["integration"] = {
        "status": "incomplete",
        "reasons": [
            {
                "code": "unsupported-integration-method",
                "message": f"{method} integration is not implemented",
            }
        ],
        "comparisons": comparisons,
    }
    assert contract.is_valid(document)


def test_partial_evidence_keeps_completed_comparisons_without_assigning_an_effective_tier():
    document = json.loads(_report())
    document["status"] = "incomplete"
    document["effective_tier"] = None
    document["branch"] = {
        "status": "incomplete",
        "reasons": [{"code": "unreadable-diff", "message": "One comparison could not be read"}],
        "comparisons": json.loads(_assessment("high"))["comparisons"],
    }
    contract = validator()

    assert contract.is_valid(document)
    document["branch"]["reasons"] = []
    assert not contract.is_valid(document)


@pytest.mark.parametrize("object_id", ["a" * 40, "b" * 64])
def test_commit_inputs_accept_git_sha1_and_sha256_object_ids(object_id: str):
    document = json.loads(_report())
    document["inputs"]["base"] = object_id

    assert validator().is_valid(document)


@pytest.mark.parametrize("object_id", ["main", "a" * 39, "b" * 65, "g" * 40])
def test_commit_inputs_require_full_hex_object_ids(object_id: str):
    document = json.loads(_report())
    document["inputs"]["base"] = object_id

    assert not validator().is_valid(document)


def test_the_old_top_level_tier_cannot_be_mistaken_for_an_effective_tier():
    document = json.loads(_report())
    document["tier"] = "low"

    assert not validator().is_valid(document)

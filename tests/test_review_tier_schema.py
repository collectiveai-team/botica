"""The review-tier contract distinguishes a classification from partial evidence."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pytest
from jsonschema import Draft7Validator

SCHEMA = Path(__file__).resolve().parents[1] / "schema" / "review-tier.schema.json"


def validator() -> Draft7Validator:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    Draft7Validator.check_schema(schema)
    return Draft7Validator(schema)


@pytest.fixture(scope="module")
def tier_binary() -> Path:
    root = SCHEMA.parents[1]
    subprocess.run(  # botica: ignore[test-no-subprocess] -- build the public CLI under test
        ["cargo", "build", "-q", "-p", "botica-cli"],
        cwd=root,
        check=True,
    )
    metadata = (
        subprocess.run(  # botica: ignore[test-no-subprocess] -- locate the public CLI under test
            ["cargo", "metadata", "--no-deps", "--format-version", "1"],
            cwd=root,
            check=True,
            capture_output=True,
            text=True,
        )
    )
    return Path(json.loads(metadata.stdout)["target_directory"]) / "debug" / "botica"


@pytest.mark.integration
@pytest.mark.parametrize(
    ("path", "content", "method", "expected_exit"),
    [
        ("docs/start.md", "Start\n", "merge", 0),
        ("docs/README.txt", "Read this\n", "merge", 0),
        ("docs/tool.rs", "pub fn answer() -> u8 { 42 }\n", "merge", 0),
        ("tools/environment-dev.txt", "dependency==1.0\n", "merge", 2),
        ("docs/start.md", "Start\n" * 1500, "merge", 0),
        ("docs/start.md", "Start\n", "squash", 2),
        ("package.json", '{"dependencies":{"new-package":"1.0.0"}}', "merge", 0),
        ("service.unknown", "Unknown endpoint syntax\n", "merge", 2),
        ("botica.toml", "[review]\nunknown = true\n", "merge", 2),
    ],
)
def test_runtime_json_satisfies_the_public_schema(
    tmp_path: Path,
    tier_binary: Path,
    path: str,
    content: str,
    method: str,
    expected_exit: int,
):
    def git(*args: str):
        return subprocess.run(  # botica: ignore[test-no-subprocess] -- build CLI Git inputs
            ["git", *args], cwd=tmp_path, check=True, capture_output=True
        )

    git("init", "-b", "main")
    git("config", "user.name", "Schema test")
    git("config", "user.email", "schema@example.com")
    target = tmp_path / path
    target.parent.mkdir(parents=True, exist_ok=True)
    if path == "botica.toml":
        target.write_text(content, encoding="utf-8")
        git("add", ".")
    git("commit", "--allow-empty", "-m", "Destination")
    git("switch", "-c", "topic")
    if path != "botica.toml":
        target.write_text(content, encoding="utf-8")
        git("add", ".")
        git("commit", "-m", "Head")
    output = subprocess.run(  # botica: ignore[test-no-subprocess] -- test the public CLI
        [
            str(tier_binary),
            "review",
            "tier",
            "--diff",
            "refs/heads/main",
            "--integration-method",
            method,
            "--format",
            "json",
        ],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        check=False,
    )
    assert output.returncode == expected_exit, output.stderr
    validator().validate(json.loads(output.stdout))


def test_null_config_digest_requires_both_invalid_config_assessments():
    document = json.loads(_report())
    document["inputs"]["config"]["digest"] = None
    contract = validator()
    assert not contract.is_valid(document)
    document["status"] = "incomplete"
    document["effective_tier"] = None
    for name in ["branch", "integration"]:
        document[name] = {
            "status": "incomplete",
            "reasons": [
                {"code": "invalid-config", "message": "A linked file has no configuration digest"}
            ],
            "comparisons": [],
        }
    assert contract.is_valid(document)


def test_size_evidence_is_informational_and_cannot_advertise_a_gate():
    document = json.loads(_report())
    size = document["branch"]["comparisons"][0]["size"]
    size["changed_lines"] = 100_000
    assert validator().is_valid(document)
    size["informational_only"] = False
    assert not validator().is_valid(document)


def test_complete_assessments_cannot_hide_unsupported_detector_evidence():
    document = json.loads(_report())
    document["branch"]["comparisons"][0]["detector_evidence"] = [
        {
            "detector": "unimplemented",
            "status": "unsupported",
            "paths": ["service.unknown"],
            "detail": "No endpoint detector supports this input",
        }
    ]
    assert not validator().is_valid(document)


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
                    "bounds": None,
                    "files": ["docs/start.md"],
                    "areas": ["periphery"],
                    "signals": ["docs-only"],
                    "evidence": [],
                    "signal_evidence": [],
                    "detector_evidence": [],
                    "size": {
                        "changed_lines": 1,
                        "non_text_files": [],
                        "informational_only": True,
                    },
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

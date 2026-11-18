"""An explicit standard set permits one delivery without weakening the default contract."""

import os
import subprocess
import sys
from pathlib import Path

import pytest
from test_check_rule_change import (
    add_rule,
    branch_that_touches_standards,
    crc,
    git,
)

PAIR = ("CES/122", "CES/134")


@pytest.mark.parametrize(
    ("approved", "expected"),
    [((), 1), (("CES/122",), 1), (PAIR, 0), ((*PAIR, "CES/135"), 1)],
)
def test_only_the_exact_acknowledged_set_permits_multiple_standards(
    tmp_path: Path,
    approved: tuple[str, ...],
    expected: int,
):
    branch_that_touches_standards(
        tmp_path,
        "standards/CES/122-process.md",
        "standards/CES/134-review.md",
    )
    assert crc.check_branch("main", tmp_path, acknowledged_standards=approved) == expected


def test_an_additional_standard_is_not_covered_by_a_prior_acknowledgment(tmp_path: Path):
    branch_that_touches_standards(
        tmp_path,
        "standards/CES/122-process.md",
        "standards/CES/134-review.md",
        "standards/CES/135-other.md",
    )
    assert crc.check_branch("main", tmp_path, acknowledged_standards=PAIR) == 1


@pytest.mark.parametrize("invalid", ["duplicate", "unnumbered"])
def test_an_acknowledged_set_cannot_waive_numbering_checks(tmp_path: Path, invalid: str):
    second = "134-other" if invalid == "duplicate" else "0-review"
    branch_that_touches_standards(
        tmp_path, "standards/CES/134-review.md", f"standards/CES/{second}.md"
    )
    allowed = ("CES/134",) if invalid == "duplicate" else ("CES/0", "CES/134")
    assert crc.check_branch("main", tmp_path, acknowledged_standards=allowed) == 1


def test_standard_acknowledgment_does_not_acknowledge_unrelated_rule_changes(tmp_path: Path):
    branch_that_touches_standards(
        tmp_path,
        "standards/CES/122-process.md",
        "standards/CES/134-review.md",
    )
    add_rule(tmp_path, "catalog/rules", "structure", "no-foo", "ast-grep")
    (tmp_path / "unrelated.py").write_text("value = 1\n")
    git(tmp_path, "add", "-A")
    git(tmp_path, "commit", "-qm", "Rule and unrelated source")
    assert crc.check_branch("main", tmp_path, acknowledged_standards=PAIR) == 1
    assert (
        crc.check_branch(
            "main",
            tmp_path,
            acknowledged_standards=PAIR,
            acknowledged=True,
        )
        == 0
    )


def test_the_public_cli_accepts_a_repeated_exact_standard_acknowledgment(tmp_path: Path):
    branch_that_touches_standards(
        tmp_path,
        "standards/CES/122-process.md",
        "standards/CES/134-review.md",
    )
    script = Path(__file__).resolve().parents[1] / "scripts/check_rule_change.py"
    result = subprocess.run(  # botica: ignore[test-no-subprocess] -- public contract CLI
        [
            sys.executable,
            str(script),
            "main",
            "--acknowledged-standard",
            "CES/122",
            "--acknowledged-standard",
            "CES/134",
        ],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "Acknowledged standard set: CES/122, CES/134" in result.stdout


@pytest.mark.parametrize("approved", ["true", "false"])
def test_ci_supplies_only_the_delivery_specific_pair_after_explicit_label_acknowledgment(
    tmp_path: Path,
    approved: str,
):
    workflow = (Path(__file__).resolve().parents[1] / ".github/workflows/ci.yml").read_text()
    step = workflow.split(
        "- name: Check the contract of the rules and the standards the change touches\n", 1
    )[1]
    body = step.split("        run: |\n", 1)[1].split("\n  #", 1)[0]
    script = "\n".join(line[10:] for line in body.splitlines())
    binary = tmp_path / "bin"
    binary.mkdir()
    uv = binary / "uv"
    uv.write_text('#!/bin/sh\nprintf "%s\\n" "$@" > "$ARGUMENTS"\n')
    uv.chmod(0o755)
    arguments = tmp_path / "arguments"
    result = (
        subprocess.run(  # botica: ignore[test-no-subprocess] -- execute the actual contract step
            ["bash", "-eo", "pipefail", "-c", script],
            cwd=tmp_path,
            env={
                **os.environ,
                "PATH": f"{binary}:{os.environ['PATH']}",
                "BASE": "origin/main",
                "ACKNOWLEDGED": "true",
                "DRAFT": "false",
                "REVIEW_FLOW_STANDARDS": approved,
                "ARGUMENTS": str(arguments),
            },
            capture_output=True,
            text=True,
            check=False,
        )
    )
    assert result.returncode == 0, result.stderr
    actual = arguments.read_text().splitlines()
    assert "--acknowledged" in actual
    expected = ["--acknowledged-standard", "CES/122", "--acknowledged-standard", "CES/134"]
    if approved == "true":
        assert actual[-4:] == expected
    else:
        assert "--acknowledged-standard" not in actual

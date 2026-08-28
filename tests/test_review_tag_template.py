"""The review-tag template ships verbatim into target repos, so it is tested here.

It is loaded by path rather than imported, because `skills/` is not a package and
the file's runtime home is another repository entirely.
"""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

import pytest

TEMPLATE = (
    Path(__file__).resolve().parent.parent
    / "skills/delivery/cloud-run-continuous-deploy/templates/review_tag.py"
)


def _load():
    import sys

    spec = importlib.util.spec_from_file_location("review_tag_template", TEMPLATE)
    assert spec is not None, f"could not build an import spec for {TEMPLATE}"
    assert spec.loader is not None, f"import spec for {TEMPLATE} has no loader"

    module = importlib.util.module_from_spec(spec)
    # Registering the module is required, not incidental: review_tag.py uses
    # `from __future__ import annotations`, so @dataclass resolves its string
    # annotations through sys.modules[cls.__module__]. exec_module alone does
    # not register it, and the dataclass then fails to build.
    sys.modules["review_tag_template"] = module
    spec.loader.exec_module(module)
    return module


review_tag = _load()


def _pr_payload(**overrides):
    payload = {
        "state": "open",
        "draft": False,
        "base": {"ref": "dev"},
        "head": {"sha": "abcdef123456789012345678901234567890abcd"},
    }
    payload.update(overrides)
    return payload


def _runner(payload, git_sha="abcdef123456789012345678901234567890abcd"):
    def run(cmd):
        if cmd[0] == "gh":
            return json.dumps(payload)
        if cmd[0] == "git":
            return git_sha + "\n"
        raise AssertionError(f"unexpected command: {cmd}")

    return run


def test_parse_review_tag():
    parsed = review_tag.parse_review_tag("review/pr-123/abcdef123456")
    assert parsed.pr_number == 123
    assert parsed.short_sha == "abcdef123456"


@pytest.mark.parametrize(
    "tag",
    [
        "review/123/abcdef1",
        "review/pr-x/abcdef1",
        "review/pr-1/nope",
        "review/pr-0/abcdef1",
        "review/pr-1/ABCDEF1",
        "refs/tags/review/pr-1/abcdef1",
    ],
)
def test_rejects_invalid_review_tag(tag):
    with pytest.raises(ValueError, match="review/pr"):
        review_tag.parse_review_tag(tag)


def test_validate_accepts_open_pr_at_tagged_head():
    parsed, head_sha = review_tag.validate(
        "review/pr-7/abcdef123456", "org/repo", "dev", run=_runner(_pr_payload())
    )
    assert parsed.pr_number == 7
    assert head_sha == "abcdef123456789012345678901234567890abcd"


def test_validate_rejects_stale_tag():
    run = _runner(_pr_payload(), git_sha="0000000000000000000000000000000000000000")
    with pytest.raises(ValueError, match="does not point at"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_closed_pr():
    run = _runner(_pr_payload(state="closed"))
    with pytest.raises(ValueError, match="not open"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_draft_pr():
    run = _runner(_pr_payload(draft=True))
    with pytest.raises(ValueError, match="draft"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_wrong_base():
    run = _runner(_pr_payload(base={"ref": "main"}))
    with pytest.raises(ValueError, match="base branch"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_short_sha_that_is_not_a_head_prefix():
    run = _runner(_pr_payload(head={"sha": "9999999999999999999999999999999999999999"}))
    with pytest.raises(ValueError, match="head"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_malformed_api_response():
    def run(cmd):
        return "not json" if cmd[0] == "gh" else "abc\n"

    with pytest.raises(ValueError, match="could not read pull request"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_api_response_with_array():
    run = _runner([])
    with pytest.raises(ValueError, match="could not read pull request"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_api_response_with_null():
    run = _runner(None)
    with pytest.raises(ValueError, match="could not read pull request"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)


def test_validate_rejects_api_response_with_non_object_head():
    run = _runner(_pr_payload(head="not-a-dict"))
    with pytest.raises(ValueError, match="head"):
        review_tag.validate("review/pr-7/abcdef123456", "org/repo", "dev", run=run)

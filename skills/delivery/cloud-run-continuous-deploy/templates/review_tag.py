"""Parse and validate immutable PR review tags.

A review tag names exactly one pull-request head:

    review/pr-<number>/<short-sha>

Validation is not a formality. Anyone who can push a tag can create one, and the
trusted deployment workflow uses the result to choose which commit to build. It
must therefore prove against both the GitHub API and git that the tag points at
the head of an open, non-draft pull request targeting the trusted base branch.

Stdlib only: this file ships into repositories that may install nothing.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from collections.abc import Callable
from dataclasses import dataclass

TAG_PATTERN = re.compile(r"^review/pr-([1-9][0-9]*)/([0-9a-f]{7,40})$")


@dataclass(frozen=True)
class ReviewTag:
    pr_number: int
    short_sha: str


def parse_review_tag(tag: str) -> ReviewTag:
    match = TAG_PATTERN.match(tag)
    if match is None:
        raise ValueError(
            f"not a review/pr tag: {tag!r}; expected review/pr-<number>/<short-sha> "
            "with a lowercase hex sha of 7 to 40 characters"
        )
    return ReviewTag(pr_number=int(match.group(1)), short_sha=match.group(2))


def _run(cmd: list[str]) -> str:
    return subprocess.run(cmd, capture_output=True, text=True, check=True).stdout


def validate(
    tag: str,
    repo: str,
    base: str,
    run: Callable[[list[str]], str] = _run,
) -> tuple[ReviewTag, str]:
    """Return (parsed tag, head sha) or raise ValueError naming the failure.

    `run` is injected so the checks are testable without a network or a git repo.
    Nothing here prints the API response: it carries tokens in no field today, but
    an unfiltered dump of a future schema is exactly how a secret leaks into a log.
    """
    parsed = parse_review_tag(tag)

    try:
        payload = json.loads(run(["gh", "api", f"repos/{repo}/pulls/{parsed.pr_number}"]))
    except (subprocess.CalledProcessError, json.JSONDecodeError, OSError) as exc:
        raise ValueError(
            f"could not read pull request {parsed.pr_number} in {repo}"
        ) from exc

    if not isinstance(payload, dict):
        raise ValueError(
            f"could not read pull request {parsed.pr_number} in {repo}"
        )

    if payload.get("state") != "open":
        raise ValueError(f"pull request {parsed.pr_number} is not open")
    if payload.get("draft"):
        raise ValueError(f"pull request {parsed.pr_number} is a draft")

    base_obj = payload.get("base")
    if not isinstance(base_obj, dict):
        raise ValueError(
            f"could not read pull request {parsed.pr_number} in {repo}"
        )
    actual_base = base_obj.get("ref")
    if actual_base != base:
        raise ValueError(
            f"pull request {parsed.pr_number} targets base branch {actual_base!r}, "
            f"expected {base!r}"
        )

    head_obj = payload.get("head")
    if not isinstance(head_obj, dict):
        raise ValueError(
            f"pull request {parsed.pr_number} head is malformed"
        )
    head_sha = head_obj.get("sha") or ""
    if not head_sha.startswith(parsed.short_sha):
        raise ValueError(
            f"tag short sha {parsed.short_sha!r} is not a prefix of the pull "
            f"request head {head_sha!r}"
        )

    try:
        target = run(["git", "rev-parse", f"{tag}^{{commit}}"]).strip()
    except (subprocess.CalledProcessError, OSError) as exc:
        raise ValueError(f"could not resolve tag {tag!r} to a commit") from exc

    if target != head_sha:
        raise ValueError(
            f"tag {tag!r} does not point at the pull request head; "
            f"tag resolves to {target!r}, head is {head_sha!r}"
        )

    return parsed, head_sha


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    validate_parser = sub.add_parser("validate")
    validate_parser.add_argument("--tag", required=True)
    validate_parser.add_argument("--repo", required=True)
    validate_parser.add_argument("--base", required=True)
    validate_parser.add_argument("--emit-github-output", action="store_true")
    args = parser.parse_args(argv)

    try:
        parsed, head_sha = validate(args.tag, args.repo, args.base)
    except ValueError as exc:
        print(f"review_tag: {exc}", file=sys.stderr)
        return 1

    print(f"review_tag: {args.tag} validated against PR #{parsed.pr_number}")

    if args.emit_github_output:
        output_path = os.environ.get("GITHUB_OUTPUT")
        if not output_path:
            print("review_tag: GITHUB_OUTPUT is not set", file=sys.stderr)
            return 1
        with open(output_path, "a", encoding="utf-8") as handle:
            handle.write(f"pr_number={parsed.pr_number}\n")
            handle.write(f"head_sha={head_sha}\n")
            handle.write(f"short_sha={head_sha[:12]}\n")
            handle.write(f"review_tag={args.tag}\n")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

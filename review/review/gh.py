"""Thin wrapper over the `gh` CLI for proposal-issue review.

All state lives in GitHub; this module shells out to `gh` and normalises the label
vocabulary (state:* / area:* / enforcer:* / priority:*) into typed dataclasses.
"""

from __future__ import annotations

import json
import os
import subprocess
from dataclasses import dataclass, field

REPO = os.environ.get("REVIEW_REPO", "collectiveai-team/botica")

STATE_LABELS = ["state:proposal", "state:had-comments", "state:approved", "state:declined"]
PRIORITY_RANK = {"high": 0, "medium": 1, "low": 2}


class GhError(RuntimeError):
    """A `gh` invocation failed."""


@dataclass
class Issue:
    """A proposal issue, normalised from gh's JSON + label vocabulary."""

    number: int
    title: str
    is_open: bool
    labels: list[str]
    type: str
    state: str
    priority: str
    area: str
    enforcers: list[str] = field(default_factory=list)
    updated_at: str = ""
    comment_count: int = 0

    @property
    def sort_key(self) -> tuple[int, int]:
        return (PRIORITY_RANK.get(self.priority, 1), self.number)

    @property
    def enforcer_text(self) -> str:
        return ",".join(self.enforcers)

    @property
    def updated_short(self) -> str:
        """Date portion of the ISO updatedAt timestamp (sortable, compact)."""
        return self.updated_at[:10]


@dataclass
class Comment:
    """A single issue comment."""

    author: str
    when: str
    body: str


@dataclass
class IssueDetail:
    """An issue body plus its comment thread."""

    body: str
    comments: list[Comment] = field(default_factory=list)


def _run(args: list[str], inp: str | None = None) -> str:
    proc = subprocess.run(["gh", *args], text=True, capture_output=True, input=inp, check=False)
    if proc.returncode != 0:
        raise GhError(proc.stderr.strip() or f"gh {' '.join(args)} failed")
    return proc.stdout


def _label_value(labels: list[str], prefix: str) -> str:
    for name in labels:
        if name.startswith(prefix):
            return name[len(prefix) :]
    return ""


def _label_values(labels: list[str], prefix: str) -> list[str]:
    return [name[len(prefix) :] for name in labels if name.startswith(prefix)]


def normalise(raw: dict) -> Issue:
    """Build an Issue from one gh JSON record."""
    labels = [lab["name"] for lab in raw.get("labels", [])]
    return Issue(
        number=raw["number"],
        title=raw["title"],
        is_open=raw.get("state", "OPEN").upper() == "OPEN",
        labels=labels,
        type=_label_value(labels, "type:"),
        state=_label_value(labels, "state:") or "proposal",
        priority=_label_value(labels, "priority:") or "medium",
        area=_label_value(labels, "area:"),
        enforcers=_label_values(labels, "enforcer:"),
        updated_at=raw.get("updatedAt", ""),
        comment_count=len(raw.get("comments") or []),
    )


def list_issues(repo: str = REPO, limit: int = 300) -> list[Issue]:
    out = _run(
        [
            "issue",
            "list",
            "--repo",
            repo,
            "--state",
            "all",
            "--limit",
            str(limit),
            "--json",
            "number,title,labels,state,updatedAt,comments",
        ]
    )
    issues = [normalise(r) for r in json.loads(out)]
    issues.sort(key=lambda i: i.sort_key)  # priority-first default; the TUI re-sorts on demand
    return issues


def issue_detail(number: int, repo: str = REPO) -> IssueDetail:
    out = _run(["issue", "view", str(number), "--repo", repo, "--json", "body,comments"])
    raw = json.loads(out)
    comments = [
        Comment(
            author=(c.get("author") or {}).get("login", "?"),
            when=(c.get("createdAt") or "")[:10],
            body=(c.get("body") or "").strip(),
        )
        for c in raw.get("comments") or []
    ]
    return IssueDetail(body=(raw.get("body") or "").strip(), comments=comments)


def add_comment(number: int, body: str, repo: str = REPO) -> None:
    _run(["issue", "comment", str(number), "--repo", repo, "--body-file", "-"], inp=body)


def set_state(issue: Issue, new_state: str, repo: str = REPO) -> None:
    """Swap the state:* label and open/close to match (declined = closed)."""
    target = f"state:{new_state}"
    to_remove = [name for name in issue.labels if name in STATE_LABELS and name != target]
    args = ["issue", "edit", str(issue.number), "--repo", repo, "--add-label", target]
    for name in to_remove:
        args += ["--remove-label", name]
    _run(args)
    if new_state == "declined" and issue.is_open:
        _run(["issue", "close", str(issue.number), "--repo", repo])
    elif new_state != "declined" and not issue.is_open:
        _run(["issue", "reopen", str(issue.number), "--repo", repo])


def web_url(number: int, repo: str = REPO) -> str:
    return f"https://github.com/{repo}/issues/{number}"

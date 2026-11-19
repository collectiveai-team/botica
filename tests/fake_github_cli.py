"""Working GitHub CLI adapter for workflow tests, with state shared across subprocesses."""

from __future__ import annotations

import json
import os
import shlex
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path


@dataclass
class Comment:
    author: str
    body: str
    id: int = 1


@dataclass
class GitHubState:
    labels: set[str]
    comments: list[Comment] = field(default_factory=list)
    available_labels: set[str] = field(default_factory=set)
    events: list[dict[str, str]] = field(default_factory=list)
    base: str = "b" * 40
    head: str = "a" * 40
    number: int = 42

    def __post_init__(self) -> None:
        self.available_labels.update(self.labels)

    def save(self, path: Path) -> None:
        path.write_text(
            json.dumps(
                {
                    "labels": sorted(self.labels),
                    "available_labels": sorted(self.available_labels),
                    "comments": [asdict(comment) for comment in self.comments],
                    "events": self.events,
                    "base": self.base,
                    "head": self.head,
                    "number": self.number,
                }
            )
        )

    @classmethod
    def load(cls, path: Path) -> GitHubState:
        stored = json.loads(path.read_text())
        return cls(
            labels=set(stored["labels"]),
            available_labels=set(stored["available_labels"]),
            comments=[Comment(**comment) for comment in stored["comments"]],
            events=stored["events"],
            base=stored["base"],
            head=stored["head"],
            number=stored["number"],
        )

    def run(self, args: list[str]) -> str:
        if args[:2] == ["label", "create"]:
            self.available_labels.add(args[2])
            return ""
        if args[0] == "api":
            return self._comments(args)
        if args[:1] != ["pr"] or args[2] != str(self.number):
            raise ValueError("Unknown pull request")
        if args[1] == "view":
            return self._view(args)
        if args[1] == "edit":
            self._edit(args[3:])
            return ""
        raise ValueError("Unsupported GitHub command")

    def _comments(self, args: list[str]) -> str:
        if "--method" in args and args[args.index("--method") + 1] != "GET":
            raise ValueError("Unsupported API method")
        endpoint = next(value for value in args if value.startswith("repos/"))
        if endpoint.split("?", 1)[0] != f"repos/owner/repo/issues/{self.number}/comments":
            raise ValueError("Unknown comments endpoint")
        return json.dumps(
            [
                {
                    "id": comment.id,
                    "user": {"login": comment.author},
                    "body": comment.body,
                }
                for comment in self.comments
            ]
        )

    def _view(self, args: list[str]) -> str:
        fields = args[args.index("--json") + 1]
        query = args[args.index("--jq") + 1]
        if fields == "labels" and query == ".labels[].name":
            return "\n".join(sorted(self.labels))
        if fields == "baseRefOid,headRefOid" and query == '[.baseRefOid,.headRefOid] | join(":")':
            return f"{self.base}:{self.head}"
        raise ValueError("Unsupported pull request fields")

    def _edit(self, args: list[str]) -> None:
        """Change the labels; like GitHub, only a change of the set records a timeline event."""
        for option, label in zip(args[::2], args[1::2], strict=True):
            if option == "--remove-label":
                if label in self.labels:
                    self.labels.remove(label)
                    self.events.append({"event": "unlabeled", "label": label})
            elif option == "--add-label":
                if label not in self.available_labels:
                    raise ValueError("A label must exist before it can be attached")
                if label not in self.labels:
                    self.labels.add(label)
                    self.events.append({"event": "labeled", "label": label})
            else:
                raise ValueError("Unsupported pull request edit")


def install(directory: Path, state: GitHubState) -> tuple[Path, Path]:
    """Expose the CLI port while persisting its in-memory model between process invocations."""
    binary = directory / "bin"
    binary.mkdir()
    path = directory / "github-state.json"
    state.save(path)
    entrypoint = binary / "gh"
    entrypoint.write_text(
        f"#!/bin/sh\nexec {shlex.quote(sys.executable)} "
        f'{shlex.quote(str(Path(__file__).resolve()))} "$@"\n',
    )
    entrypoint.chmod(0o755)
    return binary, path


if __name__ == "__main__":
    state_path = Path(os.environ["GITHUB_STATE"])
    github = GitHubState.load(state_path)
    result = github.run(sys.argv[1:])
    github.save(state_path)
    if result:
        print(result)

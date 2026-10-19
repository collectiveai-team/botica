"""Seal the status of every rule of the catalog in its release history.

`catalog/rules/history.toml` says, for each release, which rules were added and which ones changed
status. Nobody writes it by hand: the author of a change sets the `status` of a rule, and the
release runs `stamp` with its own version, which appends one event for each rule whose status the
history does not have yet. `check` fails while a status is still unsealed, so a release cannot go
out with a change that no version records.
"""

from __future__ import annotations

import re
import tomllib
from dataclasses import dataclass
from pathlib import Path

import cyclopts

app = cyclopts.App(name="stamp-release", help=__doc__)

CATALOG = Path("catalog/rules")
HISTORY = CATALOG / "history.toml"
VERSION = re.compile(r"^\d+\.\d+\.\d+$")


@dataclass(frozen=True)
class Change:
    """A status of a rule that no release has recorded yet."""

    rule: str
    status: str


@dataclass(frozen=True)
class Event:
    """What one release did to the status of one rule: an `[[event]]` of the history."""

    version: str
    rule: str
    status: str


StatusTable = dict[str, str]


def _version(text: str) -> tuple[int, ...]:
    return tuple(int(part) for part in text.split("."))


def _events(root: Path) -> list[Event]:
    path = root / HISTORY
    if not path.is_file():
        return []
    events = tomllib.loads(path.read_text(encoding="utf-8")).get("event", [])
    return [Event(event["version"], event["rule"], event["status"]) for event in events]


def _statuses(root: Path) -> StatusTable:
    """Return the status of each rule as its `rule.toml` says it, by slug."""
    found: StatusTable = {}
    for rule_toml in (root / CATALOG).glob("*/*/rule.toml"):
        data = tomllib.loads(rule_toml.read_text(encoding="utf-8"))
        found[data["id"]] = data["status"]
    return found


def unsealed(root: Path) -> list[Change]:
    """List the statuses that the history does not have as the last of their rule."""
    recorded = {event.rule: event.status for event in _events(root)}
    return [
        Change(rule, status)
        for rule, status in sorted(_statuses(root).items())
        if recorded.get(rule) != status
    ]


def _check_version(root: Path, version: str) -> None:
    if not VERSION.match(version):
        raise ValueError(f"version `{version}` must look like `0.10.0`")
    events = _events(root)
    if events and _version(version) < _version(events[-1].version):
        raise ValueError(f"version `{version}` is older than the last event of the history")


def stamp(root: Path, version: str) -> list[Change]:
    """Append to the history the unsealed statuses under `version`; return what was sealed."""
    _check_version(root, version)
    changes = unsealed(root)
    if not changes:
        return []
    path = root / HISTORY
    text = path.read_text(encoding="utf-8") if path.is_file() else ""
    blocks = [
        f'[[event]]\nversion = "{version}"\nrule = "{change.rule}"\nstatus = "{change.status}"\n'
        for change in changes
    ]
    separator = "" if not text or text.endswith("\n\n") else "\n"
    path.write_text(text + separator + "\n".join(blocks), encoding="utf-8")
    return changes


def check(root: Path) -> int:
    """Return 1 and name the statuses that no release has sealed, 0 when there are none."""
    changes = unsealed(root)
    for change in changes:
        print(f"{change.rule}: {change.status} is not in {HISTORY}")
    if changes:
        print("Run `scripts/stamp_release.py stamp <version>` as part of the release.")
    return 1 if changes else 0


@app.command(name="stamp")
def stamp_command(version: str) -> None:
    """Seal the unsealed statuses under VERSION."""
    for change in stamp(Path.cwd(), version):
        print(f"{version}: {change.rule} is {change.status}")


@app.command(name="check")
def check_command() -> None:
    """Fail while a status of the catalog is not sealed in the history."""
    raise SystemExit(check(Path.cwd()))


if __name__ == "__main__":
    app()

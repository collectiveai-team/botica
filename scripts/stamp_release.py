"""Seal the status, the code and the id of every rule of the catalog in its release history.

`catalog/rules/history.toml` says, for each release, which rules were added, which ones changed
status or code, and which ones were renamed (ADR 0016). Nobody writes it by hand: the author of a
change sets the `status` or the `code` of a rule, or gives it a new `id` and keeps the old one in
`aliases`, and the release runs `stamp` with its own version, which appends one event for each rule
whose status, code or id the history does not have yet. `check` fails while one is still unsealed,
so a release cannot go out with a change that no version records.
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
class Event:
    """What one release did to one rule: an `[[event]]` of the history."""

    version: str
    rule: str
    status: str | None = None
    code: str | None = None
    renamed_from: str | None = None


@dataclass(frozen=True)
class Change:
    """What the history does not have yet about one rule: the fields of the event it needs."""

    rule: str
    status: str | None = None
    code: str | None = None
    renamed_from: str | None = None


@dataclass(frozen=True)
class Rule:
    """The fields of a `rule.toml` that the history records."""

    id: str
    status: str
    code: str | None
    aliases: tuple[str, ...]


def _version(text: str) -> tuple[int, ...]:
    return tuple(int(part) for part in text.split("."))


def read_events(root: Path) -> list[Event]:
    path = root / HISTORY
    if not path.is_file():
        return []
    events = tomllib.loads(path.read_text(encoding="utf-8")).get("event", [])
    return [
        Event(
            event["version"],
            event["rule"],
            event.get("status"),
            event.get("code"),
            event.get("renamed-from"),
        )
        for event in events
    ]


def _rules(root: Path) -> list[Rule]:
    """Return each rule as its `rule.toml` says it, in the order of their ids."""
    found = []
    for rule_toml in (root / CATALOG).glob("*/*/rule.toml"):
        data = tomllib.loads(rule_toml.read_text(encoding="utf-8"))
        found.append(
            Rule(data["id"], data["status"], data.get("code"), tuple(data.get("aliases", [])))
        )
    return sorted(found, key=lambda rule: rule.id)


IdTable = dict[str, str]


def current_ids(events: list[Event]) -> IdTable:
    """Return the id that each former id of a rule became, following every rename."""
    renamed: IdTable = {}
    for event in events:
        if event.renamed_from:
            for old, new in renamed.items():
                if new == event.renamed_from:
                    renamed[old] = event.rule
            renamed[event.renamed_from] = event.rule
    return renamed


def _renamed_from(rule: Rule, events: list[Event], current: IdTable) -> str | None:
    """Return the alias of `rule` that the history names as the id of a rule not renamed yet."""
    named = {event.rule for event in events}
    if rule.id in named:
        return None
    return next((a for a in rule.aliases if a in named and a not in current), None)


def _change(rule: Rule, events: list[Event], current: IdTable) -> Change | None:
    ids = {rule.id, *(old for old, new in current.items() if new == rule.id)}
    own = [event for event in events if event.rule in ids]
    renamed_from = _renamed_from(rule, events, current)
    if renamed_from:
        own += [event for event in events if event.rule == renamed_from]
    status = next((e.status for e in reversed(own) if e.status), None)
    code = next((e.code for e in reversed(own) if e.code), None)
    change = Change(
        rule.id,
        rule.status if status != rule.status else None,
        rule.code if rule.code and code != rule.code else None,
        renamed_from,
    )
    return change if change != Change(rule.id) else None


def unsealed(root: Path) -> list[Change]:
    """List what the history does not have yet about each rule, in the order of their ids."""
    events = read_events(root)
    current = current_ids(events)
    changes = (_change(rule, events, current) for rule in _rules(root))
    return [change for change in changes if change]


def _owners(events: list[Event]) -> dict[str, set[str]]:
    """Return, for each code and id of the history, the rules it named, by their current id."""
    current = current_ids(events)
    owners: dict[str, set[str]] = {}
    for event in events:
        rule = current.get(event.rule, event.rule)
        for name in (event.rule, event.code, event.renamed_from):
            if name:
                owners.setdefault(name, set()).add(rule)
    return owners


def _check_reuse(root: Path, changes: list[Change]) -> None:
    owners = _owners(read_events(root))
    for change in changes:
        for name in (change.rule, change.code):
            others = owners.get(name or "", set()) - {change.rule}
            if others:
                raise ValueError(
                    f"`{name}` already names `{sorted(others)[0]}` in the history: a code or an id "
                    "that a release published never names another rule (ADR 0016)"
                )


def _check_version(root: Path, version: str) -> None:
    if not VERSION.match(version):
        raise ValueError(f"version `{version}` must look like `0.10.0`")
    events = read_events(root)
    if events and _version(version) < _version(events[-1].version):
        raise ValueError(f"version `{version}` is older than the last event of the history")


def _block(version: str, change: Change) -> str:
    lines = [f'version = "{version}"', f'rule = "{change.rule}"']
    for key, value in (
        ("renamed-from", change.renamed_from),
        ("status", change.status),
        ("code", change.code),
    ):
        if value:
            lines.append(f'{key} = "{value}"')
    return "[[event]]\n" + "\n".join(lines) + "\n"


def stamp(root: Path, version: str) -> list[Change]:
    """Append to the history what is unsealed, under `version`; return what was sealed."""
    _check_version(root, version)
    changes = unsealed(root)
    if not changes:
        return []
    _check_reuse(root, changes)
    path = root / HISTORY
    text = path.read_text(encoding="utf-8") if path.is_file() else ""
    blocks = [_block(version, change) for change in changes]
    separator = "" if not text or text.endswith("\n\n") else "\n"
    path.write_text(text + separator + "\n".join(blocks), encoding="utf-8")
    return changes


def describe(change: Change) -> str:
    """Say what the history lacks about a rule, in a few words."""
    parts = []
    if change.renamed_from:
        parts.append(f"renamed from {change.renamed_from}")
    if change.status:
        parts.append(f"is {change.status}")
    if change.code:
        parts.append(f"has the code {change.code}")
    return f"{change.rule} " + ", ".join(parts)


def check(root: Path) -> int:
    """Return 1 and name what no release has sealed, 0 when nothing is left."""
    changes = unsealed(root)
    for change in changes:
        print(f"{describe(change)}: not in {HISTORY}")
    if changes:
        print("Run `scripts/stamp_release.py stamp <version>` as part of the release.")
    return 1 if changes else 0


@app.command(name="stamp")
def stamp_command(version: str) -> None:
    """Seal what is unsealed under VERSION."""
    for change in stamp(Path.cwd(), version):
        print(f"{version}: {describe(change)}")


@app.command(name="check")
def check_command() -> None:
    """Fail while a status, a code or an id of the catalog is not sealed in the history."""
    raise SystemExit(check(Path.cwd()))


if __name__ == "__main__":
    app()

"""A release seals the status of every rule in `catalog/rules/history.toml`.

Nobody writes a version by hand: the author of a change sets the `status` of a rule, and the release
records, under its own version, every status that the history does not have yet. Asserted through
the public seam of `scripts/stamp_release.py`: the catalog and the history on disk go in, the
changes they need and the history that results come out (CES-65).
"""

from __future__ import annotations

import importlib.util
import sys
import tomllib
from pathlib import Path

import pytest

_SPEC = importlib.util.spec_from_file_location(
    "stamp_release", Path(__file__).resolve().parents[1] / "scripts" / "stamp_release.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
sr = importlib.util.module_from_spec(_SPEC)
sys.modules["stamp_release"] = sr
_SPEC.loader.exec_module(sr)

RULES = "catalog/rules"


def add_rule(root: Path, family: str, slug: str, status: str) -> None:
    folder = root / RULES / family / slug
    folder.mkdir(parents=True, exist_ok=True)
    (folder / "rule.toml").write_text(f'id = "{slug}"\nstatus = "{status}"\n', encoding="utf-8")


def write_history(root: Path, *events: tuple[str, str, str]) -> None:
    text = "# header\n\n" + "".join(
        f'[[event]]\nversion = "{version}"\nrule = "{rule}"\nstatus = "{status}"\n\n'
        for version, rule, status in events
    )
    (root / RULES).mkdir(parents=True, exist_ok=True)
    (root / RULES / "history.toml").write_text(text, encoding="utf-8")


def history(root: Path) -> list[tuple[str, str, str]]:
    data = tomllib.loads((root / RULES / "history.toml").read_text(encoding="utf-8"))
    return [(e["version"], e["rule"], e["status"]) for e in data.get("event", [])]


def changes(root: Path) -> list[tuple[str, str]]:
    return [(change.rule, change.status) for change in sr.unsealed(root)]


def test_a_rule_the_history_does_not_know_is_sealed_with_the_status_it_has(tmp_path: Path):
    add_rule(tmp_path, "logging", "fresh", "preview")
    write_history(tmp_path)

    assert changes(tmp_path) == [("fresh", "preview")]

    sr.stamp(tmp_path, "0.10.0")

    assert history(tmp_path) == [("0.10.0", "fresh", "preview")]


def test_a_rule_whose_status_changed_is_sealed_and_one_that_did_not_is_left_alone(tmp_path: Path):
    add_rule(tmp_path, "logging", "kept", "stable")
    add_rule(tmp_path, "logging", "old", "deprecated")
    write_history(tmp_path, ("0.8.0", "kept", "stable"), ("0.8.0", "old", "stable"))

    sr.stamp(tmp_path, "0.11.0")

    assert history(tmp_path) == [
        ("0.8.0", "kept", "stable"),
        ("0.8.0", "old", "stable"),
        ("0.11.0", "old", "deprecated"),
    ]


def test_the_last_event_of_a_rule_is_the_status_the_history_has(tmp_path: Path):
    add_rule(tmp_path, "logging", "grown", "stable")
    write_history(tmp_path, ("0.8.0", "grown", "preview"), ("0.9.0", "grown", "stable"))

    assert changes(tmp_path) == []

    add_rule(tmp_path, "logging", "grown", "preview")

    assert changes(tmp_path) == [("grown", "preview")]


def test_stamping_twice_changes_nothing_the_second_time(tmp_path: Path):
    add_rule(tmp_path, "logging", "fresh", "preview")
    write_history(tmp_path)
    sr.stamp(tmp_path, "0.10.0")
    sealed = (tmp_path / RULES / "history.toml").read_text(encoding="utf-8")

    assert sr.stamp(tmp_path, "0.10.0") == []
    assert (tmp_path / RULES / "history.toml").read_text(encoding="utf-8") == sealed


def test_the_events_of_a_release_come_after_the_old_ones_in_the_order_of_the_rules(tmp_path: Path):
    add_rule(tmp_path, "logging", "zeta", "preview")
    add_rule(tmp_path, "process", "alpha", "preview")
    write_history(tmp_path, ("0.8.0", "zeta", "stable"))
    before = (tmp_path / RULES / "history.toml").read_text(encoding="utf-8")

    sr.stamp(tmp_path, "0.10.0")

    text = (tmp_path / RULES / "history.toml").read_text(encoding="utf-8")
    assert text.startswith(before), "the old events are not touched"
    assert history(tmp_path) == [
        ("0.8.0", "zeta", "stable"),
        ("0.10.0", "alpha", "preview"),
        ("0.10.0", "zeta", "preview"),
    ]


def test_a_release_leaves_a_blank_line_before_its_events_even_after_a_trimmed_file(tmp_path: Path):
    add_rule(tmp_path, "logging", "fresh", "preview")
    write_history(tmp_path, ("0.8.0", "other", "stable"))
    path = tmp_path / RULES / "history.toml"
    path.write_text(path.read_text(encoding="utf-8").rstrip() + "\n", encoding="utf-8")

    sr.stamp(tmp_path, "0.10.0")

    assert 'status = "stable"\n\n[[event]]\nversion = "0.10.0"' in path.read_text(encoding="utf-8")


def test_a_version_older_than_the_last_event_or_not_a_release_is_refused(tmp_path: Path):
    add_rule(tmp_path, "logging", "fresh", "preview")
    write_history(tmp_path, ("0.9.0", "other", "stable"))

    for bad in ("0.8.0", "0.10", "v0.10.0", "latest", ""):
        with pytest.raises(ValueError, match="version"):
            sr.stamp(tmp_path, bad)

    assert history(tmp_path) == [("0.9.0", "other", "stable")]


def test_a_version_equal_to_the_last_event_is_a_release_that_stamps_again(tmp_path: Path):
    add_rule(tmp_path, "logging", "fresh", "preview")
    write_history(tmp_path, ("0.9.0", "other", "stable"))

    sr.stamp(tmp_path, "0.9.0")

    assert history(tmp_path)[-1] == ("0.9.0", "fresh", "preview")


def test_the_check_fails_while_a_status_is_not_sealed_and_names_it(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
):
    add_rule(tmp_path, "logging", "fresh", "preview")
    write_history(tmp_path)

    assert sr.check(tmp_path) == 1
    assert "fresh" in capsys.readouterr().out

    sr.stamp(tmp_path, "0.10.0")

    assert sr.check(tmp_path) == 0


def test_the_history_of_this_repository_knows_every_rule_it_records(tmp_path: Path):
    root = Path(__file__).resolve().parents[1]
    recorded = {rule for _, rule, _ in history(root)}
    slugs = {path.parent.name for path in (root / RULES).glob("*/*/rule.toml")}

    assert recorded <= slugs, f"events for rules that do not exist: {sorted(recorded - slugs)}"

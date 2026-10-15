"""Flag a change that adds a rule to a catalog and touches more than that rule needs.

A rule is its folder, `<catalog>/<family>/<slug>/`. Adding one should not require editing the
engine, so a change that adds a rule and touches anything outside its contract is a contract
change and has to be acknowledged.

A standard is a file, `standards/<set>/<number>-<slug>.md`. A change touches at most one, and no
label says otherwise. The number of a standard is the next free one of its set, so two standards of
a set never share it.
"""

from __future__ import annotations

import re
import subprocess
import tomllib
from collections import Counter
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import TYPE_CHECKING

import cyclopts

if TYPE_CHECKING:
    from collections.abc import Sequence

app = cyclopts.App(name="check-rule-change", help=__doc__)

BUILTIN_CATALOG = "lint/rules"
# The Rust a native rule lives in: its own code and the table that registers it.
NATIVE_FILES = (
    "lint/crates/botica-core/src/native.rs",
    "lint/crates/botica-core/src/layout.rs",
    "lint/crates/botica-core/src/registry.rs",
)
NATIVE_TESTS = "lint/crates/botica-core/tests/"
STANDARDS = "standards/"
STANDARD_FILE = re.compile(r"^standards/(?P<set>[^/]+)/(?P<number>\d+)-[^/]+\.md$")
# What lists the built-in catalog: pinned on purpose, so that a rule the runner stops running is
# noticed. A new rule is added to each.
INDEX_FILES = (
    "lint/crates/botica-cli/tests/conformance.rs",
    "lint/crates/botica-config/tests/resolve_shipped_catalog.rs",
    "lint/README.md",
)


@dataclass(frozen=True)
class Change:
    """One path of a branch and what happened to it (`A`dded, `M`odified, `D`eleted)."""

    status: str
    path: str


def _new_rule_folders(changes: Sequence[Change], catalogs: Sequence[str]) -> list[PurePosixPath]:
    """Return the folders of the rules a change adds; a rule that only changes family is a move."""
    rule_files = [PurePosixPath(change.path) for change in changes]
    removed = {
        path.parent.name
        for change, path in zip(changes, rule_files, strict=True)
        if change.status == "D" and path.name == "rule.toml"
    }
    folders = []
    for change, path in zip(changes, rule_files, strict=True):
        folder = path.parent
        if (
            change.status == "A"
            and path.name == "rule.toml"
            and str(folder.parents[1]) in catalogs
            and folder.name not in removed
        ):
            folders.append(folder)
    return folders


def _kinds(root: Path, folder: PurePosixPath) -> set[str]:
    data = tomllib.loads((root / folder / "rule.toml").read_text(encoding="utf-8"))
    return {implementation["kind"] for implementation in data.get("implementations", [])}


def _in_native_zone(path: str) -> bool:
    return path in NATIVE_FILES or path.startswith(NATIVE_TESTS)


def _open_to_any_rule(path: str, *, builtin: bool) -> bool:
    """Tell whether a rule may touch the path: its standard, or an index of the built-in catalog."""
    return path.startswith(STANDARDS) or (builtin and path in INDEX_FILES)


def duplicated_standards(root: Path) -> list[str]:
    """Return the `<set>/<number>` that more than one standard of the tree carries."""
    numbers = Counter(
        f"{match['set']}/{match['number']}"
        for path in sorted((root / "standards").glob("*/*.md"))
        if (match := STANDARD_FILE.match(path.relative_to(root).as_posix()))
    )
    return sorted(standard for standard, count in numbers.items() if count > 1)


def standards_touched(changes: Sequence[Change]) -> list[str]:
    """Return the standards a change touches as `<set>/<number>`, whatever their slug is."""
    touched = set()
    for change in changes:
        match = STANDARD_FILE.match(change.path)
        if match:
            touched.add(f"{match['set']}/{match['number']}")
    return sorted(touched)


def violations(
    changes: Sequence[Change], root: Path, catalogs: Sequence[str] = (BUILTIN_CATALOG,)
) -> list[str]:
    """Return the paths a change touches outside the contract of the rules it adds."""
    folders = _new_rule_folders(changes, catalogs)
    if not folders:
        return []
    builtin = [folder for folder in folders if str(folder.parents[1]) == BUILTIN_CATALOG]
    native = any("native" in _kinds(root, folder) for folder in builtin)
    own_files = {str(folder.parent / "family.toml") for folder in folders}
    return sorted(
        change.path
        for change in changes
        if not any(change.path.startswith(f"{folder}/") for folder in folders)
        and change.path not in own_files
        and not _open_to_any_rule(change.path, builtin=bool(builtin))
        and not (native and _in_native_zone(change.path))
    )


def parse_name_status(text: str) -> list[Change]:
    """Read `git diff --name-status --no-renames`: a rename arrives as a delete and an add."""
    changes = []
    for line in text.splitlines():
        status, _, path = line.partition("\t")
        if path:
            changes.append(Change(status[0], path))
    return changes


def check_branch(
    base: str,
    root: Path,
    *,
    catalogs: Sequence[str] = (BUILTIN_CATALOG,),
    acknowledged: bool = False,
) -> int:
    """Exit code for the changes of `HEAD` since it left `base`: 1 unless acknowledged."""
    diff = subprocess.run(
        ["git", "diff", "--name-status", "--no-renames", f"{base}...HEAD"],
        cwd=root,
        capture_output=True,
        text=True,
        check=True,
    )
    changes = parse_name_status(diff.stdout)
    duplicated = duplicated_standards(root)
    if duplicated:
        print("These standards share their number with another standard of the same set:")
        for standard in duplicated:
            print(f"  {standard}")
        return 1
    touched = standards_touched(changes)
    if len(touched) > 1:
        print("This change touches more than one standard; a change touches one:")
        for standard in touched:
            print(f"  {standard}")
        return 1
    found = violations(changes, root, catalogs)
    if not found:
        return 0
    print("This change adds a rule and also touches files outside the contract of that rule:")
    for path in found:
        print(f"  {path}")
    if acknowledged:
        print("Acknowledged as a contract change.")
        return 0
    print("Label the change `contract-change` to acknowledge it.")
    return 1


@app.default
def main(
    base: str,
    *,
    catalog: list[str] | None = None,
    acknowledged: bool = False,
) -> None:
    """Check the branch against BASE (a git ref); `--catalog` repeats for more catalogs."""
    raise SystemExit(
        check_branch(
            base,
            Path.cwd(),
            catalogs=tuple(catalog) if catalog else (BUILTIN_CATALOG,),
            acknowledged=acknowledged,
        )
    )


if __name__ == "__main__":
    app()

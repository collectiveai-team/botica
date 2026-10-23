"""Flag a change that adds a rule to a catalog and touches more than that rule needs.

A rule is its folder, `<catalog>/<family>/<slug>/`. Adding one should not require editing the
engine, so a change that adds a rule and touches anything outside its contract is a contract
change and has to be acknowledged.

The change may also adopt the rule in this repository (ADR 0013): a line that names it in the
`select` of `[tool.botica]` in `pyproject.toml`, the block of rules of `AGENTS.md` and the rubric
that botica exports, and comments `botica: ignore[<rule>]` or `botica: ignore-file[<rule>]` in any
file it modifies.

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
from typing import TYPE_CHECKING, Protocol

import cyclopts

if TYPE_CHECKING:
    from collections.abc import Collection, Sequence

app = cyclopts.App(name="check-rule-change", help=__doc__)

BUILTIN_CATALOG = "catalog/rules"
# The Rust a native rule lives in: its own code and the table that registers it.
NATIVE_FILES = (
    "crates/botica-core/src/native.rs",
    "crates/botica-core/src/layout.rs",
    "crates/botica-core/src/registry.rs",
)
NATIVE_TESTS = "crates/botica-core/tests/"
STANDARDS = "standards/"
STANDARD_FILE = re.compile(r"^standards/(?P<set>[^/]+)/(?P<number>\d+)-[^/]+\.md$")
# What lists the built-in catalog: pinned on purpose, so that a rule the runner stops running is
# noticed. A new rule is added to each.
INDEX_FILES = (
    "crates/botica-cli/tests/conformance.rs",
    "crates/botica-config/tests/resolve_shipped_catalog.rs",
)
# The manual names rules in its examples and lists (ADR 0012).
MANUAL = ("website/docs/guide/", "website/docs/extend/")
# Where this repository adopts a rule: its configuration and the two files botica exports from it.
CONFIG_FILE = "pyproject.toml"
AGENTS_FILE = "AGENTS.md"
RUBRIC_FILE = ".abide/rubric.json"
RULES_BLOCK = re.compile(r"<!-- botica:rules -->.*?<!-- /botica:rules -->", re.DOTALL)
SELECT_ENTRY = re.compile(r'^\s*"(?P<name>[^"]+)"\s*,?\s*(?:#.*)?$')
SUPPRESSION = re.compile(
    r"botica:\s*ignore(?P<file>-file)?\[(?P<names>[^\]]*)\](?P<reason>\s*--.*)?"
)
# What is left of a comment once the only directive in it is gone.
EMPTY_COMMENT = re.compile(r"\s*(?:#|//|--|;|/\*(?:\s*\*/)?|<!--(?:\s*-->)?)\s*$")


@dataclass(frozen=True)
class Change:
    """One path of a branch and what happened to it (`A`dded, `M`odified, `D`eleted)."""

    status: str
    path: str


class Texts(Protocol):
    """The text of a path where the branch left its base, and at the head of the branch."""

    def before(self, path: str) -> str: ...

    def after(self, path: str) -> str: ...


@dataclass(frozen=True)
class GitTexts:
    """Reads the texts from git: `base` is the commit where the branch left its base."""

    root: Path
    base: str

    def before(self, path: str) -> str:
        return self._show(f"{self.base}:{path}")

    def after(self, path: str) -> str:
        return self._show(f"HEAD:{path}")

    def _show(self, spec: str) -> str:
        shown = subprocess.run(
            ["git", "show", spec], cwd=self.root, capture_output=True, text=True, check=True
        )
        return shown.stdout


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


def _read_rule_toml(root: Path, folder: PurePosixPath) -> str:
    return (root / folder / "rule.toml").read_text(encoding="utf-8")


def _kinds(root: Path, folder: PurePosixPath) -> set[str]:
    data = tomllib.loads(_read_rule_toml(root, folder))
    return {str(implementation["kind"]) for implementation in data.get("implementations", [])}


def _rule_id(root: Path, folder: PurePosixPath) -> str:
    return str(tomllib.loads(_read_rule_toml(root, folder)).get("id", folder.name))


def _in_native_zone(path: str) -> bool:
    return path in NATIVE_FILES or path.startswith(NATIVE_TESTS)


def _open_to_any_rule(path: str, *, builtin: bool) -> bool:
    """Tell whether a rule may touch the path: its standard, or an index of the built-in catalog."""
    return path.startswith(STANDARDS) or (
        builtin and (path in INDEX_FILES or path.startswith(MANUAL))
    )


def _drop_suppressed(line: str, names: Collection[str]) -> str:
    """Return the line without the names of `names` in its suppressions of botica."""
    emptied = False

    def keep_others(match: re.Match[str]) -> str:
        nonlocal emptied
        kept = [name.strip() for name in match["names"].split(",")]
        kept = [name for name in kept if name and name not in names]
        if not kept:
            emptied = True
            return ""
        return f"botica: ignore{match['file'] or ''}[{', '.join(kept)}]{match['reason'] or ''}"

    dropped = SUPPRESSION.sub(keep_others, line)
    return EMPTY_COMMENT.sub("", dropped).rstrip() if emptied else dropped


def _without_suppressions(text: str, names: Collection[str]) -> list[str]:
    """Return the lines of the text as they were before it suppressed `names`."""
    lines = []
    for line in text.splitlines():
        dropped = _drop_suppressed(line, names)
        if dropped or not line.strip():
            lines.append(dropped)
    return lines


def only_suppresses(before: str, after: str, names: Collection[str]) -> bool:
    """Tell whether `after` is `before` with comments that silence `names` and nothing else."""
    return _without_suppressions(before, names) == _without_suppressions(after, names)


def _without_select_lines(text: str, names: Collection[str]) -> list[str]:
    """Return the lines of the text without those that list one of `names`, trailing commas off."""
    return [
        line.rstrip().rstrip(",")
        for line in text.splitlines()
        if not ((entry := SELECT_ENTRY.match(line)) and entry["name"] in names)
    ]


def only_selects(before: str, after: str, names: Collection[str]) -> bool:
    """Tell whether `after` is `before` with lines that add `names` to `[tool.botica].select`."""
    try:
        old = tomllib.loads(before)
        new = tomllib.loads(after)
    except tomllib.TOMLDecodeError:
        return False
    botica = new.get("tool", {}).get("botica", {})
    select = botica.get("select")
    if not isinstance(select, list):
        return False
    botica["select"] = [rule for rule in select if rule not in names]
    return old == new and _without_select_lines(before, names) == _without_select_lines(
        after, names
    )


def only_exports_rules(before: str, after: str) -> bool:
    """Tell whether `after` is `before` with another block of rules exported by botica."""
    return RULES_BLOCK.sub("", before) == RULES_BLOCK.sub("", after)


def adopts(change: Change, texts: Texts, names: Collection[str]) -> bool:
    """Tell whether a modified file only adopts the rules `names` in this repository."""
    if change.status != "M":
        return False
    if change.path == RUBRIC_FILE:
        return True
    before, after = texts.before(change.path), texts.after(change.path)
    if change.path == CONFIG_FILE:
        return only_selects(before, after, names)
    if change.path == AGENTS_FILE:
        return only_exports_rules(before, after)
    return only_suppresses(before, after, names)


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
    changes: Sequence[Change],
    root: Path,
    catalogs: Sequence[str] = (BUILTIN_CATALOG,),
    texts: Texts | None = None,
) -> list[str]:
    """Return the paths a change touches outside the contract of the rules it adds.

    With `texts`, a modified file that only adopts the new rules in this repository is inside the
    contract; without them, only the paths are judged.
    """
    folders = _new_rule_folders(changes, catalogs)
    if not folders:
        return []
    builtin = [folder for folder in folders if str(folder.parents[1]) == BUILTIN_CATALOG]
    native = any("native" in _kinds(root, folder) for folder in builtin)
    own_files = {str(folder.parent / "family.toml") for folder in folders}
    names = {_rule_id(root, folder) for folder in folders}
    return sorted(
        change.path
        for change in changes
        if _outside_contract(change.path, folders, own_files, builtin=bool(builtin), native=native)
        and not (texts is not None and adopts(change, texts, names))
    )


def _outside_contract(
    path: str,
    folders: Collection[PurePosixPath],
    own_files: Collection[str],
    *,
    builtin: bool,
    native: bool,
) -> bool:
    return (
        not any(path.startswith(f"{folder}/") for folder in folders)
        and path not in own_files
        and not _open_to_any_rule(path, builtin=builtin)
        and not (native and _in_native_zone(path))
    )


def parse_name_status(text: str) -> list[Change]:
    """Read `git diff --name-status --no-renames`: a rename arrives as a delete and an add."""
    changes = []
    for line in text.splitlines():
        status, _, path = line.partition("\t")
        if path:
            changes.append(Change(status[0], path))
    return changes


def _git(root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=root, capture_output=True, text=True, check=True
    ).stdout


def branch_changes(base: str, root: Path) -> list[Change]:
    """Return the files that `HEAD` changed since it left `base`."""
    return parse_name_status(_git(root, "diff", "--name-status", "--no-renames", f"{base}...HEAD"))


def branch_texts(base: str, root: Path) -> GitTexts:
    """Return the texts of the branch, from the commit where `HEAD` left `base`."""
    return GitTexts(root, _git(root, "merge-base", base, "HEAD").strip())


def verdict(
    duplicated: Sequence[str],
    touched: Sequence[str],
    found: Sequence[str],
    *,
    acknowledged: bool,
) -> tuple[int, list[str]]:
    """Return the exit code and the report for what the branch changed."""
    if duplicated:
        head = "These standards share their number with another standard of the same set:"
        return 1, [head, *(f"  {standard}" for standard in duplicated)]
    if len(touched) > 1:
        head = "This change touches more than one standard; a change touches one:"
        return 1, [head, *(f"  {standard}" for standard in touched)]
    if not found:
        return 0, []
    lines = [
        "This change adds a rule and also touches files outside the contract of that rule:",
        *(f"  {path}" for path in found),
    ]
    if acknowledged:
        return 0, [*lines, "Acknowledged as a contract change."]
    return 1, [*lines, "Label the change `contract-change` to acknowledge it."]


def check_branch(
    base: str,
    root: Path,
    *,
    catalogs: Sequence[str] = (BUILTIN_CATALOG,),
    acknowledged: bool = False,
) -> int:
    """Exit code for the changes of `HEAD` since it left `base`: 1 unless acknowledged."""
    changes = branch_changes(base, root)
    code, lines = verdict(
        duplicated_standards(root),
        standards_touched(changes),
        violations(changes, root, catalogs, branch_texts(base, root)),
        acknowledged=acknowledged,
    )
    for line in lines:
        print(line)
    return code


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

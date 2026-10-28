# A module of a script that a workflow runs as a file.
# ruff: noqa: INP001
"""The rules of `tests_fail_on_base.py`: which tests a change selects, and the verdict on them."""

from __future__ import annotations

import ast
import re
from dataclasses import dataclass, field
from pathlib import PurePosixPath
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable, Iterable, Mapping

TEST_FILE = re.compile(r"^(test_.*|.*_test)\.py$")
FIX_TITLE = re.compile(r"^fix(\([^)]*\))?!?:")
MISSING_MODULE = re.compile(r"No module named '(?P<module>[\w.]+)'")
MISSING_NAME = re.compile(
    r"cannot import name '\w+' from '(?P<module>[\w.]+)'"
    r"|module '(?P<owner>[\w.]+)' has no attribute"
)
PASSED = ("passed", "xpassed")
SKIPPED = ("skipped", "xfailed")
RED = ("red", "weak red")
NOTES = {
    "weak red": "fails on the base only because a name of the project is missing there: it "
    "checks that the name exists (a weak red).",
    "flaky on base": "failed on the base once and passed the second time, so it does not count.",
    "env missing": "cannot be imported on the base: a module outside the project is missing.",
}


@dataclass(frozen=True)
class FileChange:
    status: str  # A, M or D
    path: str


@dataclass(frozen=True)
class SelectedTest:
    path: str
    name: str  # `Class::function` or `function`
    kind: str  # added or changed
    line: int

    @property
    def nodeid(self) -> str:
        return f"{self.path}::{self.name}"


@dataclass(frozen=True)
class Outcome:
    """What pytest reports for a node (`collect-error` for a file), or the verdict on it."""

    outcome: str
    message: str = ""


# The outcome of each test node and each collection error, by node id.
OutcomeTable = dict[str, Outcome]
# The AST and the line of each test function of a file, by `Class::function`.
FunctionTable = dict[str, tuple[str, int]]


@dataclass(frozen=True)
class Row:
    test: SelectedTest
    nodeid: str
    head: str
    base: str
    verdict: str
    message: str = ""


@dataclass(frozen=True)
class Annotation:
    level: str  # error, warning or notice
    message: str
    file: str = ""
    line: int = 0


@dataclass
class Report:
    status: str  # pass, fail, error or not applicable
    summary: str
    rows: list[Row] = field(default_factory=list)
    annotations: list[Annotation] = field(default_factory=list)
    declared: bool = False

    @property
    def shown(self) -> str:
        """The status as the output says it: a failure that the label declares is a warning."""
        return "warning" if self.status == "fail" and self.declared else self.status

    @property
    def exit_code(self) -> int:
        if self.status == "error":
            return 2
        return 1 if self.status == "fail" and not self.declared else 0


def kind_of_path(path: str) -> str:
    """Return `test`, `docs` or `code` for a path of the diff."""
    p = PurePosixPath(path)
    if TEST_FILE.match(p.name) or p.name == "conftest.py" or "tests" in p.parts[:-1]:
        return "test"
    if "docs" in p.parts[:-1] or p.suffix in {".md", ".rst"}:
        return "docs"
    return "code"


def parse_name_status(text: str) -> list[FileChange]:
    """Read `git diff --name-status -M -z`; a rename is its old path deleted and its new added."""
    fields = text.split("\0")
    changes: list[FileChange] = []
    at = 0
    while at < len(fields) and fields[at]:
        status = fields[at][0]
        if status in "RC":
            if status == "R":
                changes.append(FileChange("D", fields[at + 1]))
            changes.append(FileChange("A", fields[at + 2]))
            at += 3
        else:
            changes.append(FileChange("M" if status == "T" else status, fields[at + 1]))
            at += 2
    return changes


def find_test_functions(source: bytes | None) -> FunctionTable:
    """Map each test function of a file to its AST, without positions, and to its line.

    The AST keeps the decorators: a new `parametrize` changes it, and a comment or a change of
    format does not.
    """
    found: FunctionTable = {}
    try:
        tree = ast.parse(source or b"")
    except SyntaxError:
        return found

    def visit(body: list[ast.stmt], prefix: str) -> None:
        for node in body:
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                if node.name.startswith("test"):
                    dump = ast.dump(node, include_attributes=False)
                    found[prefix + node.name] = (dump, node.lineno)
            elif isinstance(node, ast.ClassDef) and (
                node.name.startswith("Test")
                or any("TestCase" in ast.unparse(base) for base in node.bases)
            ):
                visit(node.body, f"{prefix}{node.name}::")

    visit(tree.body, "")
    return found


def select_tests(
    changes: Iterable[FileChange],
    before: Callable[[str], bytes | None],
    after: Callable[[str], bytes | None],
) -> list[SelectedTest]:
    """Select the test functions whose name is new in their file or whose AST changed."""
    selected: list[SelectedTest] = []
    for change in changes:
        if change.status == "D" or not TEST_FILE.match(PurePosixPath(change.path).name):
            continue
        old = find_test_functions(before(change.path))
        for name, (dump, line) in find_test_functions(after(change.path)).items():
            if name not in old:
                selected.append(SelectedTest(change.path, name, "added", line))
            elif old[name][0] != dump:
                selected.append(SelectedTest(change.path, name, "changed", line))
    return selected


def missing_name(message: str, project: set[str]) -> str | None:
    """Return `project` or `external` for a message that says that a module or a name is gone."""
    found = MISSING_MODULE.search(message) or MISSING_NAME.search(message)
    if found is None:
        return None
    module = found.group("module") or found.group("owner")
    return "project" if module.split(".")[0] in project else "external"


def function_of(nodeid: str) -> str:
    return re.sub(r"\[.*\]$", "", nodeid)


def collect_error(nodeid: str, outcomes: Mapping[str, Outcome]) -> Outcome | None:
    """Return the collection error of the file or of a folder of `nodeid`, if there is one."""
    for key, outcome in outcomes.items():
        inside = nodeid.startswith((f"{key}::", f"{key.rstrip('/')}/"))
        if outcome.outcome == "collect-error" and (not key or inside):
            return outcome
    return None


def verdict_on_base(nodeid: str, outcomes: Mapping[str, Outcome], project: set[str]) -> Outcome:
    result = outcomes.get(nodeid) or collect_error(nodeid, outcomes)
    if result is None:
        return Outcome("not run on base")
    if result.outcome in ("failed", "collect-error"):
        missing = missing_name(result.message, project)
        if missing == "external" and result.outcome == "collect-error":
            return Outcome("env missing", result.message)
        return Outcome("weak red" if missing == "project" else "red", result.message)
    if result.outcome == "error":
        return Outcome("red", result.message)
    return Outcome("green" if result.outcome in PASSED else "skipped on base")


def nodes_at_head(
    selected: list[SelectedTest], at_head: Mapping[str, Outcome]
) -> list[tuple[SelectedTest, str, Outcome]]:
    """List the nodes of each selected function; one with no node gets its collection error."""
    nodes = []
    for test in selected:
        found = [
            (test, nodeid, outcome)
            for nodeid, outcome in sorted(at_head.items())
            if function_of(nodeid) == test.nodeid and outcome.outcome != "collect-error"
        ]
        error = collect_error(test.nodeid, at_head) or Outcome("")
        nodes += found or [(test, test.nodeid, Outcome("not run", error.message))]
    return nodes


def red_nodes(
    nodes: list[tuple[SelectedTest, str, Outcome]],
    at_base: Mapping[str, Outcome],
    project: set[str],
) -> list[str]:
    """List the nodes that pass at the head and are red on the base, to run them again."""
    return [
        nodeid
        for _, nodeid, head in nodes
        if head.outcome in PASSED and verdict_on_base(nodeid, at_base, project).outcome in RED
    ]


def classify(
    nodes: list[tuple[SelectedTest, str, Outcome]],
    at_base: Mapping[str, Outcome],
    again: Mapping[str, Outcome],
    project: set[str],
) -> list[Row]:
    """Give each node its verdict; a node red on the base counts only if it is red `again`."""
    rows = []
    for test, nodeid, head in nodes:
        if head.outcome in SKIPPED:
            rows.append(Row(test, nodeid, head.outcome, "", "skipped at head"))
        elif head.outcome not in PASSED:
            rows.append(Row(test, nodeid, head.outcome, "", "broken at head", head.message))
        else:
            first = verdict_on_base(nodeid, at_base, project)
            base = at_base[nodeid].outcome if nodeid in at_base else "not collected"
            if first.outcome in RED and verdict_on_base(nodeid, again, project).outcome not in RED:
                first = Outcome("flaky on base", first.message)
            rows.append(Row(test, nodeid, head.outcome, base, first.outcome, first.message))
    return rows


def fix_without_tests(title: str, selected: list[SelectedTest]) -> list[Annotation]:
    if selected or not FIX_TITLE.match(title):
        return []
    message = (
        "The title starts with fix: and the pull request adds or changes no test. A test that "
        "fails before the fix and passes after it shows that the fix works."
    )
    return [Annotation("warning", message)]


def node_annotations(rows: list[Row]) -> list[Annotation]:
    """Return a notice for each node whose verdict needs one, and a warning for each function.

    A function gets the warning when none of its nodes is red on the base.
    """
    found = [
        Annotation(
            "notice",
            f"{row.nodeid} {NOTES[row.verdict]} {row.message}",
            row.test.path,
            row.test.line,
        )
        for row in rows
        if row.verdict in NOTES
    ]
    for test in dict.fromkeys(row.test for row in rows if row.verdict != "skipped at head"):
        if not any(row.verdict in RED for row in rows if row.test == test):
            message = f"No case of {test.nodeid} fails on the base: it passes before the change."
            found.append(Annotation("warning", message, test.path, test.line))
    return found


def decide(rows: list[Row], declared: bool, label: str, notes: list[Annotation]) -> Report:
    """Pass with a red node, fail without one, and stop on a node that fails at the head."""
    broken = [row for row in rows if row.verdict == "broken at head"]
    ran = [row for row in rows if row.verdict != "skipped at head"]
    red = [row for row in ran if row.verdict in RED]
    report = Report("pass", "", rows, notes, declared)
    if broken:
        report.status = "error"
        report.summary = f"{len(broken)} selected test nodes do not pass at the head."
        report.annotations += [
            Annotation(
                "error",
                f"{row.nodeid} does not pass at the head. {row.message}",
                row.test.path,
                row.test.line,
            )
            for row in broken
        ]
        return report
    if not ran:
        report.status, report.summary = "not applicable", "Every selected test skips at the head."
        return report
    report.annotations += node_annotations(rows)
    if red:
        report.summary = (
            f"{len(red)} of {len(ran)} selected test nodes fail on the base, in "
            f"{len({row.test for row in red})} of {len({row.test for row in ran})} test functions."
        )
        return report
    report.status = "fail"
    report.summary = "No test that the pull request adds or changes fails on its base."
    if declared:
        report.summary += f" The label {label} declares it."
    else:
        report.summary += (
            " If they pass on the base on purpose, as the characterisation tests of a refactor "
            f"do, add the label {label} to the pull request."
        )
    report.annotations.append(Annotation("warning" if declared else "error", report.summary))
    return report

"""The check of `ci-tests-fail-on-base-python`: the tests that a pull request adds fail on its base.

Asserted through the public seam of `.github/scripts/tests_fail_on_base.py`, `check`: an in-memory
repository and an in-memory pytest go in, the report comes out (CES-65). The adapters are tested in
`test_tests_fail_on_base_adapters.py`.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Protocol

import pytest
from tests_fail_on_base_support import core, tfb

if TYPE_CHECKING:
    from pathlib import Path

CODE = "src/calc/__init__.py"
TESTS = "tests/test_calc.py"
MEAN = "def mean(values):\n    return sum(values) / len(values)\n"
TEST_MEAN = "from calc import mean\n\n\ndef test_mean():\n    assert mean([1, 2, 3]) == 2\n"
BASE = {CODE: MEAN, TESTS: TEST_MEAN, "uv.lock": "lock of the base\n"}


class Outcome(Protocol):
    """What the in-memory pytest reads of an outcome of `tests_fail_on_base_core`."""

    outcome: str
    message: str


# The outcome of each test node, by node id.
OutcomeTable = dict[str, Outcome]


def passed() -> Outcome:
    return core.Outcome("passed")


def failed(message: str = "assert 2 == 3") -> Outcome:
    return core.Outcome("failed", message)


class MemoryRepository:
    """A repository whose base and head are dictionaries of files."""

    def __init__(self, root: Path, base: dict[str, str], head: dict[str, str]) -> None:
        self.root = root
        self.files = {
            "base": {path: text.encode() for path, text in base.items()},
            "HEAD": {path: text.encode() for path, text in head.items()},
        }
        self.overlay: dict[str, bytes | None] = {}
        self.removed: list[Path] = []

    def merge_base(self, base: str) -> str:
        assert base == "main"
        return "base"

    def changes(self, base: str) -> list[object]:
        before, after = self.files[base], self.files["HEAD"]
        return [
            core.FileChange("A" if path not in before else "D" if path not in after else "M", path)
            for path in sorted(before.keys() | after.keys())
            if before.get(path) != after.get(path)
        ]

    def read(self, rev: str, path: str) -> bytes | None:
        return self.files[rev].get(path)

    def base_tree(self, rev: str, overlay: dict[str, bytes | None]) -> Path:
        assert rev == "base"
        self.overlay = dict(overlay)
        return self.root / "base"

    def remove_tree(self, tree: Path) -> None:
        self.removed.append(tree)


class MemoryPytest:
    """pytest with the outcomes of the head and of each run on the base written down."""

    def __init__(self, head: OutcomeTable, *base: OutcomeTable) -> None:
        self.head = head
        self.base = list(base)
        self.runs: list[list[str]] = []
        self.broken: set[str] = set()

    def install(self, tree: Path) -> None:
        if tree.name in self.broken:
            message = f"`uv sync --frozen` failed in {tree}"
            raise tfb.SetupError(message)

    def run(self, tree: Path, nodeids: list[str]) -> OutcomeTable:
        self.runs.append(list(nodeids))
        table = self.base.pop(0) if tree.name == "base" else self.head
        return {
            nodeid: outcome
            for nodeid, outcome in table.items()
            if nodeid in nodeids
            or core.function_of(nodeid) in nodeids
            or outcome.outcome == "collect-error"
        }


@pytest.fixture
def project(tmp_path: Path) -> Path:
    (tmp_path / "src" / "calc").mkdir(parents=True)
    (tmp_path / "src" / "calc" / "__init__.py").write_text(MEAN, encoding="utf-8")
    return tmp_path


def run_check(root: Path, head: dict[str, str], runner: MemoryPytest, **inputs: object):
    repository = MemoryRepository(root, BASE, head)
    report = tfb.check(repository, runner, tfb.Inputs(base="main", **inputs))
    return report, repository


def test_a_fix_passes_when_one_of_its_new_cases_fails_on_the_base(project: Path):
    test = (
        TEST_MEAN + "\n\n@pytest.mark.parametrize('values', [[1], []])\n"
        "def test_mean_of_few(values):\n    assert mean(values) == (values or [0])[0]\n"
        "\n\ndef test_mean_of_two():\n    assert mean([1, 2]) == 1.5\n"
    )
    head = {**BASE, CODE: MEAN.replace("return", "return 0 if not values else"), TESTS: test}
    few = "tests/test_calc.py::test_mean_of_few"
    runner = MemoryPytest(
        {
            f"{few}[values0]": passed(),
            f"{few}[values1]": passed(),
            f"{TESTS}::test_mean_of_two": passed(),
        },
        {
            f"{few}[values0]": passed(),
            f"{few}[values1]": failed("ZeroDivisionError: division by zero"),
            f"{TESTS}::test_mean_of_two": passed(),
        },
        {f"{few}[values1]": failed("ZeroDivisionError: division by zero")},
    )

    report, _ = run_check(project, head, runner)

    assert (report.status, report.exit_code) == ("pass", 0)
    assert (
        report.summary == "1 of 3 selected test nodes fail on the base, in 1 of 2 test functions."
    )
    assert runner.runs[-1] == [f"{few}[values1]"]
    warnings = [note for note in report.annotations if note.level == "warning"]
    assert [(note.file, note.line) for note in warnings] == [(TESTS, 13)]
    assert "test_mean_of_two" in warnings[0].message


def test_new_tests_that_all_pass_on_the_base_fail_the_check_unless_the_label_declares_it(
    project: Path,
):
    head = {**BASE, CODE: MEAN + "# total\n", TESTS: TEST_MEAN + "\n\ndef test_one():\n    pass\n"}
    node = f"{TESTS}::test_one"

    report, _ = run_check(project, head, MemoryPytest({node: passed()}, {node: passed()}))
    declared, _ = run_check(
        project, head, MemoryPytest({node: passed()}, {node: passed()}), declared=True
    )

    assert (report.status, report.exit_code) == ("fail", 1)
    assert report.annotations[-1].level == "error"
    assert "add the label tests-pass-on-base" in report.annotations[-1].message
    assert (declared.status, declared.exit_code) == ("fail", 0)
    assert declared.annotations[-1].level == "warning"
    assert "The label tests-pass-on-base declares it." in declared.annotations[-1].message


def test_a_test_that_imports_a_new_name_of_the_project_is_a_weak_red(project: Path):
    test = "from calc import median\n\n\ndef test_median():\n    assert median([1, 3]) == 2\n"
    head = {**BASE, CODE: MEAN + "def median(v): ...\n", "tests/test_median.py": test}
    error = core.Outcome(
        "collect-error", "ImportError: cannot import name 'median' from 'calc' (/src/calc.py)"
    )
    runner = MemoryPytest(
        {"tests/test_median.py::test_median": passed()},
        {"tests/test_median.py": error},
        {"tests/test_median.py": error},
    )

    report, _ = run_check(project, head, runner)

    assert report.status == "pass"
    assert [row.verdict for row in report.rows] == ["weak red"]
    notices = [note.message for note in report.annotations if note.level == "notice"]
    assert len(notices) == 1
    assert "a name of the project is missing" in notices[0]


def test_a_module_outside_the_project_missing_on_the_base_is_not_red(project: Path):
    test = "import pytest_timeout\n\n\ndef test_many():\n    assert True\n"
    head = {**BASE, CODE: MEAN + "# many\n", "tests/test_many.py": test}
    error = core.Outcome("collect-error", "ModuleNotFoundError: No module named 'pytest_timeout'")
    runner = MemoryPytest(
        {"tests/test_many.py::test_many": passed()}, {"tests/test_many.py": error}
    )

    report, _ = run_check(project, head, runner)

    assert report.status == "fail"
    assert [row.verdict for row in report.rows] == ["env missing"]


def test_a_node_counts_as_red_only_when_it_fails_on_the_base_twice(project: Path):
    head = {**BASE, CODE: MEAN + "# race\n", TESTS: TEST_MEAN + "\n\ndef test_race():\n    pass\n"}
    node = f"{TESTS}::test_race"
    runner = MemoryPytest({node: passed()}, {node: failed()}, {node: passed()})

    report, _ = run_check(project, head, runner)

    assert report.status == "fail"
    assert [row.verdict for row in report.rows] == ["flaky on base"]


def test_a_selected_test_that_fails_at_the_head_stops_the_check(project: Path):
    head = {**BASE, CODE: MEAN + "# x\n", TESTS: TEST_MEAN + "\n\ndef test_x():\n    pass\n"}
    node = f"{TESTS}::test_x"

    report, _ = run_check(project, head, MemoryPytest({node: failed()}, {node: failed()}, {}))

    assert (report.status, report.exit_code) == ("error", 2)
    assert report.annotations[0].message == f"{node} does not pass at the head. assert 2 == 3"


def test_an_environment_that_does_not_install_stops_the_check_and_removes_the_base(project: Path):
    head = {**BASE, CODE: MEAN + "# x\n", TESTS: TEST_MEAN + "\n\ndef test_x():\n    pass\n"}
    runner = MemoryPytest({})
    runner.broken.add("base")

    report, repository = run_check(project, head, runner)

    assert (report.status, report.exit_code) == ("error", 2)
    assert repository.removed == [project / "base"]


def test_the_base_takes_the_test_files_and_the_lockfile_of_the_head(project: Path):
    head = {
        **{k: v for k, v in BASE.items() if k != TESTS},
        CODE: MEAN + "# x\n",
        "tests/test_new.py": "def test_new():\n    assert False\n",
        "tests/data/case.json": "{}\n",
        "uv.lock": "lock of the head\n",
    }
    node = "tests/test_new.py::test_new"

    report, repository = run_check(
        project, head, MemoryPytest({node: passed()}, {node: failed()}, {node: failed()})
    )

    assert report.status == "pass"
    assert repository.overlay == {
        TESTS: None,
        "tests/data/case.json": b"{}\n",
        "tests/test_new.py": b"def test_new():\n    assert False\n",
        "uv.lock": b"lock of the head\n",
    }
    assert repository.removed == [project / "base"]


@pytest.mark.parametrize(
    ("head", "summary"),
    [
        ({**BASE, TESTS: TEST_MEAN + "\n\ndef test_x():\n    pass\n"}, "changes no code"),
        ({**BASE, "docs/index.md": "# calc\n", TESTS: TEST_MEAN + "\n# a\n"}, "changes no code"),
        (
            {**BASE, CODE: MEAN + "# x\n", TESTS: TEST_MEAN.replace("\n\n\n", "\n\n\n# why\n")},
            "adds or changes no test function",
        ),
    ],
    ids=["tests only", "docs only", "a comment in a test"],
)
def test_the_check_does_not_apply_without_code_or_without_a_new_test(
    project: Path, head: dict[str, str], summary: str
):
    report, _ = run_check(project, head, MemoryPytest({}))

    assert (report.status, report.exit_code) == ("not applicable", 0)
    assert summary in report.summary


@pytest.mark.parametrize(
    ("title", "warned"),
    [("fix: mean of none", True), ("fix(calc)!: mean", True), ("feat: mean", False)],
)
def test_a_fix_that_adds_no_test_gets_a_warning(project: Path, title: str, *, warned: bool):
    head = {**BASE, CODE: MEAN.replace("len(values)", "max(len(values), 1)")}

    report, _ = run_check(project, head, MemoryPytest({}), title=title)

    assert report.status == "not applicable"
    assert bool(report.annotations) is warned
    if warned:
        assert report.annotations[0].message.startswith("The title starts with fix:")


def test_a_new_decorator_or_a_new_method_of_a_test_class_is_selected():
    before = b"class TestMean:\n    def test_a(self):\n        assert 1\n"
    after = (
        b"import pytest\n\n\nclass TestMean:\n    @pytest.mark.parametrize('x', [1])\n"
        b"    def test_a(self, x):\n        assert x\n\n    def test_b(self):\n        assert 2\n"
    )
    changes = [core.FileChange("M", "tests/test_mean.py"), core.FileChange("M", "tests/helpers.py")]

    def at_base(_path: str) -> bytes:
        return before

    def at_head(_path: str) -> bytes:
        return after

    selected = core.select_tests(changes, at_base, at_head)

    assert [(t.nodeid, t.kind, t.line) for t in selected] == [
        ("tests/test_mean.py::TestMean::test_a", "changed", 6),
        ("tests/test_mean.py::TestMean::test_b", "added", 9),
    ]

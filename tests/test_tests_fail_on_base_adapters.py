"""The edges of the check of `ci-tests-fail-on-base-python`.

The adapter of git runs on a repository in a temporary folder, the recorder of pytest runs
in-process, and the inputs and the output are the variables and the workflow commands of GitHub.
"""

from __future__ import annotations

import json
import subprocess
from typing import TYPE_CHECKING

from tests_fail_on_base_support import core, recorder, tfb

if TYPE_CHECKING:
    from pathlib import Path

    import pytest

pytest_plugins = ("pytester",)


def git(root: Path, *args: str) -> None:
    subprocess.run(  # botica: ignore[test-no-subprocess] -- the adapter under test drives git
        ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args],
        cwd=root,
        check=True,
        capture_output=True,
    )


def test_the_adapter_of_git_reads_a_rename_as_a_deletion_and_an_addition(tmp_path: Path):
    (tmp_path / "tests").mkdir()
    (tmp_path / "tests" / "test_old.py").write_text("def test_a():\n    pass\n", encoding="utf-8")
    git(tmp_path, "init", "-q", "-b", "main")
    git(tmp_path, "add", "-A")
    git(tmp_path, "commit", "-q", "-m", "base")
    git(tmp_path, "mv", "tests/test_old.py", "tests/test_new.py")
    (tmp_path / "app.py").write_text("x = 1\n", encoding="utf-8")
    git(tmp_path, "add", "-A")
    git(tmp_path, "commit", "-q", "-m", "head")
    repository = tfb.GitRepository(tmp_path)

    base = repository.merge_base("HEAD~1")
    tree = repository.base_tree(base, {"tests/test_new.py": b"new\n", "tests/test_old.py": None})

    assert repository.changes(base) == [
        core.FileChange("A", "app.py"),
        core.FileChange("D", "tests/test_old.py"),
        core.FileChange("A", "tests/test_new.py"),
    ]
    assert repository.read(base, "app.py") is None
    assert sorted(p.name for p in (tree / "tests").iterdir()) == ["test_new.py"]
    repository.remove_tree(tree)
    assert not tree.parent.exists()


def test_the_recorder_keeps_each_outcome_and_each_collection_error(pytester: pytest.Pytester):
    pytester.makepyfile(
        test_cases="""
        import pytest

        @pytest.fixture
        def broken():
            raise RuntimeError("no database")

        def test_fails():
            assert 1 == 2

        def test_setup(broken):
            pass

        @pytest.mark.parametrize("x", [1, 2])
        def test_case(x):
            assert x == 1

        @pytest.mark.skip
        def test_skipped():
            pass
        """,
        test_import="from calc import median\n\ndef test_median():\n    pass\n",
    )
    output = pytester.path / "outcomes.json"

    recorder.record([str(output), *tfb.PYTEST_ARGS, f"--rootdir={pytester.path}"])

    written = json.loads(output.read_text(encoding="utf-8"))
    data = {nodeid: (entry["outcome"], entry["message"]) for nodeid, entry in written.items()}
    assert data["test_cases.py::test_fails"] == ("failed", "assert 1 == 2")
    assert data["test_cases.py::test_setup"] == ("error", "setup: RuntimeError: no database")
    assert data["test_cases.py::test_case[1]"] == ("passed", "")
    assert data["test_cases.py::test_case[2]"][0] == "failed"
    assert data["test_cases.py::test_skipped"] == ("skipped", "")
    assert data["test_import.py"] == (
        "collect-error",
        "ModuleNotFoundError: No module named 'calc'",
    )


def test_the_log_carries_the_annotations_as_workflow_commands_of_github():
    test = core.SelectedTest("tests/a,b.py", "test_x", "added", 4)
    row = core.Row(test, test.nodeid, "passed", "passed", "green")
    report = core.decide([row], declared=False, label="tests-pass-on-base", notes=[])

    lines = tfb.workflow_commands(report)
    summary = tfb.summary_markdown(report)

    assert lines[0] == "green            added    tests/a,b.py::test_x"
    assert lines[1].startswith("::warning file=tests/a%2Cb.py,line=4::No case of")
    assert lines[-1].startswith("fail: No test that the pull request adds or changes fails")
    assert "| `tests/a,b.py::test_x` | added | passed | passed | green |" in summary


def test_a_failure_that_the_label_declares_is_shown_as_a_warning():
    test = core.SelectedTest("tests/test_a.py", "test_x", "added", 4)
    row = core.Row(test, test.nodeid, "passed", "passed", "green")
    report = core.decide([row], declared=True, label="tests-pass-on-base", notes=[])

    lines = tfb.workflow_commands(report)
    summary = tfb.summary_markdown(report)

    assert report.exit_code == 0
    assert lines[-1].startswith("warning: ")
    assert "\nwarning: " in summary


def test_the_inputs_come_from_the_variables_that_the_workflow_sets():
    environ = {"BASE_SHA": "abc", "PR_TITLE": "fix: x", "DECLARED": "true", "INSTALL": ""}

    inputs = tfb.Inputs.from_environment(environ)

    assert inputs == tfb.Inputs(base="abc", title="fix: x", declared=True)
    assert tfb.Inputs.from_environment({"LOCKFILES": "a.txt b.txt"}).lockfiles == ("a.txt", "b.txt")

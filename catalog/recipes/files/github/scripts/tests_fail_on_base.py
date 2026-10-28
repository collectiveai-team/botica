# A script that a workflow runs as a file, and that runs git, uv and pytest.
# ruff: noqa: INP001, S603, S607
"""Run the tests that a pull request adds or changes on its base (CES-124).

The recipe `ci-tests-fail-on-base-python` installs this script, and the workflow
`tests-fail-on-base-python.yml` runs it with `uv run` from the root of the checkout. It reads
`git diff <merge base>..HEAD` and selects the test functions that the change adds or changes. It
runs them with pytest at the head, and in a worktree of the merge base with the test files of the
head copied in. Both trees take the environment of the lockfile of the head. The check passes when
at least one selected test node fails on the base and every one passes at the head. The workflow
passes the inputs (`Inputs`) as variables of the environment.

Exit 0 when the check passes or does not apply, 1 when it fails, 2 when it cannot run.
"""

from __future__ import annotations

import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Protocol

from tests_fail_on_base_core import (
    Annotation,
    FileChange,
    Outcome,
    OutcomeTable,
    Report,
    Row,
    SelectedTest,
    classify,
    decide,
    fix_without_tests,
    kind_of_path,
    nodes_at_head,
    parse_name_status,
    red_nodes,
    select_tests,
)

if TYPE_CHECKING:
    from collections.abc import Callable, Mapping

# addopts may deselect by marker or add --doctest-modules, which change what runs. A file that the
# base cannot import stops the session unless --continue-on-collection-errors is given.
PYTEST_ARGS = [
    *("-p", "no:cacheprovider", "-p", "no:randomly", "-q", "--no-header", "-o", "addopts="),
    "--continue-on-collection-errors",
]


@dataclass(frozen=True)
class Inputs:
    """What the workflow passes, from the variables of the environment in parentheses."""

    base: str  # BASE_SHA: the commit of the base branch
    title: str = ""  # PR_TITLE: the title of the pull request
    declared: bool = False  # DECLARED: `true` when the pull request carries LABEL
    label: str = "tests-pass-on-base"  # LABEL
    install: str = "uv sync --frozen"  # INSTALL: installs the `.venv` of a tree
    lockfiles: tuple[str, ...] = ("uv.lock",)  # LOCKFILES: what the base takes from the head
    summary: str = ""  # GITHUB_STEP_SUMMARY

    @classmethod
    def from_environment(cls, environ: Mapping[str, str]) -> Inputs:
        """Read the variables that the workflow sets; an empty one takes the default."""
        default = cls(base="")
        return cls(
            base=environ.get("BASE_SHA", ""),
            title=environ.get("PR_TITLE", ""),
            declared=environ.get("DECLARED", "") == "true",
            label=environ.get("LABEL") or default.label,
            install=environ.get("INSTALL") or default.install,
            lockfiles=tuple(environ.get("LOCKFILES", "").split()) or default.lockfiles,
            summary=environ.get("GITHUB_STEP_SUMMARY", ""),
        )


class SetupError(Exception):
    """An environment cannot be installed."""


class Repository(Protocol):
    """The git repository of the pull request, checked out at its head in `root`."""

    root: Path

    def merge_base(self, base: str) -> str: ...
    def changes(self, base: str) -> list[FileChange]: ...
    def read(self, rev: str, path: str) -> bytes | None: ...
    def base_tree(self, rev: str, overlay: Mapping[str, bytes | None]) -> Path: ...
    def remove_tree(self, tree: Path) -> None: ...


class Runner(Protocol):
    """pytest in the environment of a tree."""

    def install(self, tree: Path) -> None: ...
    def run(self, tree: Path, nodeids: list[str]) -> OutcomeTable: ...


def project_names(tree: Path) -> set[str]:
    """Return the top-level import names of the project, at its root and in `src/`."""
    names = {path.parent.name for path in tree.glob("*/__init__.py")}
    if (tree / "src").is_dir():
        names |= {path.stem for path in (tree / "src").iterdir() if path.stem.isidentifier()}
    return names - {"tests", "test", "docs"}


def check(repository: Repository, runner: Runner, inputs: Inputs) -> Report:
    """Select the added and changed tests, run them at the head and on the base, and decide."""
    if not inputs.base:
        return Report(
            "error", "BASE_SHA is not set.", [], [Annotation("error", "BASE_SHA is not set.")]
        )
    base = repository.merge_base(inputs.base)
    changes = repository.changes(base)
    selected = select_tests(
        changes,
        lambda path: repository.read(base, path),
        lambda path: repository.read("HEAD", path),
    )
    notes = fix_without_tests(inputs.title, selected)
    if not any(kind_of_path(change.path) == "code" for change in changes):
        return Report("not applicable", "The pull request changes no code.", [], notes)
    if not selected:
        return Report(
            "not applicable", "The pull request adds or changes no test function.", [], notes
        )
    overlay = {
        change.path: None if change.status == "D" else repository.read("HEAD", change.path)
        for change in changes
        if kind_of_path(change.path) == "test"
    }
    overlay.update({path: repository.read("HEAD", path) for path in inputs.lockfiles})
    project = project_names(repository.root)
    base_tree = repository.base_tree(base, overlay)
    try:
        rows = run_trees(runner, (repository.root, base_tree), selected, project)
    except SetupError as error:
        return Report("error", str(error), [], [Annotation("error", str(error))])
    finally:
        repository.remove_tree(base_tree)
    return decide(rows, inputs.declared, inputs.label, notes)


def run_trees(
    runner: Runner, trees: tuple[Path, Path], selected: list[SelectedTest], project: set[str]
) -> list[Row]:
    """Run the selected tests at the head and on the base, and the red nodes on the base again."""
    head_tree, base_tree = trees
    runner.install(head_tree)
    runner.install(base_tree)
    ids = sorted({test.nodeid for test in selected})
    nodes = nodes_at_head(selected, runner.run(head_tree, ids))
    at_base = runner.run(base_tree, ids)
    red = red_nodes(nodes, at_base, project)
    again = runner.run(base_tree, red) if red else {}
    return classify(nodes, at_base, again, project)


def escape(text: str, *, prop: bool = False) -> str:
    """Escape a value of a workflow command of GitHub."""
    text = text.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
    return text.replace(":", "%3A").replace(",", "%2C") if prop else text


def workflow_commands(report: Report) -> list[str]:
    """Return the lines of the log: a line per node, the annotations and the result."""
    lines = [
        f"{row.verdict:<16} {row.test.kind:<8} {row.nodeid}  {row.message[:200]}".rstrip()
        for row in report.rows
    ]
    for note in report.annotations:
        where = [f"file={escape(note.file, prop=True)}"] if note.file else []
        where += [f"line={note.line}"] if note.line else []
        props = f" {','.join(where)}" if where else ""
        lines.append(f"::{note.level}{props}::{escape(note.message.strip())}")
    return [*lines, f"{report.shown}: {report.summary}"]


def summary_markdown(report: Report) -> str:
    lines = ["## Tests that fail on the base", "", f"{report.shown}: {report.summary}", ""]
    if report.rows:
        lines += ["| Test | Change | Head | Base | Verdict |", "|---|---|---|---|---|"]
        for row in report.rows:
            test = row.nodeid.replace("|", "\\|").replace("`", "'")
            base = row.base or "-"
            lines.append(f"| `{test}` | {row.test.kind} | {row.head} | {base} | {row.verdict} |")
    return "\n".join(lines) + "\n"


class GitRepository:
    """The repository of a folder, through the command git."""

    def __init__(self, root: Path) -> None:
        self.root = root

    def _git(self, *args: str) -> subprocess.CompletedProcess[bytes]:
        return subprocess.run(
            ["git", "-C", str(self.root), *args], capture_output=True, check=False
        )

    def merge_base(self, base: str) -> str:
        result = self._git("merge-base", base, "HEAD")
        result.check_returncode()
        return result.stdout.decode().strip()

    def changes(self, base: str) -> list[FileChange]:
        diff = self._git("diff", "--name-status", "-M", "-z", base, "HEAD")
        return parse_name_status(diff.stdout.decode())

    def read(self, rev: str, path: str) -> bytes | None:
        result = self._git("show", f"{rev}:{path}")
        return result.stdout if result.returncode == 0 else None

    def base_tree(self, rev: str, overlay: Mapping[str, bytes | None]) -> Path:
        """Add a worktree at `rev` in a temporary folder and write `overlay` into it."""
        tree = Path(tempfile.mkdtemp(prefix="tests-fail-on-base-")) / "base"
        self._git("worktree", "add", "--detach", "--quiet", str(tree), rev).check_returncode()
        for path, content in overlay.items():
            (tree / path).unlink(missing_ok=True)
            if content is not None:
                (tree / path).parent.mkdir(parents=True, exist_ok=True)
                (tree / path).write_bytes(content)
        return tree

    def remove_tree(self, tree: Path) -> None:
        self._git("worktree", "remove", "--force", str(tree))
        shutil.rmtree(tree.parent, ignore_errors=True)


class UvPytest:
    """pytest in the `.venv` of a tree, which the command `install` installs."""

    def __init__(self, install: str, recorder: Path, environ: Mapping[str, str]) -> None:
        self.install_command = shlex.split(install)
        self.recorder = recorder
        self.environ = environ

    def install(self, tree: Path) -> None:
        # `uv run` points VIRTUAL_ENV at the environment of the check; `uv pip` would install there.
        env = {**self.environ, "VIRTUAL_ENV": str(tree / ".venv")}
        result = subprocess.run(
            self.install_command, cwd=tree, env=env, capture_output=True, text=True, check=False
        )
        if result.returncode:
            command = shlex.join(self.install_command)
            raise SetupError(f"`{command}` failed in {tree}: {result.stderr.strip()[-600:]}")

    def run(self, tree: Path, nodeids: list[str]) -> OutcomeTable:
        python = tree / ".venv" / "bin" / "python"
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / "outcomes.json"
            command = [str(python), str(self.recorder), str(output), *PYTEST_ARGS]
            result = subprocess.run(
                [*command, f"--rootdir={tree}", *nodeids],
                cwd=tree,
                capture_output=True,
                text=True,
                check=False,
            )
            data = json.loads(output.read_text(encoding="utf-8")) if output.exists() else {}
        outcomes = {nodeid: Outcome(**entry) for nodeid, entry in data.items()}
        if not outcomes:
            tail = (result.stdout + result.stderr).strip().splitlines()[-5:]
            outcomes[""] = Outcome("collect-error", " ".join(tail))
        return outcomes


def main(environ: Mapping[str, str], write: Callable[[str], object]) -> int:
    """Run the check on the repository of the current folder, and report it."""
    inputs = Inputs.from_environment(environ)
    recorder = Path(__file__).with_name("tests_fail_on_base_pytest.py")
    runner = UvPytest(inputs.install, recorder, environ)
    report = check(GitRepository(Path.cwd()), runner, inputs)
    for line in workflow_commands(report):
        write(line)
    if inputs.summary:
        with Path(inputs.summary).open("a", encoding="utf-8") as summary:
            summary.write(summary_markdown(report))
    return report.exit_code


if __name__ == "__main__":
    sys.exit(main(os.environ, print))  # noqa: BTCONF001 -- the inputs of a step of a workflow

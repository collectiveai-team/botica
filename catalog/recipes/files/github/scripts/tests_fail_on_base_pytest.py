# A script that a workflow runs as a file.
# ruff: noqa: INP001
"""Run pytest and write the outcome of each test node and each collection error to a file.

`tests_fail_on_base.py` runs it in the Python of a tree, whose environment has pytest:
`python tests_fail_on_base_pytest.py <output> <arguments of pytest>`.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

import pytest


class Recorder:
    """A plugin of pytest that keeps the outcome of each test node and each collection error."""

    def __init__(self) -> None:
        self.outcomes: dict[str, dict[str, str]] = {}

    @staticmethod
    def _message(report: pytest.CollectReport | pytest.TestReport) -> str:
        crash = getattr(report.longrepr, "reprcrash", None)
        lines = str(crash.message if crash else report.longrepr or "").strip().splitlines()
        line = lines[0] if crash and lines else (lines[-1] if lines else "")
        return re.sub(r"^E\s+", "", line)[:300]

    def pytest_collectreport(self, report: pytest.CollectReport) -> None:
        if report.failed:
            entry = {"outcome": "collect-error", "message": self._message(report)}
            self.outcomes[report.nodeid] = entry

    def pytest_runtest_logreport(self, report: pytest.TestReport) -> None:
        entry = self.outcomes.setdefault(report.nodeid, {"outcome": "passed", "message": ""})
        xfail = hasattr(report, "wasxfail")
        if report.when == "call" and report.failed:
            entry.update(outcome="failed", message=self._message(report))
        elif report.when == "call" and report.skipped:
            entry["outcome"] = "xfailed" if xfail else "skipped"
        elif report.when == "call" and xfail:
            entry["outcome"] = "xpassed"
        elif report.failed:
            entry.update(outcome="error", message=f"{report.when}: {self._message(report)}")
        elif report.when == "setup" and report.skipped:
            entry["outcome"] = "skipped"


def record(argv: list[str]) -> int:
    """Run pytest with `argv[1:]` and write the outcome of each node to the file `argv[0]`."""
    output, *args = argv
    # `python -m pytest` puts the current folder first on the path, and a script its own folder.
    sys.path.insert(0, str(Path.cwd()))
    recorder = Recorder()
    pytest.main(args, plugins=[recorder])
    Path(output).write_text(json.dumps(recorder.outcomes, indent=1), encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(record(sys.argv[1:]))

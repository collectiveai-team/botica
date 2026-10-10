"""Parity of botica's `file-size-guard` and `no-utils` with the `python -c` hooks they replace.

The hooks are the ones in scaffolding/templates/prek-python.toml, run by prek itself (so the
file selection is prek's: tracked files, `types`, `files`, `exclude`). Two checks:

1. the real repositories of real-repos.txt: both tools run on the same checkout and what they
   report is compared file by file. A difference is expected only if it is one of two decided ones
   (see DECIDED below); anything else is a failure.
2. edge cases (edge_cases()): a small git repository with one file per case, where the table says
   what the hook reports and what botica reports. A case where the two differ is a decided
   difference, with its reason: this table is the one in the spec, so it cannot go stale.

Usage: hook_parity.py <botica-binary> [workdir]
Needs prek on PATH. Clones go to <workdir>/real/<name> (shared with parity_real.sh); amarc is
private, so it is skipped if it cannot be cloned. Exit status 1 if anything is unexplained.
"""

import json
import re
import subprocess
import sys
import tempfile
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip())
BENCH = ROOT / "lint" / "bench"
TEMPLATE = ROOT / "scaffolding" / "templates" / "prek-python.toml"
RULES = ("file-size-guard", "no-utils")
HOOK_LINE = re.compile(r"^\s+(Error|Warning): (CES-\d+ \((?:file-size-guard|no-utils)\): .*)$")
PATH_IN_MESSAGE = re.compile(r"\): (.+?) (?:is too large|is large|is a catch-all)")
WALK = ROOT / "lint" / "crates" / "botica-core" / "src" / "walk" / "mod.rs"
# The directories of tools the walk does not enter, read from the walk itself.
TOOL_DIRS = frozenset(
    re.findall(
        r'"(\.[^"]+)"', re.search(r"const TOOL_DIRS[^=]*= \[(.*?)\];", WALK.read_text(), re.S)[1]
    )
)
DECIDED = {
    "scope": "botica only, not Python: file-size-guard covers every language (spec 4.2)",
    "tool_dir": "hook only, in the directory of a tool (TOOL_DIRS): botica's walk skips it",
}


@dataclass(frozen=True)
class Finding:
    severity: str
    path: str
    message: str


@dataclass(frozen=True)
class Case:
    """One file; what each tool reports on it (None = nothing), and why they differ if they do."""

    path: str
    content: bytes
    hook: str | None
    botica: str | None
    rule: str = "file-size-guard"
    note: str = ""


@dataclass
class Comparison:
    same: int = 0
    scope: list[str] = field(default_factory=list)
    tool_dir: list[str] = field(default_factory=list)
    unexplained: list[str] = field(default_factory=list)


def lines(count: int, eol: bytes = b"\n", final: bool = True) -> bytes:
    body = eol.join(b"x = 1" for _ in range(count))
    return body + eol if final and count else body


def edge_cases() -> list[Case]:
    cr = "A lone \\r: Python's universal newlines count it, botica counts \\n only"
    nul = "A binary file (NUL byte) is skipped by botica; the hook counts it as text"
    names = "utils.py", "helpers.py", "aux.py", "misc.py", "common.py", "pkg/utils.py"
    quiet = "tests/utils.py", "pkg/tests/helpers.py", "my_utils.py", "Utils.py", "utils.pyi"
    cases = [
        Case("lines_399.py", lines(399), None, None),
        Case("lines_400.py", lines(400), "warning", "warning"),
        Case("lines_699.py", lines(699), "warning", "warning"),
        Case("lines_700.py", lines(700), "error", "error"),
        Case("empty.py", b"", None, None),
        Case("no_final_newline.py", lines(700, final=False), "error", "error"),
        Case("crlf.py", lines(700, b"\r\n"), "error", "error"),
        Case("lone_cr.py", lines(700, b"\r"), "error", None, note=cr),
        Case("latin1.py", lines(700) + b"# caf\xe9\n", "error", "error"),
        Case("binary.py", b"\x00" + lines(700), "error", None, note=nul),
        Case("big.ts", lines(700), None, "error", note=DECIDED["scope"]),
        Case("big.sh", lines(450), None, "warning", note=DECIDED["scope"]),
        Case("big.md", lines(700), None, None),
        Case("big.rst", lines(700), None, None),
        Case("poetry.lock", lines(700), None, None),
        Case(".hidden/big.py", lines(700), "error", "error"),
        Case(".venv/big.py", lines(700), "error", None, note=DECIDED["tool_dir"]),
        Case("tool", b"#!/usr/bin/env python3\n" + lines(700), "error", "error"),
        Case("stub.pyi", lines(700), None, "error", note="prek's `python` type leaves out .pyi"),
        Case("ignored.py", lines(700), "error", None, note="Tracked but in .gitignore"),
    ]
    cases += [Case(n, b"x = 1\n", "error", "error", rule="no-utils") for n in names]
    cases += [Case(n, b"x = 1\n", None, None, rule="no-utils") for n in quiet]
    cases += [
        Case("latin1/utils.py", b"# caf\xe9\n", "error", "error", rule="no-utils"),
        Case("pkg/utils.js", b"", None, None, rule="no-utils"),
        Case(".github/utils.py", b"", "error", "error", rule="no-utils"),
    ]
    return cases


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command, cwd=cwd, capture_output=True, text=True, errors="replace", check=False
    )


def severity(prefix: str) -> str:
    return prefix.lower()


def hook_findings(repo: Path, rule: str) -> list[Finding]:
    done = run(["prek", "run", rule, "--all-files", "--verbose", "-c", str(TEMPLATE)], repo)
    if done.returncode not in (0, 1):
        sys.exit(f"prek failed on {repo} ({rule}): {done.stderr or done.stdout}")
    found = []
    for line in done.stdout.splitlines():
        if match := HOOK_LINE.match(line):
            path = PATH_IN_MESSAGE.search(match.group(2))
            found.append(Finding(severity(match.group(1)), path.group(1), match.group(2)))
    return found


def botica_findings(botica: Path, repo: Path, rule: str) -> list[Finding]:
    command = [str(botica), "lint", ".", "--no-astgrep", "--select", rule]
    done = run([*command, "--output-format", "json"], repo)
    report = json.loads(done.stdout)
    if report["errors"]:
        sys.exit(f"botica reported errors on {repo}: {report['errors']}")
    return [Finding(d["severity"], d["path"], d["message"]) for d in report["diagnostics"]]


def in_tool_dir(path: str) -> bool:
    return any(part in TOOL_DIRS for part in path.split("/"))


def compare(hook: list[Finding], botica: list[Finding]) -> Comparison:
    by_hook = {f.path: f for f in hook}
    by_botica = {f.path: f for f in botica}
    result = Comparison()
    for path in sorted(by_hook.keys() | by_botica.keys()):
        theirs, ours = by_hook.get(path), by_botica.get(path)
        if theirs == ours:
            result.same += 1
        elif theirs is None and not path.endswith(".py"):
            result.scope.append(path)
        elif ours is None and in_tool_dir(path):
            result.tool_dir.append(path)
        else:
            result.unexplained.append(f"{path}: hook {theirs and theirs.message} | botica {ours}")
    return result


def pinned_checkouts(workdir: Path) -> list[tuple[str, Path]]:
    checkouts = []
    for line in (BENCH / "real-repos.txt").read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        name, url, commit = line.split()
        repo = workdir / "real" / name
        clone = ["git", "clone", "--quiet", url, str(repo)]
        if not (repo / ".git").is_dir() and run(clone, ROOT).returncode:
            print(f"== {name}: cannot clone, skipped")
            continue
        run(["git", "checkout", "--quiet", commit], repo)
        checkouts.append((name, repo))
    return checkouts


def print_comparison(name: str, rule: str, result: Comparison) -> None:
    extensions = Counter(Path(p).suffix or p for p in result.scope).most_common(4)
    print(
        f"== {name} {rule}: {result.same} identical, {len(result.scope)} scope"
        f" {extensions}, {len(result.tool_dir)} in tool directories,"
        f" {len(result.unexplained)} unexplained"
    )
    for item in result.tool_dir[:5]:
        print(f"   tool directory {item}")
    for item in result.unexplained[:10]:
        print(f"   UNEXPLAINED {item}")


def real_repo_report(botica: Path, workdir: Path) -> int:
    failures = 0
    for name, repo in pinned_checkouts(workdir):
        for rule in RULES:
            result = compare(hook_findings(repo, rule), botica_findings(botica, repo, rule))
            print_comparison(name, rule, result)
            failures += len(result.unexplained)
        shape = run(["prek", "run", "repo-shape", "--all-files", "-c", str(TEMPLATE)], repo)
        skipped = "no files to check" in shape.stdout
        print(f"== {name} repo-shape hook: {'no files to check' if skipped else 'RAN'}")
        failures += not skipped
    return failures


def write_edge_repo(repo: Path, cases: list[Case]) -> None:
    run(["git", "init", "--quiet"], repo)
    (repo / ".gitignore").write_text("ignored.py\n")
    for case in cases:
        target = repo / case.path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(case.content)
        if case.path == "tool":
            target.chmod(0o755)
    run(["git", "add", "--force", "--all"], repo)


def check_cases(botica: Path, repo: Path, rule: str, cases: list[Case]) -> int:
    by_hook = {f.path: f.severity for f in hook_findings(repo, rule)}
    by_botica = {f.path: f.severity for f in botica_findings(botica, repo, rule)}
    failures = 0
    for case in (c for c in cases if c.rule == rule):
        actual = (by_hook.get(case.path), by_botica.get(case.path))
        status = "ok  " if actual == (case.hook, case.botica) else "FAIL"
        failures += status == "FAIL"
        print(
            f"{status} {rule:15} {case.path:22} hook={actual[0]!s:8} botica={actual[1]!s:8}"
            f" {case.note}"
        )
    return failures


def edge_case_report(botica: Path, workdir: Path) -> int:
    cases = edge_cases()
    with tempfile.TemporaryDirectory(prefix="edge-", dir=workdir) as scratch:
        repo = Path(scratch)
        write_edge_repo(repo, cases)
        return sum(check_cases(botica, repo, rule, cases) for rule in RULES)


def main() -> int:
    botica = Path(sys.argv[1]).resolve()
    workdir = Path(sys.argv[2] if len(sys.argv) > 2 else ROOT / ".tmp" / "bench").resolve()
    (workdir / "real").mkdir(parents=True, exist_ok=True)
    print("-- edge cases")
    failures = edge_case_report(botica, workdir)
    print("-- real repositories")
    failures += real_repo_report(botica, workdir)
    print("FAILURES:", failures)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())

"""Selection + plan-building + apply.

Clean-adds only; merges are deferred. The single bounded exception is the
``unignore`` op — see ``docs/adr/0001-scoped-unignore-op.md``.
"""

from __future__ import annotations

import subprocess
from pathlib import Path
from typing import TYPE_CHECKING

from scaffolding import console
from scaffolding.components import REGISTRY, Component, Context, lookup
from scaffolding.plan import Decisions, Disposition, Op, Plan
from scaffolding.skills import UNIGNORE_WHITELIST, installed_names

if TYPE_CHECKING:
    from scaffolding.facts import Facts
    from scaffolding.settings import Settings


class NotAGitRepo(Exception):
    pass


class UnknownComponent(Exception):
    pass


def _initial_selection(
    requested: list[str], facts: Facts, settings: Settings
) -> tuple[list[str], bool]:
    """Return (component names, explicit) before skip/dep resolution."""
    if requested:
        for name in requested:
            if lookup(name) is None:
                raise UnknownComponent(name)
        return list(dict.fromkeys(requested)), True
    names = [c.name for c in REGISTRY if c.default_on and c.gate(facts)]
    if settings.with_ci and "ci" not in names:
        names.append("ci")
    return names, False


def _gate_notices(names: list[str], facts: Facts) -> list[str]:
    """Warn when an explicitly-selected component's gate is not satisfied."""
    out = []
    for n in names:
        c = lookup(n)
        if c and not c.gate(facts):
            out.append(f"{n}: gate not satisfied for this repo — including anyway (explicit)")
    return out


def _resolve_deps(names: list[str], skip: list[str], facts: Facts) -> list[str]:
    """Append missing dependencies in place; return the notices for them."""
    notices = []
    for n in list(names):
        c = lookup(n)
        if not c:
            continue
        for dep in c.deps(facts):
            if dep not in names and dep not in skip:
                names.append(dep)
                notices.append(f"{dep} (required by {n})")
    return notices


def select_components(
    requested: list[str],
    skip: list[str],
    facts: Facts,
    settings: Settings,
) -> tuple[list[Component], list[str]]:
    """Resolve the component set. Returns (ordered components, notices)."""
    skip = skip or []
    names, explicit = _initial_selection(requested, facts, settings)
    if settings.skip_ci and "ci" in names:
        names.remove("ci")
    names = [n for n in names if n not in skip]

    notices: list[str] = []
    if explicit:
        notices += _gate_notices(names, facts)
    if not settings.no_deps:
        notices += _resolve_deps(names, skip, facts)

    ordered = [c for c in REGISTRY if c.name in names]
    return ordered, notices


def build_plan(
    root: Path,
    facts: Facts,
    settings: Settings,
    *,
    requested: list[str] | None = None,
    skip: list[str] | None = None,
    decisions: Decisions | None = None,
    interactive: bool = False,
) -> Plan:
    components, notices = select_components(requested or [], skip or [], facts, settings)
    ctx = Context(
        root=root,
        facts=facts,
        settings=settings,
        decisions=decisions if decisions is not None else Decisions(),
        interactive=interactive,
    )
    plan = Plan(facts=facts, notices=notices)
    for comp in components:
        plan.ops.extend(comp.plan(ctx))
    plan.decisions = list(ctx._decisions)
    return plan


def _apply_write(op: Op) -> None:
    if op.path is None:
        raise ValueError(f"write op for {op.target} has no path")
    p = Path(op.path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(op.content or "", encoding="utf-8")


def _apply_append(op: Op) -> None:
    if op.path is None:
        raise ValueError(f"append op for {op.target} has no path")
    p = Path(op.path)
    p.parent.mkdir(parents=True, exist_ok=True)
    cur = p.read_text(encoding="utf-8") if p.exists() else ""
    if cur and not cur.endswith("\n"):
        cur += "\n"
    p.write_text(cur + (op.content or ""), encoding="utf-8")


def _apply_symlink(op: Op) -> None:
    if op.path is None or op.content is None:
        raise ValueError(f"symlink op for {op.target} needs path and content")
    link = Path(op.path)
    link.parent.mkdir(parents=True, exist_ok=True)
    if link.exists() or link.is_symlink():
        return  # clean-adds-only: never replace an existing path
    link.symlink_to(op.content)


def _apply_unignore(op: Op) -> None:
    """Remove one literal, whitelisted line from .gitignore.

    The single exception to clean-adds-only (CES-107). It is bounded by an explicit
    literal whitelist rather than a pattern language, so "clean-adds plus a known
    set of line removals" remains a checkable invariant.
    """
    if op.path is None or op.content is None:
        raise ValueError(f"unignore op for {op.target} needs path and content")
    line = op.content.strip()
    if line not in UNIGNORE_WHITELIST:
        raise ValueError(f"unignore refuses non-whitelisted line: {line!r}")
    path = Path(op.path)
    if not path.exists():
        return
    # newline="" disables newline translation on both ends: a CRLF or mixed-ending
    # .gitignore keeps its endings, and a file with no trailing newline does not
    # grow one. Removing one line must not produce an all-lines diff.
    raw = path.read_text(encoding="utf-8", newline="")
    kept = [ln for ln in raw.splitlines(keepends=True) if ln.strip() != line]
    if len(kept) == len(raw.splitlines(keepends=True)):
        console.warn(f"{op.target}: no literal `{line}` line found — nothing removed")
        return
    path.write_text("".join(kept), encoding="utf-8", newline="")


def _apply_add(op: Op) -> None:
    if op.kind == "write":
        _apply_write(op)
    elif op.kind == "append":
        _apply_append(op)
    elif op.kind == "symlink":
        _apply_symlink(op)
    elif op.kind == "unignore":
        _apply_unignore(op)


def _delivered(op: Op, root: Path) -> bool:
    """Assert a run op's post-condition: the skills it promised are now on disk.

    Checked against the derived tree, not against ``skills-lock.json``. The lock is
    written by the same tool whose work we are verifying, so a manifest-only check
    passes whenever the tool records an install it did not perform.
    """
    if not op.expect_skills:
        return True
    if op.expect_dir is None:
        raise ValueError(f"{op.target}: expect_skills set without expect_dir")
    present = set(installed_names(root, op.expect_dir))
    missing = [n for n in op.expect_skills if n not in present]
    if not missing:
        return True
    console.warn(
        f"{op.target}: ran but {op.expect_dir} is missing {', '.join(missing)} — "
        "the skill names are wrong or upstream renamed them, and this repo got nothing"
    )
    return False


def _apply_run(op: Op, root: Path) -> bool:
    """Run op.cmd. Return True on success or non-fatal failure, False otherwise.

    Two failure classes are deliberately distinguished (CES-107): a tool that cannot
    RUN (missing binary, OSError) is never fatal, because offline installs are
    legitimate. A tool that ran and did not deliver is fatal when the op declared a
    post-condition — that means our own skill names are wrong, and swallowing it is
    what let five weeks of installs silently install nothing.
    """
    console.run(op.target)
    if op.cmd is None:
        console.warn(f"{op.target}: no command")
        return op.optional
    try:
        ok = subprocess.run(op.cmd, check=False).returncode == 0
    except OSError as exc:
        console.skip(f"{op.target}: {exc} — tool unavailable (non-fatal)")
        return True
    if op.expect_skills:
        delivered = _delivered(op, root)
        if delivered and not ok:
            console.warn(f"{op.target}: exited non-zero but delivered — continuing")
        return delivered
    if ok:
        return True
    if op.optional:
        console.skip(f"{op.target} failed (non-fatal)")
        return True
    console.warn(f"{op.target} failed")
    return False


def apply(plan: Plan, root: Path) -> int:
    """Execute ADD/RUN ops; report SKIP/DEFER/WARN. Returns process-style code."""
    console.print_notices(plan)
    errors = 0
    for op in plan.ops:
        label = op.target + (f" — {op.detail}" if op.detail else "")
        d = op.disposition
        if d is Disposition.ADD:
            _apply_add(op)
            console.add(label)
        elif d is Disposition.RUN:
            errors += 0 if _apply_run(op, root) else 1
        elif d is Disposition.DEFER:
            console.defer(label)
        elif d is Disposition.SKIP:
            console.skip(label)
        elif d is Disposition.WARN:
            console.warn(label)
    return 1 if errors else 0

"""Skills manifest — CES-107.

``skills-lock.json`` is the tracked declaration of a repo's agent skills; the
``.agents/skills/`` tree is *derived* from it and is never the source of truth.
This is the ``pyproject.toml`` / ``uv.lock`` split, with the house baseline below
playing the role of the declared intent and the lock file the resolved set.

**The `skills` CLI owns this file. We read it; we never write it.** That is the
central constraint of this module, and it is not stylistic. The upstream schema
(``vercel-labs/skills``, ``src/local-lock.ts``) carries ``ref``, ``skillPath`` and
``computedHash`` per entry, and its own comments explain the cost of losing them:

    /** Path to the skill's SKILL.md within the source repo.
     *  Required to re-install only this skill on update — without it, an update
     *  would refetch every skill in the source repo. */

Re-serialising the file from a narrower Python model silently drops those fields.
The same source is explicit that the file is ours to commit:

    /** This file is meant to be checked into version control. */

What the lock does and does not buy, measured against ``skills@1.5.22`` rather
than assumed:

- ``ref`` records the branch or tag used at install time, and only when the source
  was pinned (``owner/repo#v1.2.3``). Tags are mutable, so this is tag-level, not
  commit-level, reproducibility.
- ``computedHash`` is written on ``add`` and compared only in ``sync.ts`` on the
  ``experimental_sync``/node_modules path. Restore never verifies it. There is no
  integrity guarantee.

So: reproducible in *which skills, from where, at what ref*. Not byte-reproducible.
Say that, and do not call it a lock in the ``uv.lock`` sense.

This module takes primitives rather than a ``Context`` so that it stays below
``components`` in the import order (CES-5) and is testable through its own seam.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import TYPE_CHECKING

from scaffolding.plan import Disposition, Op

if TYPE_CHECKING:
    from pathlib import Path

MANIFEST_FILE = "skills-lock.json"

# The engine may delete ONLY these literal lines from a .gitignore. Literals, never
# patterns — that is what keeps "clean-adds plus a whitelist" a checkable invariant.
UNIGNORE_WHITELIST = frozenset({MANIFEST_FILE})

# Pinned to an upstream release tag. Unpinned, `npx skills add` tracks the default
# branch, so an upstream rename lands silently in every scaffolded repo (v1.2.0
# renamed writing-great-skills -> writing-for-agents with no alias, and the installer
# skips unknown names without failing). Note the ref must be a `#fragment`: the
# `owner/repo@ref` form is parsed as a skill-name filter and the ref is ignored.
# The skills-drift workflow reports when a newer tag is available.
MATTPOCOCK_REF = "v1.2.3"
MATTPOCOCK_REPO = "mattpocock/skills"
MATTPOCOCK_SOURCE = f"{MATTPOCOCK_REPO}#{MATTPOCOCK_REF}"
SCAFFOLDING_SOURCE = "collectiveai-team/scaffolding"
VARLOCK_SOURCE = "dmno-dev/varlock"

MATTPOCOCK_SKILLS = [
    "grill-with-docs",
    "triage",
    "improve-codebase-architecture",
    "setup-matt-pocock-skills",
    "to-spec",
    "to-tickets",
    "implement",
    "wayfinder",
    "prototype",
    "diagnosing-bugs",
    "research",
    "tdd",
    "domain-modeling",
    "codebase-design",
    "code-review",
    "resolving-merge-conflicts",
    "wizard",
    "grill-me",
    "teach",
    "writing-for-agents",
    "grilling",
    "wait-what",
]
LOCAL_SKILLS = ["ask-user", "journalist", "handoff", "test-smell-review"]
VARLOCK_SKILLS = ["varlock"]


@dataclass(frozen=True)
class SkillEntry:
    """One skill in the *house baseline*: its name and where it comes from.

    This is our declared intent, not a row of the lock file. Nothing here is ever
    serialised into ``skills-lock.json`` — it only ever becomes ``skills add``
    arguments, and the CLI writes the resulting entry itself.
    """

    name: str
    source: str
    source_type: str = "github"


@dataclass(frozen=True)
class Manifest:
    """The declared skill set, read from the lock file.

    Names only. We compare against them and we plan from them; we never round-trip
    the file, so the fields we do not model cannot be lost.
    """

    names: frozenset[str]


@dataclass(frozen=True)
class AddCommand:
    """A single ``skills add`` invocation, grouped by source."""

    source: str
    names: list[str]
    argv: list[str]


@dataclass(frozen=True)
class SkillsPlanInput:
    """Everything the skills planner needs, decisions already resolved."""

    root: Path
    agent: str
    skills_dir: str
    manifest_ignored: bool
    unignore: bool = True


def house_baseline() -> list[SkillEntry]:
    """Return the skill set the scaffolder seeds a repo with.

    Used only when a repo has no lock file yet. It is a starting point, not a set
    the repo is held to: once the lock exists it is the source of truth, and a
    skill the repo dropped stays dropped (CES-30).
    """
    return [
        *(SkillEntry(n, MATTPOCOCK_SOURCE) for n in MATTPOCOCK_SKILLS),
        *(SkillEntry(n, SCAFFOLDING_SOURCE) for n in LOCAL_SKILLS),
        *(SkillEntry(n, VARLOCK_SOURCE) for n in VARLOCK_SKILLS),
    ]


def manifest_path(root: Path) -> Path:
    return root / MANIFEST_FILE


def read_manifest(root: Path) -> Manifest | None:
    """Parse the declared skill names, or None when absent/unreadable/malformed.

    ``None`` deliberately conflates "absent" and "unparseable". Callers must not
    treat it as "safe to create one" — use :func:`manifest_path` to tell the two
    apart, because overwriting a file we failed to parse is exactly the destructive
    move this module exists to avoid.
    """
    path = manifest_path(root)
    if not path.exists():
        return None
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (ValueError, OSError):
        return None
    if not isinstance(raw, dict):
        return None
    skills = raw.get("skills")
    if not isinstance(skills, dict):
        return None
    return Manifest(names=frozenset(str(name) for name in skills))


def installed_names(root: Path, skills_dir: str) -> list[str]:
    """Names in the derived tree. Read from disk — no subprocess, no network."""
    tree = root / skills_dir
    if not tree.is_dir():
        return []
    return sorted(p.name for p in tree.iterdir() if (p / "SKILL.md").is_file())


def ignores_literal_line(root: Path, line: str) -> bool:
    """Report whether ``<root>/.gitignore`` carries ``line`` as an exact literal entry.

    Detection for the unignore op must match what remediation can actually do.
    ``git check-ignore`` answers a broader question — it is satisfied by ``*.json``,
    by ``/skills-lock.json``, by ``.git/info/exclude`` — and acting on that answer
    with a literal line removal silently does nothing while reporting success.
    """
    path = root / ".gitignore"
    if not path.is_file():
        return False
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError:
        return False
    return any(existing.strip() == line for existing in raw.splitlines())


def add_commands(entries: list[SkillEntry], agent: str) -> list[AddCommand]:
    """Group entries into one ``skills add`` per source.

    Locally-sourced entries are skipped: they are authored in the working tree and
    restore reads them from there, so there is nothing to fetch.
    """
    by_source: dict[str, list[str]] = {}
    for entry in entries:
        if entry.source_type == "local":
            continue
        by_source.setdefault(entry.source, []).append(entry.name)
    return [
        AddCommand(
            source=source,
            names=sorted(names),
            argv=[
                "npx",
                "skills",
                "add",
                source,
                "--agent",
                agent,
                "--yes",
                "--skill",
                *sorted(names),
            ],
        )
        for source, names in sorted(by_source.items())
    ]


def restore_argv() -> list[str]:
    return ["npx", "skills", "experimental_install"]


def _restore_op(manifest: Manifest, skills_dir: str) -> Op:
    """Restore the declared set, and assert it actually landed on disk."""
    return Op(
        "skills",
        "run",
        f"restore declared skills ({skills_dir})",
        Disposition.RUN,
        cmd=restore_argv(),
        detail=f"{len(manifest.names)} declared in {MANIFEST_FILE}",
        expect_skills=sorted(manifest.names),
        expect_dir=skills_dir,
    )


def _seed_ops(entries: list[SkillEntry], agent: str, skills_dir: str) -> list[Op]:
    """Fetch ops that must actually deliver — post-condition asserted by the engine."""
    return [
        Op(
            "skills",
            "run",
            f"{cmd.source} ({skills_dir})",
            Disposition.RUN,
            cmd=cmd.argv,
            detail=f"{len(cmd.names)} skill" + ("s" if len(cmd.names) != 1 else ""),
            expect_skills=cmd.names,
            expect_dir=skills_dir,
        )
        for cmd in add_commands(entries, agent)
    ]


def _unignore_ops(inp: SkillsPlanInput) -> list[Op]:
    """Build the ops for a manifest that exists but that git is told to ignore."""
    if not inp.unignore:
        return [
            Op(
                "skills",
                "noop",
                ".gitignore",
                Disposition.WARN,
                detail=f"{MANIFEST_FILE} stays ignored — the manifest cannot be tracked",
            )
        ]
    if not ignores_literal_line(inp.root, MANIFEST_FILE):
        return [
            Op(
                "skills",
                "noop",
                ".gitignore",
                Disposition.WARN,
                detail=(
                    f"{MANIFEST_FILE} is ignored by a pattern this cannot safely edit "
                    f"(not a literal `{MANIFEST_FILE}` line in .gitignore) — "
                    "unignore it by hand so the manifest can be tracked"
                ),
            )
        ]
    return [
        Op(
            "skills",
            "unignore",
            ".gitignore",
            Disposition.ADD,
            path=str(inp.root / ".gitignore"),
            content=MANIFEST_FILE,
            detail=f"stop ignoring {MANIFEST_FILE} so the manifest can be tracked",
        )
    ]


def _replaced_ops(
    installed: list[str], baseline: list[SkillEntry], inp_skills_dir: str
) -> list[Op]:
    """Warn before seeding over a skill that is already on disk under the same name.

    Seeding runs ``skills add``, which overwrites the directory. A repo that
    hand-edited a skill sharing a baseline name would lose it silently, and the
    derived tree is gitignored, so there is no diff to notice afterwards. Warned
    rather than skipped: skipping would leave the skill installed and undeclared,
    which is the state this standard exists to remove.
    """
    known = {e.name for e in baseline}
    return [
        Op(
            "skills",
            "noop",
            f"{inp_skills_dir}/{name}",
            Disposition.WARN,
            detail=(
                "already on disk and will be replaced by the seed — if it was "
                "edited by hand, copy it out first; the derived tree is gitignored, "
                "so there is no diff to recover it from"
            ),
        )
        for name in installed
        if name in known
    ]


def _undeclared_ops(installed: list[str], baseline: list[SkillEntry]) -> list[Op]:
    """Warn about installed skills that seeding will not declare.

    Provenance is not recoverable from disk: an installed directory holds only
    ``SKILL.md``, and with no lock file ``skills list --json`` reports
    ``source: null``. We will not guess it, and we will not fabricate a lock entry
    — the fix is one ``npx skills add`` per skill, which lets the CLI record the
    source, ref and hash it actually used.
    """
    known = {e.name for e in baseline}
    return [
        Op(
            "skills",
            "noop",
            f"{MANIFEST_FILE}: {name}",
            Disposition.WARN,
            detail=(
                "installed but not in the house baseline, and its source cannot be "
                "recovered from disk — run `npx skills add <source> --skill "
                f"{name}` so the CLI declares it"
            ),
        )
        for name in installed
        if name not in known
    ]


def plan_manifest_ops(inp: SkillsPlanInput) -> list[Op]:
    """Two states: the repo has a lock file, or it does not.

    Lock present  -> restore from it. It is the source of truth, full stop: we do
                     not top it up towards the house baseline, because a skill the
                     repo removed is a decision, not a gap (CES-30). Adding one
                     back is ``npx skills add``.
    Lock absent   -> seed the house baseline, which makes the CLI write the lock.

    A third case exists only to be refused: a lock file that is present but does
    not parse. Seeding would make the CLI rewrite it, so we stop and say so.
    """
    manifest = read_manifest(inp.root)

    if manifest is None and manifest_path(inp.root).exists():
        return [
            Op(
                "skills",
                "noop",
                MANIFEST_FILE,
                Disposition.DEFER,
                detail="exists but is unreadable or malformed — fix or delete it by hand; "
                "installing would overwrite it",
            )
        ]

    if manifest is None:
        baseline = house_baseline()
        installed = installed_names(inp.root, inp.skills_dir)
        ops = _undeclared_ops(installed, baseline) + _replaced_ops(
            installed, baseline, inp.skills_dir
        )
        return [*ops, *_seed_ops(baseline, inp.agent, inp.skills_dir)]

    ops: list[Op] = []
    if inp.manifest_ignored:
        ops += _unignore_ops(inp)
    ops.append(_restore_op(manifest, inp.skills_dir))
    return ops

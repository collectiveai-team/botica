"""Plan / Op model and the pydantic report used at the JSON serialization boundary."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import StrEnum

from pydantic import BaseModel

from scaffolding.facts import Facts  # noqa: TC001 (pydantic PlanReport resolves Facts at runtime)


class Disposition(StrEnum):
    ADD = "add"
    SKIP = "skip"
    DEFER = "defer"
    RUN = "run"
    WARN = "warn"


class Agent(StrEnum):
    """Supported agent targets.

    opencode + codex follow the .agents/AGENTS.md standard; claude-code diverges
    (.claude/ + CLAUDE.md) and is bridged by symlink.
    """

    OPENCODE = "opencode"
    CLAUDE_CODE = "claude-code"
    CODEX = "codex"


@dataclass
class Op:
    """One planned operation. Plan computes everything; apply just executes."""

    component: str
    kind: str  # write | append | symlink | unignore | run | noop
    target: str
    disposition: Disposition
    detail: str = ""
    path: str | None = None
    content: str | None = None
    cmd: list[str] | None = None
    optional: bool = False
    # Post-condition for a run op (CES-107): skill names that MUST exist on disk
    # afterwards, and the directory to look in. Set => the op is fatal when it runs
    # and does not deliver, because a silently-empty install is the failure mode
    # this exists to catch. Asserted against the derived tree, not against the lock
    # file — asking the tool to confirm its own bookkeeping verifies nothing.
    expect_skills: list[str] | None = None
    expect_dir: str | None = None


@dataclass
class Decision:
    """A Tier-2/3 choice that must be made by the user (even agentic)."""

    tier: int
    key: str
    question: str
    default: str


class Decisions(BaseModel):
    """User answers to Tier-2/3 decisions; field names match ``Decision.key``."""

    agents: list[Agent] | None = None
    pyproject_name: str | None = None
    pyproject_description: str | None = None
    ci_parts: list[str] | None = None
    varlock: bool | None = None
    # CES-107. The only skills consent left: everything else is either the lock
    # file's decision or the `skills` CLI's, and neither is ours to override.
    skills_unignore: bool | None = None


class OpView(BaseModel):
    """Serialization-safe projection of an Op (omits file contents and cmd)."""

    component: str
    kind: str
    target: str
    disposition: Disposition
    detail: str


class PlanReport(BaseModel):
    """The pydantic model emitted by ``plan --json`` — the serialization boundary."""

    facts: Facts
    clean_adds: list[OpView]
    # Destructive edits to existing files, kept out of `clean_adds` so that field
    # keeps meaning what it says. Today this is only the CES-107 unignore op; the
    # ADR's promise is that you can enumerate them here.
    edits: list[OpView]
    runs: list[OpView]
    skips: list[OpView]
    defers: list[OpView]
    warnings: list[OpView]
    decisions_needed: list[Decision]
    deferred_merges: list[str]
    notices: list[str]


@dataclass
class Plan:
    facts: Facts
    ops: list[Op] = field(default_factory=list)
    decisions: list[Decision] = field(default_factory=list)
    notices: list[str] = field(default_factory=list)

    def by(self, disp: Disposition) -> list[Op]:
        return [o for o in self.ops if o.disposition == disp]

    @property
    def deferred_merges(self) -> list[str]:
        return [o.target for o in self.by(Disposition.DEFER)]

    def report(self) -> PlanReport:
        """Build the pydantic report consumed by ``plan --json``."""

        def view(disp: Disposition, *, kinds: set[str] | None = None) -> list[OpView]:
            return [
                OpView(
                    component=o.component,
                    kind=o.kind,
                    target=o.target,
                    disposition=o.disposition,
                    detail=o.detail,
                )
                for o in self.by(disp)
                if kinds is None or o.kind in kinds
            ]

        edits = {"unignore"}
        return PlanReport(
            facts=self.facts,
            clean_adds=[o for o in view(Disposition.ADD) if o.kind not in edits],
            edits=view(Disposition.ADD, kinds=edits),
            runs=view(Disposition.RUN),
            skips=view(Disposition.SKIP),
            defers=view(Disposition.DEFER),
            warnings=view(Disposition.WARN),
            decisions_needed=self.decisions,
            deferred_merges=self.deferred_merges,
            notices=self.notices,
        )

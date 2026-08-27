"""Component registry: each former ensure_* step as a selectable component.

Every component exposes ``plan(ctx) -> list[Op]`` that inspects the filesystem
and facts and returns ops WITHOUT side effects (apply executes them). Plan may
register Tier-2/3 ``Decision``s via ``ctx.add_decision``.
"""

from __future__ import annotations

import contextlib
import subprocess
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from scaffolding.agent_config import (
    AGENTS_SKILLS_DIR,
    plan_agent_config,
    register_agents_decision,
)
from scaffolding.facts import gitignored
from scaffolding.ops import write_if_absent
from scaffolding.plan import Agent, Decision, Decisions, Disposition, Op
from scaffolding.skills import (
    MANIFEST_FILE,
    SkillsPlanInput,
    manifest_path,
    plan_manifest_ops,
)
from scaffolding.templates_registry import template_text

if TYPE_CHECKING:
    from collections.abc import Callable
    from pathlib import Path

    from scaffolding.facts import Facts
    from scaffolding.settings import Settings

# .agents/skills/ is DERIVED from the tracked skills manifest (CES-107), so it is
# ignored. skills-lock.json itself must never appear here — it is the tracked
# declaration, and the `skills` component will offer to un-ignore it if it is.
GITIGNORE_ENTRIES = [
    ".env",
    "!.env.schema",
    ".tmp/",
    ".scratch/",
    ".worktrees/",
    ".journals/",
    ".agents/skills/",
]
AGENTS_MARKER = "## Repo Workspace Defaults"
# Marker-gated section the `standards` component owns in AGENTS.md (sibling to AGENTS_MARKER).
STANDARDS_MARKER = "## Engineering Standards"
# Per-CES detail files under .agents/rules/ shipped by the standards component. One file per
# standard (a CES may ship several ast-grep/prek slugs but documents them in one detail file).
STANDARDS_RULE_DETAILS = [
    "no-dict",
    "file-size-guard",
    "skills-manifest",
    "log-get-logger",
    "log-no-print",
    "core-logger",
    "api-schemas-extra-forbid",
    "settings-module",
    "cli-typed-framework",
    "arch-database-package",
    "no-utils",
    "repo-shape",
    "agents-conventional-commits",
    "import-linter",
    "api-boundary-layout",
    "arch-vocabulary",
    "spaghetti-mixed-orchestration",
    "general-respect-local-repo",
    "py-legacy-lint-stack",
    "test-in-memory-adapters",
    "test-through-interface",
    "test-coverage-gap",
    "no-ai-coauthorship",
    "dep-hygiene-deptry",
    "cognitive-complexity-complexipy",
    "test-order-randomization-pytest-randomly",
    "dependency-review-action",
    "hadolint-dockerfile-lint",
    "code-duplication-jscpd",
    "osv-scanner-replace-pip-audit",
]
# Canonical drop-in / comparison code shipped under snippets/ (may be nested, e.g. core/logger.py).
STANDARDS_SNIPPETS = [
    "no-dict-boundary.py",
    "core/logger.py",
    "api-schemas.py",
    "settings.py",
    "tests/in_memory_repository.py",
]
ASTGREP_RULES = [
    "no-dict-call-return",
    "no-dict-literal-return",
    "no-dict-return-annotation",
    "no-dict-alias",
    "log-get-logger",
    "log-no-print",
    "api-schemas-extra-forbid",
    "settings-module",
    "cli-typed-framework",
    "arch-database-package",
]
# MATTPOCOCK_REF / MATTPOCOCK_SKILLS / LOCAL_SKILLS now live in scaffolding/skills.py,
# next to the manifest logic that consumes them (CES-107).
DEFAULT_CI_PARTS = ["tests", "security", "docker"]
# "opencode" is opt-in only (off by default): it needs repo secrets and the
# OpenCode GitHub App installed, so it is never added unless explicitly chosen.
ALL_CI_PARTS = ["tests", "security", "docker", "publish", "opencode"]


@dataclass
class Context:
    root: Path
    facts: Facts
    settings: Settings
    decisions: Decisions
    interactive: bool
    _decisions: list[Decision] = field(default_factory=list)

    @property
    def guide_url(self) -> str:
        return f"{self.settings.raw_base}/guide.md"

    def add_decision(self, d: Decision) -> None:
        if all(existing.key != d.key for existing in self._decisions):
            self._decisions.append(d)


@dataclass
class Component:
    name: str
    summary: str
    tier: int
    default_on: bool
    gate: Callable[[Facts], bool]
    plan: Callable[[Context], list[Op]]
    deps: Callable[[Facts], list[str]] = field(default=lambda f: [])


# --- helpers -----------------------------------------------------------------
def _norm_name(s: str) -> str:
    return s.strip().lower().replace(" ", "-").replace("_", "-")


def infer_project_name(root: Path) -> str:
    with contextlib.suppress(OSError, subprocess.SubprocessError):
        out = subprocess.run(
            ["git", "remote", "get-url", "origin"],
            cwd=root,
            capture_output=True,
            text=True,
            timeout=5,
            check=False,
        )
        if out.returncode == 0 and out.stdout.strip():
            tail = out.stdout.strip().rstrip("/").split("/")[-1].removesuffix(".git")
            if tail:
                return _norm_name(tail)
    return _norm_name(root.name)


# --- gates -------------------------------------------------------------------
def _always(_f: Facts) -> bool:
    return True


def _python(f: Facts) -> bool:
    return f.is_python


# --- component plans ---------------------------------------------------------
def plan_gitignore(ctx: Context) -> list[Op]:
    dest = ctx.root / ".gitignore"
    if not dest.exists():
        content = "\n".join(GITIGNORE_ENTRIES) + "\n"
        return [
            Op(
                "gitignore",
                "write",
                ".gitignore",
                Disposition.ADD,
                path=str(dest),
                content=content,
                detail="create with workspace entries",
            )
        ]
    present = {ln.rstrip() for ln in dest.read_text(encoding="utf-8").splitlines()}
    missing = [e for e in GITIGNORE_ENTRIES if e not in present]
    if not missing:
        return [
            Op("gitignore", "noop", ".gitignore", Disposition.SKIP, detail="all entries present")
        ]
    return [
        Op(
            "gitignore",
            "append",
            ".gitignore",
            Disposition.ADD,
            path=str(dest),
            content="\n".join(missing) + "\n",
            detail="add: " + ", ".join(missing),
        )
    ]


def plan_prek(ctx: Context) -> list[Op]:
    dest = ctx.root / "prek.toml"
    if dest.exists():
        return [
            Op(
                "prek",
                "noop",
                "prek.toml",
                Disposition.DEFER,
                detail=f"already exists — merge via guide: {ctx.guide_url}",
            )
        ]
    content = template_text("prek-generic.toml")
    detail = "generic hooks"
    if ctx.facts.is_python:
        content += "\n" + template_text("prek-python.toml")
        detail = "generic + python hooks"
    return [
        Op(
            "prek",
            "write",
            "prek.toml",
            Disposition.ADD,
            path=str(dest),
            content=content,
            detail=detail,
        )
    ]


def plan_astgrep(ctx: Context) -> list[Op]:
    ops = [
        write_if_absent(
            "ast-grep",
            ctx.root / "sgconfig.yml",
            template_text("sgconfig-template.yml"),
            "sgconfig.yml",
            ctx.guide_url,
        )
    ]
    for rule in ASTGREP_RULES:
        dest = ctx.root / "ast-grep" / "rules" / f"{rule}.yml"
        ops.append(
            write_if_absent(
                "ast-grep",
                dest,
                template_text(f"ast-grep-rules/{rule}.yml"),
                f"ast-grep/rules/{rule}.yml",
                ctx.guide_url,
            )
        )
    return ops


def plan_pyproject(ctx: Context) -> list[Op]:
    dest = ctx.root / "pyproject.toml"
    if dest.exists():
        return [
            Op(
                "pyproject",
                "noop",
                "pyproject.toml",
                Disposition.DEFER,
                detail=f"already exists — merge via guide: {ctx.guide_url}",
            )
        ]
    name = ctx.decisions.pyproject_name or infer_project_name(ctx.root)
    desc = ctx.decisions.pyproject_description or f"{name} project"
    ctx.add_decision(Decision(2, "pyproject_name", "Project name?", name))
    ctx.add_decision(Decision(2, "pyproject_description", "Project description?", desc))
    content = (
        template_text("pyproject-template.toml")
        .replace('name = "replace-me"', f'name = "{name}"')
        .replace('description = "Replace me"', f'description = "{desc}"')
    )
    return [
        Op(
            "pyproject",
            "write",
            "pyproject.toml",
            Disposition.ADD,
            path=str(dest),
            content=content,
            detail=f"name={name}",
        )
    ]


def _ci_workflow_op(ctx: Context, rel: str) -> Op:
    return write_if_absent(
        "ci",
        ctx.root / f".github/{rel}",
        template_text(f"github/{rel}"),
        f".github/{rel}",
        ctx.guide_url,
    )


def _ci_always_on_ops(ctx: Context) -> list[Op]:
    # CES-75 / CES-91: Conventional Commits + no-ai-coauthorship PR checks ship whenever CI is
    # set up, independent of parts (they mirror the always-on commit-msg prek hooks).
    return [
        _ci_workflow_op(ctx, "workflows/conventional-commits.yml"),
        _ci_workflow_op(ctx, "workflows/commit-policy.yml"),
    ]


def _ci_dependency_review_ops(ctx: Context) -> list[Op]:
    # CES-113: free for public repos (just needs "Dependency graph" enabled — a web-UI-only
    # toggle, no API), but on PRIVATE/internal repos the action requires GitHub Advanced
    # Security / Code Security, a paid tier. This tool is tier-agnostic (works on free public
    # AND free private repos without assuming a paid upgrade — see docker.yml's identical GHCR
    # billing skip below), so only ship it where it's actually free.
    if ctx.facts.visibility in ("private", "internal"):
        return [
            Op(
                "ci",
                "noop",
                ".github/workflows/dependency-review.yml",
                Disposition.SKIP,
                detail="repo non-public — dependency-review-action requires GitHub Advanced "
                "Security/Code Security (paid) on private repos. Skipped; add manually if your "
                "plan includes it.",
            )
        ]
    return [_ci_workflow_op(ctx, "workflows/dependency-review.yml")]


def _ci_security_ops(ctx: Context) -> list[Op]:
    ops = [
        _ci_workflow_op(ctx, "workflows/zizmor.yml"),
        _ci_workflow_op(ctx, "zizmor.yml"),
        *_ci_dependency_review_ops(ctx),
    ]
    # CES-119: osv-scanner reads uv.lock; gated on Python until a non-Python lockfile template
    # exists (see the CES-119 detail file).
    if ctx.facts.is_python:
        ops.append(_ci_workflow_op(ctx, "workflows/osv-scanner.yml"))
    return ops


def _ci_tests_ops(ctx: Context) -> list[Op]:
    if not ctx.facts.is_python:
        return []
    return [
        _ci_workflow_op(ctx, "workflows/tests.yml"),
        _ci_workflow_op(ctx, "dependabot.yml"),
    ]


def _ci_docker_ops(ctx: Context) -> list[Op]:
    if not ctx.facts.has_dockerfile:
        return []
    if ctx.facts.visibility in ("private", "internal"):
        return [
            Op(
                "ci",
                "noop",
                ".github/workflows/docker.yml",
                Disposition.SKIP,
                detail="repo non-public — docker.yml pushes to GHCR (bills private "
                "packages past the free tier). Skipped; add manually if intended.",
            )
        ]
    return [_ci_workflow_op(ctx, "workflows/docker.yml")]


def _ci_publish_ops(ctx: Context) -> list[Op]:
    return [_ci_workflow_op(ctx, f"workflows/{wf}") for wf in ("release.yml", "pypi.yml")]


def _ci_opencode_ops(ctx: Context) -> list[Op]:
    return [
        _ci_workflow_op(ctx, f"workflows/{wf}") for wf in ("opencode.yml", "proposal-update.yml")
    ]


# Each entry ships only when its key is in the selected `ci_parts`.
_CI_PART_PLANNERS: dict[str, Callable[[Context], list[Op]]] = {
    "security": _ci_security_ops,
    "tests": _ci_tests_ops,
    "docker": _ci_docker_ops,
    "publish": _ci_publish_ops,
    "opencode": _ci_opencode_ops,
}


def plan_ci(ctx: Context) -> list[Op]:
    parts = ctx.decisions.ci_parts or DEFAULT_CI_PARTS
    ctx.add_decision(
        Decision(
            2,
            "ci_parts",
            "Which CI parts? (tests,security,docker,publish,opencode)",
            ",".join(DEFAULT_CI_PARTS),
        )
    )
    ops = _ci_always_on_ops(ctx)
    for part, planner in _CI_PART_PLANNERS.items():
        if part in parts:
            ops += planner(ctx)
    if not ops:
        ops.append(Op("ci", "noop", "ci", Disposition.SKIP, detail="no applicable CI parts"))
    return ops


def plan_agents(ctx: Context) -> list[Op]:
    dest = ctx.root / "AGENTS.md"
    section = template_text("agents-workspace-defaults.md")
    if dest.exists():
        if AGENTS_MARKER in dest.read_text(encoding="utf-8"):
            return [
                Op(
                    "agents",
                    "noop",
                    "AGENTS.md",
                    Disposition.SKIP,
                    detail=f"{AGENTS_MARKER} already present",
                )
            ]
        return [
            Op(
                "agents",
                "append",
                "AGENTS.md",
                Disposition.ADD,
                path=str(dest),
                content="\n" + section,
                detail="append workspace-defaults section",
            )
        ]
    return [
        Op(
            "agents",
            "write",
            "AGENTS.md",
            Disposition.ADD,
            path=str(dest),
            content="# Agent Notes\n\n" + section,
            detail="create with section",
        )
    ]


def _standards_index_op(ctx: Context) -> Op:
    """Append the marker-gated ## Engineering Standards index to AGENTS.md (idempotent)."""
    dest = ctx.root / "AGENTS.md"
    section = template_text("standards-index.md")
    if dest.exists() and STANDARDS_MARKER in dest.read_text(encoding="utf-8"):
        return Op(
            "standards",
            "noop",
            "AGENTS.md",
            Disposition.SKIP,
            detail=f"{STANDARDS_MARKER} already present",
        )
    # Always append (never create/overwrite): the `agents` dep creates the AGENTS.md base, and
    # _apply_append tolerates a missing file, so this can never clobber an existing AGENTS.md.
    return Op(
        "standards",
        "append",
        "AGENTS.md",
        Disposition.ADD,
        path=str(dest),
        content="\n" + section,
        detail="append Engineering Standards index",
    )


def plan_standards(ctx: Context) -> list[Op]:
    ops = [_standards_index_op(ctx)]
    ops += [
        write_if_absent(
            "standards",
            ctx.root / ".agents" / "rules" / f"{slug}.md",
            template_text(f"agents-rules/{slug}.md"),
            f".agents/rules/{slug}.md",
            ctx.guide_url,
        )
        for slug in STANDARDS_RULE_DETAILS
    ]
    ops += [
        write_if_absent(
            "standards",
            ctx.root / ".agents" / "snippets" / snippet,
            template_text(f"snippets/{snippet}"),
            f".agents/snippets/{snippet}",
            ctx.guide_url,
        )
        for snippet in STANDARDS_SNIPPETS
    ]
    return ops


def _decided(answer: bool | None, default: bool) -> bool:
    """Unanswered decisions take their default — merging, per CES-107."""
    return default if answer is None else answer


def _register_skills_decisions(ctx: Context, *, ignored: bool) -> None:
    """Ask before editing a file the repo owns (CES-30).

    Exactly one question, and only when the repo is in the state that raises it.
    There is nothing to ask about the skill set itself: if a lock file exists it is
    the source of truth, and if it does not the repo is being seeded.
    """
    if ignored:
        ctx.add_decision(
            Decision(
                2,
                "skills_unignore",
                f"Stop gitignoring {MANIFEST_FILE} so the skills manifest can be tracked?",
                "yes",
            )
        )


def plan_skills(ctx: Context) -> list[Op]:
    if ctx.settings.skip_skills:
        return [Op("skills", "noop", "skills", Disposition.SKIP, detail="SKIP_SKILLS set")]
    if not ctx.facts.has_npx:
        return [Op("skills", "noop", "skills", Disposition.SKIP, detail="npx not found")]
    register_agents_decision(ctx)

    ignored = manifest_path(ctx.root).exists() and gitignored(ctx.root, MANIFEST_FILE)
    _register_skills_decisions(ctx, ignored=ignored)

    # Skills install ONCE into the shared .agents/skills standard (read by opencode +
    # codex). Claude reaches the same files via the .claude/skills -> .agents/skills
    # symlink that agent-config creates, so this is never re-run per agent.
    return plan_manifest_ops(
        SkillsPlanInput(
            root=ctx.root,
            agent=Agent.OPENCODE.value,
            skills_dir=AGENTS_SKILLS_DIR,
            manifest_ignored=ignored,
            unignore=_decided(ctx.decisions.skills_unignore, True),
        )
    )


def plan_varlock(ctx: Context) -> list[Op]:
    if ctx.settings.skip_varlock:
        return [Op("varlock", "noop", "varlock", Disposition.SKIP, detail="SKIP_VARLOCK set")]
    if (ctx.root / ".env.schema").exists():
        return [
            Op(
                "varlock",
                "noop",
                ".env.schema",
                Disposition.DEFER,
                detail="exists — skipping varlock init to avoid rewriting it",
            )
        ]
    if (ctx.root / ".env.example").exists():
        ctx.add_decision(
            Decision(
                3, "varlock", "Run varlock init? .env.example exists and may be rewritten", "no"
            )
        )
        if not ctx.decisions.varlock:
            return [
                Op(
                    "varlock",
                    "noop",
                    "varlock init",
                    Disposition.SKIP,
                    detail=".env.example exists — pass --varlock to run init",
                )
            ]
    if ctx.facts.has_varlock:
        cmd = ["varlock", "init"]
    elif ctx.facts.has_npx:
        cmd = ["npx", "varlock", "init", "--agent"]
    else:
        return [Op("varlock", "noop", "varlock", Disposition.SKIP, detail="varlock/npx not found")]
    return [
        Op(
            "varlock",
            "run",
            "varlock init",
            Disposition.RUN,
            cmd=cmd,
            optional=True,
            detail="agent-agnostic; runtime redaction (opencode-varlock) is opencode-only — "
            "claude/codex rely on the deny-list/guidance",
        )
    ]


# --- registry ----------------------------------------------------------------
REGISTRY: list[Component] = [
    Component("gitignore", "Workspace .gitignore entries", 1, True, _always, plan_gitignore),
    Component(
        "agent-config",
        "Per-agent config (opencode.jsonc / .claude + CLAUDE.md / codex notice)",
        1,
        True,
        _always,
        plan_agent_config,
        deps=lambda f: ["agents"],
    ),
    Component(
        "prek",
        "prek.toml hooks (generic + python)",
        1,
        True,
        _always,
        plan_prek,
        deps=lambda f: ["ast-grep"] if f.is_python else [],
    ),
    Component("ast-grep", "sgconfig.yml + starter rules", 1, True, _python, plan_astgrep),
    Component(
        "pyproject", "pyproject.toml (guided name/description)", 2, True, _python, plan_pyproject
    ),
    Component("ci", "GitHub Actions workflows + dependabot", 2, False, _always, plan_ci),
    Component("agents", "AGENTS.md workspace-defaults section", 1, True, _always, plan_agents),
    Component(
        "standards",
        "Engineering Standards index + .agents/rules detail + snippets",
        1,
        True,
        _python,
        plan_standards,
        deps=lambda f: ["agents"],
    ),
    Component("skills", "Install curated agent skills via npx", 2, True, _always, plan_skills),
    Component("varlock", "Varlock secret schema init", 3, True, _always, plan_varlock),
]

BY_NAME = {c.name: c for c in REGISTRY}


def lookup(name: str) -> Component | None:
    return BY_NAME.get(name)

"""`check` — completeness verification from the guide Verify checklist."""

from __future__ import annotations

import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

from scaffolding.agent_config import AGENTS_SKILLS_DIR
from scaffolding.components import AGENTS_MARKER, GITIGNORE_ENTRIES
from scaffolding.skills import MANIFEST_FILE, installed_names, read_manifest


@dataclass
class CheckResult:
    name: str
    ok: bool
    detail: str


def _strip_jsonc(text: str) -> str:
    # remove // line comments and /* */ block comments, then trailing commas
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)
    text = re.sub(r"(^|\s)//[^\n]*", r"\1", text)
    return re.sub(r",(\s*[}\]])", r"\1", text)


def _git_tracked(root: Path, rel: str) -> bool:
    try:
        out = subprocess.run(
            ["git", "ls-files", "--error-unmatch", rel],
            cwd=root,
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        return out.returncode == 0
    except Exception:
        return False


def _gitignored(root: Path, rel: str) -> bool:
    try:
        out = subprocess.run(
            ["git", "check-ignore", "-q", rel],
            cwd=root,
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        return out.returncode == 0
    except Exception:
        return False


def _check_opencode(root: Path) -> list[CheckResult]:
    oc = root / "opencode.jsonc"
    if not oc.exists():
        return []
    try:
        data = json.loads(_strip_jsonc(oc.read_text(encoding="utf-8")))
        need = [k for k in ("$schema", "plugin", "permission") if k not in data]
        ok = not need
        detail = "ok" if ok else f"missing keys: {', '.join(need)}"
        return [CheckResult("opencode.jsonc valid", ok, detail)]
    except (ValueError, OSError) as exc:
        return [CheckResult("opencode.jsonc valid", False, f"parse error: {exc}")]


def _check_claude(root: Path) -> list[CheckResult]:
    out: list[CheckResult] = []
    settings = root / ".claude" / "settings.json"
    if settings.exists():
        try:
            data = json.loads(settings.read_text(encoding="utf-8"))
            deny = data.get("permissions", {}).get("deny", [])
            ok = any(".env" in str(rule) for rule in deny)
            out.append(
                CheckResult(
                    ".claude/settings.json valid",
                    ok,
                    "ok" if ok else "permissions.deny missing .env rules",
                )
            )
        except (ValueError, OSError) as exc:
            out.append(CheckResult(".claude/settings.json valid", False, f"parse error: {exc}"))

    claude_md = root / "CLAUDE.md"
    if claude_md.is_symlink() or claude_md.exists():
        bridged = (claude_md.is_symlink() and "AGENTS.md" in str(claude_md.readlink())) or (
            claude_md.is_file() and "AGENTS.md" in claude_md.read_text(encoding="utf-8")
        )
        out.append(
            CheckResult(
                "CLAUDE.md -> AGENTS.md",
                bridged,
                "bridged" if bridged else "CLAUDE.md does not reference AGENTS.md",
            )
        )
    return out


def _agent_config_checks(root: Path) -> list[CheckResult]:
    return _check_opencode(root) + _check_claude(root)


def _check_skills_manifest(root: Path) -> list[CheckResult]:
    """CES-107: the manifest is tracked, and the derived tree matches what it declares."""
    manifest = read_manifest(root)
    if manifest is None:
        return [
            CheckResult(
                "skills manifest",
                False,
                f"{MANIFEST_FILE} missing or unreadable — run `scaffolding install skills`",
            )
        ]

    out: list[CheckResult] = []
    ignored = _gitignored(root, MANIFEST_FILE)
    out.append(
        CheckResult(
            "skills manifest not ignored",
            not ignored,
            "ok" if not ignored else f"remove {MANIFEST_FILE} from .gitignore — it must be tracked",
        )
    )
    # Tracking is checked separately from ignoring: the engine never touches the git
    # index, so `git add` stays a human act that this check enforces.
    tracked = _git_tracked(root, MANIFEST_FILE)
    out.append(
        CheckResult(
            "skills manifest tracked",
            tracked,
            "tracked" if tracked else f"exists but untracked — run `git add {MANIFEST_FILE}`",
        )
    )

    # Drift is checked in both directions. A declared-but-absent skill means the
    # tree is stale; an installed-but-undeclared one means the repo has a skill that
    # evaporates on a fresh clone, since .agents/skills/ is gitignored. That second
    # case is the failure mode this standard exists to eliminate, so it cannot be
    # the one direction we do not look at.
    installed = set(installed_names(root, AGENTS_SKILLS_DIR))
    missing = sorted(manifest.names - installed)
    out.append(
        CheckResult(
            "declared skills installed",
            not missing,
            "ok"
            if not missing
            else f"declared but not in {AGENTS_SKILLS_DIR}: {', '.join(missing)}",
        )
    )
    extra = sorted(installed - manifest.names)
    out.append(
        CheckResult(
            "installed skills declared",
            not extra,
            "ok"
            if not extra
            else f"in {AGENTS_SKILLS_DIR} but not in {MANIFEST_FILE}: {', '.join(extra)} — "
            "run `npx skills add <source> --skill <name>` to declare them",
        )
    )
    return out


def _check_gitignore(root: Path) -> CheckResult:
    gi = root / ".gitignore"
    if not gi.exists():
        return CheckResult(".gitignore entries", False, ".gitignore missing")
    present = {ln.rstrip() for ln in gi.read_text(encoding="utf-8").splitlines()}
    missing = [e for e in GITIGNORE_ENTRIES if e not in present]
    detail = "all present" if not missing else f"missing: {', '.join(missing)}"
    return CheckResult(".gitignore entries", not missing, detail)


def _check_prek(root: Path) -> CheckResult:
    prek = root / "prek.toml"
    if not prek.exists():
        return CheckResult("prek.toml", False, "prek.toml missing")
    has_betterleaks = "betterleaks" in prek.read_text(encoding="utf-8")
    detail = "present" if has_betterleaks else "betterleaks hook missing"
    return CheckResult("prek betterleaks hook", has_betterleaks, detail)


def _check_env_schema(root: Path) -> CheckResult:
    schema = root / ".env.schema"
    if not schema.exists():
        return CheckResult(".env.schema", False, "missing (run varlock)")
    tracked = _git_tracked(root, ".env.schema")
    detail = "tracked" if tracked else "exists but not tracked by git"
    return CheckResult(".env.schema tracked", tracked, detail)


def _check_env_ignored(root: Path) -> CheckResult:
    ok = _gitignored(root, ".env")
    return CheckResult(".env ignored", ok, "ignored" if ok else ".env is not gitignored")


def _check_astgrep(root: Path) -> CheckResult | None:
    prek = root / "prek.toml"
    prek_has_astgrep = prek.exists() and "ast-grep" in prek.read_text(encoding="utf-8")
    if not prek_has_astgrep:
        return None
    rules_dir = root / "ast-grep" / "rules"
    rules = list(rules_dir.glob("*.yml")) if rules_dir.exists() else []
    ok = (root / "sgconfig.yml").exists() and bool(rules)
    detail = "ok" if ok else "ast-grep hook present but sgconfig.yml/rules missing"
    return CheckResult("ast-grep config", ok, detail)


def _check_agents_md(root: Path) -> CheckResult:
    am = root / "AGENTS.md"
    if not am.exists():
        return CheckResult("AGENTS.md", False, "missing")
    has = AGENTS_MARKER in am.read_text(encoding="utf-8")
    detail = "present" if has else f"{AGENTS_MARKER} missing"
    return CheckResult("AGENTS.md section", has, detail)


def run_checks(root: Path | None = None) -> list[CheckResult]:
    root = root or Path.cwd()
    # Agent config is per-agent and optional: validate whatever is present rather than
    # requiring opencode.jsonc. AGENTS.md is the only universal requirement. `_check_astgrep`
    # returns None (filtered out) when the ast-grep hook isn't present in prek.toml.
    checks: list[CheckResult | None] = [
        _check_gitignore(root),
        _check_prek(root),
        *_agent_config_checks(root),
        _check_env_schema(root),
        _check_env_ignored(root),
        _check_astgrep(root),
        *_check_skills_manifest(root),
        _check_agents_md(root),
    ]
    return [c for c in checks if c is not None]

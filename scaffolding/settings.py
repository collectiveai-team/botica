"""Validated config: env vars (back-compat) + CLI flag overrides + defaults."""

from __future__ import annotations

from pydantic_settings import BaseSettings, SettingsConfigDict

DEFAULT_RAW_BASE = "https://raw.githubusercontent.com/collectiveai-team/botica/main"


class Settings(BaseSettings):
    """Field names match the legacy env vars case-insensitively.

    e.g. ``agent`` reads ``AGENT``, ``skip_skills`` reads ``SKIP_SKILLS``.
    """

    model_config = SettingsConfigDict(
        env_prefix="",
        case_sensitive=False,
        extra="ignore",
    )

    # Legacy single-agent env var (AGENT). Prefer ``agents`` (the resolved list).
    agent: str = "opencode"
    assume_yes: bool = False
    with_ci: bool = False
    skip_ci: bool = False
    skip_skills: bool = False
    skip_varlock: bool = False
    no_deps: bool = False
    raw_base: str = DEFAULT_RAW_BASE

    @property
    def agents(self) -> list[str]:
        """Resolved agent targets, parsed from the legacy ``agent`` value.

        Accepts comma/slash-separated values (e.g. ``opencode,codex``); falls back
        to ``["opencode"]`` when empty.
        """
        raw = [a.strip().lower() for a in self.agent.replace("/", ",").split(",")]
        return list(dict.fromkeys([a for a in raw if a])) or ["opencode"]

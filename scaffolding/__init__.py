"""scaffolding — deterministic, clean-adds-only repo bootstrap CLI.

The CLI is the deterministic engine for repo bootstrap; an agent drives it for
the merge/judgment cases. It never edits, merges, or overwrites existing files:
existing targets are deferred to the agentic guide.
"""

from importlib.metadata import PackageNotFoundError, version

try:
    # hatch-vcs derives the version from the git tag at build time, so the
    # installed distribution's metadata is the only place it is correct. A
    # hardcoded literal here silently drifts from the tag — it did, sitting at
    # 0.1.0 through two releases.
    __version__ = version("scaffolding")
except PackageNotFoundError:  # pragma: no cover — running from an uninstalled tree
    __version__ = "0+unknown"

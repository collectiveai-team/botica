from pathlib import Path

CACHE = Path.home() / ".cache" / "app"
CACHE.mkdir(parents=True, exist_ok=True)

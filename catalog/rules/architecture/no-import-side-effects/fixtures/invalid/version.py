from pathlib import Path

__version__ = Path(__file__).with_name("VERSION").read_text().strip()

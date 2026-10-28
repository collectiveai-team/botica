import os

from sqlalchemy import create_engine

engine = create_engine(os.environ["DATABASE_URL"])
LOG_LEVEL = os.environ.get("LOG_LEVEL", "INFO")
HAS_CONFIG = os.path.exists("app.toml")
read = lambda path: open(path).read()  # noqa: E731

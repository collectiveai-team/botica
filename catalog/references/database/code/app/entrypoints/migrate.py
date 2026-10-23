"""The migration step of a deployment: `python -m app.entrypoints.migrate`, before the API runs."""

from app.database.services.migrations import upgrade_to_head


def main() -> None:
    """Bring the database of the settings to the last migration."""
    upgrade_to_head()


if __name__ == "__main__":
    main()

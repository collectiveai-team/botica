from alembic import command


def migrate(config) -> None:
    command.upgrade(config, "head")

from sqlmodel import SQLModel


def build(engine) -> None:
    SQLModel.metadata.create_all(engine)

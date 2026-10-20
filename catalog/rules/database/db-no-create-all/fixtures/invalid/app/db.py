from sqlmodel import SQLModel

from app.engine import engine

SQLModel.metadata.create_all(engine)
Base.metadata.create_all(bind=engine)

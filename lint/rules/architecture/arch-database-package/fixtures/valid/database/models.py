from sqlalchemy.orm import sessionmaker
from sqlmodel import Field, SQLModel, create_engine


class User(SQLModel, table=True):
    id: int = Field(primary_key=True)


engine = create_engine("sqlite://")
Session = sessionmaker(engine)

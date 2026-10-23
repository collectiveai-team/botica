"""The endpoints of users over the SQL repository, with the database of the test."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient

from app.api.v1.routers.users import router
from app.database.repositories.users import SqlUserRepository
from app.database.services.session import get_user_repository

if TYPE_CHECKING:
    from sqlmodel import Session


@pytest.fixture
def client(session: Session) -> TestClient:
    app = FastAPI()
    app.include_router(router, prefix="/v1")
    app.dependency_overrides[get_user_repository] = lambda: SqlUserRepository(session)
    return TestClient(app)


def test_a_registered_user_comes_back_with_the_fields_of_the_response(client: TestClient):
    created = client.post("/v1/users", json={"email": "ana@example.com"})

    assert created.status_code == 201
    assert created.json() == {"id": 1, "email": "ana@example.com"}
    assert client.get("/v1/users/ana@example.com").json() == created.json()


def test_an_email_that_is_taken_is_a_409(client: TestClient):
    client.post("/v1/users", json={"email": "ana@example.com"})

    assert client.post("/v1/users", json={"email": "ana@example.com"}).status_code == 409


def test_an_unknown_email_is_a_404(client: TestClient):
    assert client.get("/v1/users/nobody@example.com").status_code == 404

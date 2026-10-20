"""The schemas of the topic, through the endpoint that uses them."""

from __future__ import annotations

from dataclasses import dataclass

import pytest
from fastapi import FastAPI
from fastapi.testclient import TestClient

from app.api.v1.routers.users import router
from app.api.v1.schemas.responses.users import UserResponse


@pytest.fixture
def client() -> TestClient:
    app = FastAPI()
    app.include_router(router, prefix="/v1")
    return TestClient(app, raise_server_exceptions=False)


def test_a_valid_request_returns_the_user(client: TestClient):
    response = client.post("/v1/users", json={"email": "a@b.co", "display_name": "Ana"})

    assert response.status_code == 201
    body = response.json()
    assert set(body) == {"id", "email", "display_name"}
    assert (body["email"], body["display_name"]) == ("a@b.co", "Ana")


def test_a_key_that_the_request_does_not_declare_is_a_422(client: TestClient):
    response = client.post(
        "/v1/users", json={"email": "a@b.co", "display_name": "Ana", "is_admin": True}
    )

    assert response.status_code == 422
    assert response.json()["detail"][0]["type"] == "extra_forbidden"


def test_a_handler_that_returns_a_dict_with_an_extra_key_fails_with_a_500():
    app = FastAPI()

    stored = {"id": 1, "email": "a@b.co", "display_name": "Ana", "password": "x"}

    @app.get("/leak")
    def leak() -> UserResponse:
        return stored  # pyrefly: ignore[bad-return]

    assert TestClient(app, raise_server_exceptions=False).get("/leak").status_code == 500


def test_from_attributes_ignores_extra_forbid():
    @dataclass
    class Row:
        id: int
        email: str
        display_name: str
        password: str

    built = UserResponse.model_validate(Row(1, "a@b.co", "Ana", "x"), from_attributes=True)

    assert built.model_dump() == {"id": 1, "email": "a@b.co", "display_name": "Ana"}

import os


def test_env():
    assert os.getenv("APP_DEBUG") is None

from myapp.settings import Settings


def test_default():
    assert Settings().database_url

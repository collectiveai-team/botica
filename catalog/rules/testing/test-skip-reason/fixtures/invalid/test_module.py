import pytest

pytestmark = pytest.mark.skipif("sys.platform == 'win32'")


@pytest.mark.parametrize(
    "value",
    [1, pytest.param(2, marks=pytest.mark.xfail)],
)
def test_a_value(value):
    assert value == 1

import sys

import pytest

FLAKY = "the server of the sandbox drops one request in fifty"
pytestmark = [pytest.mark.skipif(sys.platform == "win32", reason="no fork on Windows")]


@pytest.mark.skip("the export moves to the new API in #120")
def test_a_positional_reason(): ...


@pytest.mark.skip(reason="the export moves to the new API in #120")
def test_a_keyword_reason(): ...


@pytest.mark.skipif(sys.version_info < (3, 12), reason="needs PEP 695")
def test_a_skipif(): ...


@pytest.mark.xfail(reason=FLAKY, strict=False)
def test_an_xfail_whose_reason_is_a_name(): ...


@pytest.mark.parametrize(
    "value",
    [1, pytest.param(2, marks=pytest.mark.xfail(reason="#87: two rounds down"))],
)
def test_a_value(value):
    assert value == 1


@pytest.mark.timeout(5)
def test_another_mark(): ...

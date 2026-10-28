import pytest


def test_a_skip_with_no_reason():
    pytest.skip()


def test_an_xfail_with_no_reason():
    pytest.xfail()


def test_an_empty_skip():
    pytest.skip("")


def test_an_empty_reason():
    pytest.skip(reason="", allow_module_level=False)

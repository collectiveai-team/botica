import sys

import pytest


@pytest.mark.skip
def test_a_bare_skip(): ...


@pytest.mark.skip()
def test_a_skip_with_no_argument(): ...


@pytest.mark.skip(reason="")
def test_an_empty_reason(): ...


@pytest.mark.skipif(sys.platform == "win32")
def test_a_skipif_with_no_reason(): ...


@pytest.mark.xfail
def test_a_bare_xfail(): ...


@pytest.mark.xfail(raises=KeyError, strict=True)
def test_an_xfail_that_names_the_error(): ...


@pytest.mark.xfail("sys.version_info < (3, 12)")
def test_an_xfail_whose_argument_is_a_condition(): ...

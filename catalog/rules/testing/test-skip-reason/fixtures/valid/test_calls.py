import pytest

numpy = pytest.importorskip("numpy")


def test_a_skip(tmp_path):
    if not (tmp_path / "data").exists():
        pytest.skip("the data set is not downloaded")


def test_a_keyword_reason():
    pytest.skip(reason=f"needs numpy {numpy.__version__}")


def test_an_xfail():
    pytest.xfail("#87: two rounds down")


def test_a_fail():
    pytest.fail("not written")


def skip():
    return None


def test_a_function_of_its_own():
    skip()

from pytest import mark, skip, xfail


@mark.skip
def test_a_bare_skip(): ...


def test_a_skip_with_no_reason():
    skip()


def test_an_xfail_with_no_reason():
    xfail()

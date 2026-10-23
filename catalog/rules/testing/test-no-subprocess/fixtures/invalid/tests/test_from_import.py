from subprocess import PIPE, run


def test_the_version_is_printed():
    done = run(["mytool", "--version"], stdout=PIPE, check=True)
    assert done.stdout

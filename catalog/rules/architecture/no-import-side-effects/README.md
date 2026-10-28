## Directive

Importing a Python module reads and writes no file, opens no connection and starts no process. The
rule reports a call of this list that runs when the module is imported: at the top of the module,
in a `with`, `if`, `try` or `for` block or a comprehension there, in a class body, in a decorator
or in the default value of a parameter. A warning by default.

- Files: `open`, `io.open`, `os.open`, `listdir`, `scandir`, `walk`, `mkdir`, `makedirs`, `remove`,
  `unlink`, `rmdir`, `removedirs`, `rename`, `renames` and `replace` of `os`, `copy`, `copy2`,
  `copyfile`, `copytree`, `move`, `rmtree`, `make_archive` and `unpack_archive` of `shutil`,
  `glob.glob`, `glob.iglob`, `pandas.read_*` and `pd.read_*`, and the methods `read_text`,
  `read_bytes`, `write_text`, `write_bytes`, `iterdir`, `rglob`, `mkdir`, `touch`, `unlink` and
  `rmdir` of any object.
- Processes: `run`, `call`, `check_call`, `check_output`, `Popen`, `getoutput` and
  `getstatusoutput` of `subprocess`, `os.system` and `os.popen`.
- Network: `get`, `post`, `put`, `patch`, `delete`, `head`, `options`, `request` and `stream` of
  `requests` and `httpx`, `urllib.request.urlopen`, and `socket`, `create_connection`,
  `create_server`, `getaddrinfo` and `gethostbyname` of `socket`.
- Databases: `connect` of `sqlite3`, `psycopg`, `psycopg2`, `pymysql` and `duckdb`.

It also reports a function of these modules imported by name, as in `from subprocess import
check_output`.

## Why

Python runs the top of a module when a program imports it, so the I/O there runs in each program
that imports the module for any of its names: `--help` of a command-line tool, the collection of
the tests by pytest, the start of an API. When the file is missing or the network is down, the
import fails, and the test cannot replace the call, since it ran before the test started. ruff
0.17.1 has no rule for it: `RUF067` reports code in an `__init__.py`, not I/O in other modules.

## Exceptions

The rule does not report a call in the body of a function or in a lambda, which run when they are
called, or in the `if __name__ == "__main__":` block and the `if TYPE_CHECKING:` block. The `else`
of those blocks runs at import and is reported. It leaves out the test files (`test_*.py`,
`*_test.py`), stubs (`*.pyi`), `__main__.py`, `setup.py`, `noxfile.py` and the `conf.py` of Sphinx
under `doc/` or `docs/`, which a tool runs and nothing imports. `conftest.py` is checked.

It does not report the constructors of clients. `create_engine` of SQLAlchemy and `redis.Redis`
connect at the first command, and the AWS Lambda documentation says to "initialize SDK clients and
database connections outside of the function handler", so that the invocations share them, which
puts `boto3.client` at the top of the module. It does not report `load_dotenv`,
`logging.basicConfig`, reading `os.environ`, `os.path.exists` or `shutil.which`. It does not
report a module imported under another name (`import subprocess as sp`), a function imported under
another name (`from subprocess import run as sh`), or `Path.open` and `Path.glob`, whose names
other objects also use.

## Do

```python
@cache
def version() -> str:
    return Path(__file__).with_name("VERSION").read_text().strip()


engine = create_engine(settings.database_url)


def main() -> int:
    return subprocess.run(["git", "status"], check=False).returncode


if __name__ == "__main__":
    sys.exit(main())
```

## Don't

```python
__version__ = Path(__file__).with_name("VERSION").read_text().strip()

with open("schema.json") as handle:
    SCHEMA = json.load(handle)

COMMIT = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
```

## Suppression

One comment silences one finding, on the first line of the call. There are two forms:
`# botica: ignore[<slug>]` names the rule by its slug, and `# noqa: <code> -- <reason>` names it
by its code, with the reason after `--`.

```python
PLUGINS = os.listdir("plugins")  # botica: ignore[no-import-side-effects] -- a folder of the wheel
PLUGINS = os.listdir("plugins")  # noqa: BTARCH008 -- a folder of the wheel
```

A script that runs its work at the top takes `# botica: ignore-file[no-import-side-effects] --
<reason>` in a comment. A folder of scripts, examples or benchmarks goes in the `extend-ignores`
of `[rules.no-import-side-effects]`.

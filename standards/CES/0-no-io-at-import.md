---
id: CES-0
title: Importing a module does no I/O
---

## Directive

Importing a Python module reads and writes no file, opens no connection and starts no process. The
code at the top of a module, in a class body, in a decorator or in the default value of a parameter
runs when the module is imported, so it holds no such call. The call goes into a function that the
caller runs, or into the `if __name__ == "__main__":` block of a script.

- A file, a network request, a subprocess or a connection to a database opens inside a function.
  A value read once and kept, such as the version in a `VERSION` file, comes from a function with
  `functools.cache`.
- A client that connects at its first use, such as the `Engine` of `create_engine` in SQLAlchemy,
  may be built at the top of a module. So may a client of AWS in a Lambda function, which the AWS
  documentation tells to "initialize SDK clients and database connections outside of the function
  handler".
- Reading `os.environ`, configuring `logging` and building the settings object (CES-76) are not I/O
  under this standard.
- A script runs its work from `if __name__ == "__main__":` or a `main()` that the block calls. A
  file that only a tool runs and nothing imports, such as `__main__.py`, `setup.py`, `noxfile.py`
  or the `conf.py` of Sphinx, is out of scope.
- A test file (`test_*.py`, `*_test.py`) may read its data at the top. `conftest.py` follows the
  standard, since pytest imports it before each run of the tests it covers.

## Why

Python runs the top of a module each time a program imports it for the first time. A program that
imports the module for one function also does its I/O: `--help` of a command-line tool, the
collection of the tests by pytest, a worker that imports the routes of an API, and the autodoc of
Sphinx, which imports the package to read its docstrings. When the I/O fails, because the file is not
there, the network is down or the database refuses the connection, the import fails, and the error
names the import and not the function that needed the file.

I/O at import also makes the module depend on the directory it runs from and on the services it
reaches, so a test cannot replace them: the call has run before the test starts. Moved into a
function, the call runs when the caller asks for it, and a test replaces it through a port
(CES-64).

The SQLAlchemy documentation says of `create_engine`: "The Engine, when first returned by
create_engine(), has not actually tried to connect to the database yet; that happens only the
first time it is asked to perform a task against the database." So the engine at the top of a
module does no I/O.

ruff 0.17.1 has no rule for it. `RUF067` (preview) reports any code in an `__init__.py` other than
imports, assignments to dunder names and `if TYPE_CHECKING:` blocks, and not I/O in other modules.
The requests for such a rule, astral-sh/ruff#4757 and astral-sh/ruff#20113, are open.

## Examples

```python
# Do
@cache
def version() -> str:
    return Path(__file__).with_name("VERSION").read_text().strip()


engine = create_engine(settings.database_url)


# Don't: every importer reads the file, and fails without it
__version__ = Path(__file__).with_name("VERSION").read_text().strip()

with open("schema.json") as handle:
    SCHEMA = json.load(handle)
```

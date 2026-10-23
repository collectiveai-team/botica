## Directive

A test file does not start a process. In Python, the rule reports, in a file under `tests/`, a
`test_*.py`, a `*_test.py` or a `conftest.py`, a call to `subprocess.run`, `call`, `check_call`,
`check_output`, `Popen`, `getoutput` and `getstatusoutput`, to `os.system`, `os.popen`, `os.exec*`,
`os.spawn*` and `os.posix_spawn*`, and to `asyncio.create_subprocess_exec` and
`asyncio.create_subprocess_shell`, also when the name comes from `from subprocess import run`. In
TypeScript and JavaScript, it reports, in a `*.test.*`, a `*.spec.*` or a file under `__tests__/`,
`test/` or `tests/`, an import or a `require` of `child_process`, `node:child_process`, `execa`,
`zx` and `cross-spawn`, and a call to `Bun.spawn`, `Bun.spawnSync`, `` Bun.$`...` `` and
`new Deno.Command`. A warning by default. With `botica lint --diff`, only a finding on a line that the change adds is reported.

## Why

A process that a test starts runs outside the runtime of the test: pytest-socket, which blocks the
sockets of the interpreter of pytest, does not block it, and `curl https://example.com` in a child
process gets its answer from the network. The test also depends on the programs installed on the
host, so it passes on one machine and fails on another. ruff `S603` does not report a command
written as a literal list, since ruff 0.11.3, because a literal command is not an injection.

In TypeScript and JavaScript the rule reports the import and not each call: `promisify(exec)` and
`import * as cp` start processes through a name that the call does not show.

## Exceptions

The rule leaves out the code of the application, where a command is the work of an adapter, and the
files that are not tests. It does not report `multiprocessing`, whose workers run Python code of the
test, a Python module imported under another name (`import subprocess as sp`), a Python function
imported under another name (`from subprocess import run as sh`), or an `import type` of
`child_process`. A helper or a global setup under `test/` or `tests/` that starts a service, such as
`docker compose up` before the tests of Playwright, is reported: it is silenced with its reason.

## Do

```python
class InMemoryRemote:
    def __init__(self) -> None:
        self.tags: list[str] = []

    def push(self, tag: str) -> None:
        self.tags.append(tag)


def test_a_release_pushes_its_tag():
    remote = InMemoryRemote()
    Releases(remote).publish("v1.0.0")
    assert remote.tags == ["v1.0.0"]
```

```ts
class InMemoryRemote implements Remote {
  readonly tags: string[] = [];

  push(tag: string): void {
    this.tags.push(tag);
  }
}

test("a release pushes its tag", () => {
  const remote = new InMemoryRemote();
  new Releases(remote).publish("v1.0.0");
  expect(remote.tags).toEqual(["v1.0.0"]);
});
```

A test of the adapter that runs the command is a test of the command itself. In Python it carries
the marker `integration` and the comment that silences the line:

```python
@pytest.mark.integration
def test_the_remote_pushes_a_tag(tmp_path):
    subprocess.run(["git", "init", tmp_path], check=True)  # noqa: BTTEST009 -- the adapter runs git
```

## Don't

```python
def test_the_file_downloads():
    subprocess.run(["/usr/bin/curl", "-o", "out.json", "https://example.com"], check=True)
```

```ts
import { execSync } from "node:child_process";

test("the tag is read", () => {
  expect(execSync("git describe").toString()).toMatch(/^v/);
});
```

## Suppression

One comment silences one finding, on the first line of the call or of the import:

```python
subprocess.run(  # botica: ignore[test-no-subprocess] -- the script under test drives git
    ["git", "status"], check=True
)
```

```ts
import { execFileSync } from "node:child_process"; // botica: ignore[test-no-subprocess] -- the adapter runs git
```

`# noqa: BTTEST009 -- <reason>` works in Python too. A folder of tests that run commands on purpose,
such as the tests of a command line tool, goes in the `extend-ignores` of
`[rules.test-no-subprocess]`.

# References: a collection of the catalog that points at code and documentation

**Status:** proposed

The scaffolder copies five snippets of `scaffolding/templates/snippets/` to `.agents/snippets/` of a
repository: `core/logger.py` (CES-74), `tests/in_memory_repository.py` (CES-64), `settings.py`,
`api-schemas.py` and `no-dict-boundary.py`. Each needs adapting before it can be used: the logger
names the path of the package (`<your_package>/core/logger.py`), the in-memory repository implements
one example Protocol, and the settings carry their own fields. A copy drifts from the code it came
from and shows nothing of how that code is used. A topic such as structured logging has several
pieces of code worth reading (the setup, a call in an endpoint, a binding in a middleware), and it
concerns several standards and rules at once (CES-45, CES-46, CES-74, `log-get-logger`,
`log-no-print`).

## Decision

1. **References are a third collection of the catalog,** `references/<topic>/`, beside the rules
   and the standards. A topic is a `topic.toml` that lists its references and a `README.md` that says
   what to read in each one and what to adapt. A reference is evaluated by no tool and required by no
   standard.

   ```toml
   # lint/references/structured-logging/topic.toml
   title = "Structured logging with structlog"

   [[references]]
   repo = "github.com/<owner>/<repo>"
   rev = "3f2a9c1d5e7b4a6c8d0e1f2a3b4c5d6e7f8a9b0c"   # the full SHA of a commit: the lines never change
   path = "src/app/core/logger.py"
   lines = "1-48"
   anchor = "def get_logger"          # found again when the reference is updated
   license = "MIT"
   note = "the configuration: JSON in production, a console in development"

   [[references]]
   url = "https://www.structlog.org/en/stable/getting-started.html"
   note = "the getting-started page of the documentation of structlog"
   ```

2. **A standard or a rule cites topics by id,** `references = ["structured-logging"]`. A topic may be
   cited by several standards and rules, and it may hold several references.
3. **Any repository can be a reference,** of Collective or external. Each reference to a file of
   another repository records the license of the code it shows.
4. **A reference is a file or a page.** A page has a `url` where a file has a `path`: it is the
   documentation of a library, with its note.
5. **A reference is code to read and adapt.** Botica does not copy a reference into a repository,
   and the snippets of the scaffolder become references to the code they came from.
6. **`botica refs` reads the collection,** apart from `botica lint`:

   | Command | What it does |
   |---|---|
   | `botica refs list` | lists the topics, with their references and who cites them |
   | `botica refs show <topic, rule or standard>` | prints each reference with its context and its permalink |
   | `botica refs pin <permalink> --topic <topic>` | turns a permalink of GitHub (`…/blob/<sha>/<path>#L10-L40`) into a reference, with its anchor |
   | `botica refs check` | fails when a reference no longer resolves |
   | `botica refs update <topic>` | finds the anchor on the current branch and proposes the new commit and lines |

7. **The site publishes an `llms.txt`** in the format of https://llmstxt.org/, at the root of the
   site. `scripts/site_llms.py` writes it when `scripts/build_site.sh` builds the site, from the
   pages exported from the catalog and copied from `standards/`. It has the name and the description
   of the site from `website/zensical.toml`, a section Rules with each rule by code, the address of
   its page and its description, and a section Standards with each standard by number, the address
   of its page and its title.

8. **A reference has a `path` or a `url`, and a file is internal or external.** An internal file has
   no `repo`, `rev` or `license`, and its `path` goes from the root of the repository that holds
   `references/`. An external file has `repo`, `rev` and `license`. A page has `url` and `note`, and
   `lines`, `anchor`, `repo`, `rev` and `license` do not apply to it. The parser rejects `repo`
   without `rev`, `rev` without `repo`, an external file without `license` and a `license` without
   `repo`. `rev` is a full commit SHA (40 hexadecimal digits in lower case, 64 in a SHA-256
   repository): a branch or a tag moves, and a remote cannot be asked for an abbreviated SHA. `repo`
   is `<host>/<path>` with no scheme, no user and no port, and botica asks `https://<host>/<path>.git`.
   `note` is required. `lines` is `<first>-<last>` or one line, counted from 1, and a file reference
   without it is the whole file.
9. **The topics that the built-in rules cite are in the binary,** in `lint/references/`, compiled
   in like `lint/rules/`. They point at pages and at external files only. A project adds its own in
   `references/` at its root, read from disk like the standards (ADR 0004). The loader is a module
   of `botica-catalog`. A topic of the project with the id of a built-in topic is a load error,
   unless `replace` in the `[references]` table of the configuration lists the id; the topic of the
   project then takes the place of the built-in one, and the rules that cite the id resolve to it.
   A `replace` entry that names no built-in topic, or a built-in topic that `references/` does not
   define, is an error.
10. **A rule cites topics in `rule.toml`** with `references = ["<topic>"]`, as kebab-case ids that
    the catalog checks when it loads. A standard cites them in its front matter with
    `references: ["<topic>"]`, a JSON list (ADR 0004). The page of a rule has the field in its front
    matter, and `catalog.json` has it in every rule.
11. **`botica refs list`, `show` and `check` read the topics of the binary and of the project.**
    - `list` prints each topic with its references, the rules and standards that cite it and where it
      comes from (`built-in` or `project`).
    - `show` takes a topic, a rule (slug, code or alias, also of a catalog of the project) or a
      standard, and prints the README of each topic and, for each reference, its note and the lines
      of the file. A page is its address and its note. An external file also has its license and, on
      GitHub, the permalink of its lines.
    - `check` fails (exit 1) when a file cannot be read, `lines` is past the end, `anchor` is not in
      the lines, a path is absolute or leaves the project or the repository, a `topic.toml` does not
      load, or a rule or a standard cites a topic that does not exist. A page is not fetched.
    - `export` prints the topics as JSON, with the text of each file reference, for the pages of
      the site. It stops at the first reference that cannot be read.
    - A file is shown inside a fence of backticks one longer than the longest run of them in the
      lines, and of three at least. A Markdown file with blocks of code cannot close it.
12. **External files are read with `git`,** the one of the user, with its credentials, its helpers
    and its `url.<base>.insteadOf`. In a scratch directory of the cache botica runs
    `git init --bare`, `git fetch --depth 1 --filter=blob:none -- https://<repo>.git <rev>`,
    `git ls-tree` and `git show <rev>:<path>`, and removes the directory. `GIT_TERMINAL_PROMPT=0`
    makes a missing credential an error. Each command is killed after 60 seconds, or after the
    seconds in `BOTICA_REFS_TIMEOUT`. The errors name the repository and the commit and carry the
    first line that git printed.
13. **The cache has a file per repository, commit and path,** `<cache>/<repo>/<rev>/<path>`, written
    whole or not at all. A commit does not change, so an entry does not expire, and a hit reads the
    file with no git and no network. The cache is `--cache-dir` of `refs show` and `refs check`, else
    `botica/refs` under `XDG_CACHE_HOME`, `LOCALAPPDATA` (Windows) or `~/.cache`. With a cold cache
    there is one fetch per file, even when two files share a commit.
14. **`check` fetches the references of the topics of the project, and `check --all` also those of
    the built-in topics.** Every run checks that each rule and standard cites a topic that exists,
    built-in or of the project. The references of the built-in topics point at botica, and the job
    `test` of `ci.yml` runs `botica refs check --all`. Without the network `check` fails for the
    files that the cache does not hold.
15. **The site has a page per topic,** `references/<topic>/`, written by
    `scripts/site_references.py` when `scripts/build_site.sh` builds the site. The script reads the
    output of `botica refs export`, which prints every topic as JSON and gives each file reference
    the text it shows: the lines, read from the project or fetched at the commit. A page is the
    README, each reference and the rules and standards that cite the topic. A page reference is a
    link with its note. A file has its lines in a block of code, and an external file also its
    repository, its commit, its license and, on GitHub, its permalink: a reader does not leave the
    page to understand a reference. The build needs `git` and the network for external files, and
    stops when a reference cannot be read. The page of a rule or a standard lists its topics at the
    end. The navigation and `llms.txt` have a section References.
16. **The first topics are `structured-logging` and `in-memory-adapters`,** in `lint/references/`.
    They point at the snippets of `scaffolding/templates/snippets/` at the commit `9c225ad`.
    `log-get-logger` and `log-no-print` cite the first, and so does CES-74. CES-64 cites
    `in-memory-adapters`.

## Not built yet

- `botica refs pin` and `botica refs update`. The `rev` of a built-in topic is edited by hand.
- The removal of `.agents/snippets/` and `scaffolding/templates/snippets/`. The two topics point at
  the snippets at a commit, which stays readable after the removal.
- The instruction of the skill to run `botica refs show <rule>` before writing code.

## Not decided

- Whether the site publishes a Markdown version of each page (`<page>/index.md`), which the format
  of `llms.txt` recommends as the target of its links. The links of `llms.txt` point at the HTML
  pages.
- Whether the site also shows the content of a page reference. It shows its address and its note.

## Considered options

- **References inside the folder of a rule.** Rejected: a topic concerns several rules and
  standards, and its references would be repeated.
- **Snippets copied into each repository.** Rejected: a copy drifts and needs adapting anyway.
- **Context7 for the documentation of libraries.** It indexes the documentation of public libraries
  and serves it through MCP; private sources need a paid plan. Left out: a direct link to the page of
  the documentation needs no service.
- **A `docs` list in `topic.toml`.** Replaced by page references: the list had no note.
- **A field `kind = "doc"` or `"code"`.** Rejected: the field that is present, `url` or `path`,
  says it.
- **`check` fetches each page.** Rejected: it adds the network to every run, and a page that moves
  breaks the build for a reason that has nothing to do with the change.
- **A branch, a tag or an abbreviated SHA as `rev`.** Rejected: a branch or a tag moves, a remote
  cannot be asked for an abbreviated SHA, and the cache of a moving name would be stale.
- **A full URL, an SSH address or a `file://` path as `repo`.** Rejected: several syntaxes to
  validate, and `file://` reads the disk under the name of an external reference. A user of SSH sets
  `url.<base>.insteadOf` in the configuration of git.
- **An HTTP client against `raw.githubusercontent.com`, `gh api` or the crate `git2`.** Rejected:
  GitHub only, a token written by hand for private repositories, and a dependency (`git2` needs
  libgit2 and a copy of the credential helpers).
- **A cache in `.botica/` of the project.** Rejected: CI discards it with each checkout, and each
  project keeps its own copy.
- **A bare repository per repository as the cache.** Rejected: it grows with each commit, and with
  `--filter=blob:none` a file that was never asked for needs the network.
- **`--offline`, or skipping the references that cannot be fetched.** Rejected: a deleted commit or a
  moved repository would pass.
- **The replacement mechanism of the rules for topics** (a topic with its own id that declares
  `replaces`, and a `replace` table). Rejected: the built-in rules cite the built-in id, and with
  the same id their citations stay as they are.
- **A topic of the project with the id of a built-in one wins without a word.** Rejected: the same
  collision is an error between rules, and the configuration says when it is intended.
- **`botica refs export` writes the pages of the topics.** Rejected: `scripts/build_site.sh` already
  decorates the standards with a script, and the pages of the topics are written the same way.
  `export` prints JSON, and the script writes the pages.
- **The site does not fetch the external files and links them.** Rejected: a reader would have to
  open another site to understand a reference.
- **The script of the site runs `botica refs show` or `git` by itself.** Rejected: the output of
  `show` is text for a terminal, and a second implementation of the fetch and the cache would drift
  from the first.
- **A search of code of botica's own.** Rejected: grep.app, the MCP of Sourcegraph and the Librarian
  of Amp search code on GitHub. Botica keeps the selection: `refs pin` records what a search found.

## Consequences

- The catalog carries `lint/references/`, loads `references/` of the project, and `botica refs
  check --all` runs in CI.
- `botica refs show <rule>` works in any project with `git`, and reads the network once per file.
- `.agents/snippets/` and `scaffolding/templates/snippets/` are removed once each snippet is a
  reference.
- The skill tells an agent to run `botica refs show <rule>` before it writes code that the rule
  covers.
- The page of a standard or a rule on the site lists its topics and links them.

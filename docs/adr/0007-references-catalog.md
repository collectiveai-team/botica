# References: a collection of the catalog that points at code in real repositories

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
   # references/structured-logging/topic.toml
   title = "Structured logging with structlog"
   docs = ["https://www.structlog.org/en/stable/getting-started.html"]

   [[references]]
   repo = "github.com/<owner>/<repo>"
   rev = "3f2a9c1"                    # a commit: the lines never change
   path = "src/app/core/logger.py"
   lines = "1-48"
   anchor = "def get_logger"          # found again when the reference is updated
   note = "the configuration: JSON in production, a console in development"
   license = "MIT"
   ```

2. **A standard or a rule cites topics by id,** `references = ["structured-logging"]`. A topic may be
   cited by several standards and rules, and it may hold several references.
3. **Any repository can be a reference,** of Collective or external. Each reference records the
   license of the code it shows.
4. **`docs` links the documentation of the libraries a topic uses,** by the URL of the page.
5. **A reference is code to read and adapt.** Botica does not copy a reference into a repository,
   and the snippets of the scaffolder become references to the code they came from.
6. **`botica refs` reads the collection,** with the network and apart from `botica lint`:

   | Command | What it does |
   |---|---|
   | `botica refs list` | lists the topics, with their references and who cites them |
   | `botica refs show <topic, rule or standard>` | prints each reference with its context and its permalink |
   | `botica refs pin <permalink> --topic <topic>` | turns a permalink of GitHub (`…/blob/<sha>/<path>#L10-L40`) into a reference, with its anchor |
   | `botica refs check` | fails when a reference no longer resolves |
   | `botica refs update <topic>` | finds the anchor on the current branch and proposes the new commit and lines |

   `show` uses the credentials of git of the user, which reach private repositories, and caches each
   reference by its commit.
7. **The site publishes an `llms.txt`** in the format of https://llmstxt.org/, at the root of the
   site. `scripts/site_llms.py` writes it when `scripts/build_site.sh` builds the site, from the
   pages exported from the catalog and copied from `standards/`. It has the name and the description
   of the site from `website/zensical.toml`, a section Rules with each rule by code, the address of
   its page and its description, and a section Standards with each standard by number, the address
   of its page and its title.

8. **A reference is internal or external.** An internal reference has no `repo`, `rev` or
   `license`, and its `path` goes from the root of the repository that holds `references/`. An
   external reference has `repo`, `rev` and `license`. The parser accepts both forms and rejects
   `repo` without `rev`, `rev` without `repo`, an external reference without `license` and a
   `license` without `repo`. `note` and `path` are required. `lines` is `<first>-<last>` or one line,
   counted from 1, and a reference without it is the whole file.
9. **`references/` is read from disk at the root of the project,** beside `standards/`, like the
   standards (ADR 0004); the binary has none of it. The loader is a module of `botica-catalog`.
10. **A rule cites topics in `rule.toml`** with `references = ["<topic>"]`, as kebab-case ids that
    the catalog checks when it loads. A standard cites them in its front matter with
    `references: ["<topic>"]`, a JSON list (ADR 0004). The page of a rule has the field in its front
    matter, and `catalog.json` has it in every rule.
11. **`botica refs list`, `show` and `check` are built for internal references.**
    - `list` prints each topic with its references and the rules and standards that cite it.
    - `show` takes a topic, a rule (slug, code or alias, also of a catalog of the project) or a
      standard, and prints the README of each topic, its `docs` and, for each reference, its note and
      the lines of the file.
    - `check` fails (exit 1) when a file is gone, `lines` is past the end, `anchor` is not in the
      lines, a path is absolute or leaves the project, a `topic.toml` does not load, or a standard or
      a rule of a catalog of the project cites a topic that does not exist.
    - `show` and `check` report an external reference as not supported yet.
12. **`check` leaves the citations of the built-in rules to a test of this checkout.** Those rules
    cite the topics of this repository, and a project that has none of them would fail on every run.
    `lint/crates/botica-cli/tests/repo_references.rs` checks them, and `botica refs check` runs in
    the job `test` of `ci.yml`.
13. **The site has a page per topic,** `references/<topic>/`, written by
    `scripts/site_references.py` when `scripts/build_site.sh` builds the site: the README, the
    `docs`, each reference with the lines of its file, and the rules and standards that cite the
    topic. The page of a rule or a standard lists its topics at the end. The navigation and
    `llms.txt` have a section References.
14. **The first topics are `structured-logging` and `in-memory-adapters`,** and they point at the
    snippets of `scaffolding/templates/snippets/`. `log-get-logger` and `log-no-print` cite the first,
    and so does CES-74.

## Not built yet

- `botica refs pin` and `botica refs update`.
- External references: `show` and `check` with the network and a cache by commit.
- The removal of `.agents/snippets/` and `scaffolding/templates/snippets/`. The two topics point at
  the snippets of the scaffolder, so removing them means moving each reference to the code it came
  from.
- The instruction of the skill to run `botica refs show <rule>` before writing code.

## Not decided

- Whether CES-64 cites `in-memory-adapters`. A pull request touches one standard, so the citation
  goes in its own.
- Whether `check` also verifies the citations of the built-in rules in a project that has the
  topics they name.
- Whether the site publishes a Markdown version of each page (`<page>/index.md`), which the format
  of `llms.txt` recommends as the target of its links. The links of `llms.txt` point at the HTML
  pages.

## Considered options

- **References inside the folder of a rule.** Rejected: a topic concerns several rules and
  standards, and its references would be repeated.
- **Snippets copied into each repository.** Rejected: a copy drifts and needs adapting anyway.
- **Context7 for the documentation of libraries.** It indexes the documentation of public libraries
  and serves it through MCP; private sources need a paid plan. Left out: a direct link to the page of
  the documentation needs no service.
- **`lint/references/` embedded in the binary, like `lint/rules/`.** Rejected: the standards are not
  embedded (ADR 0004), and a topic that points at files of the repository needs those files.
- **`botica refs check` verifies the citations of the built-in rules everywhere.** Rejected: it
  would fail in every project that does not have the topics of this repository. An option to ask for
  it was left out as one more flag.
- **`botica refs export` writes the pages of the topics.** Rejected: `scripts/build_site.sh` already
  decorates the standards with a script, and the pages of the topics are written the same way.
- **A search of code of botica's own.** Rejected: grep.app, the MCP of Sourcegraph and the Librarian
  of Amp search code on GitHub. Botica keeps the selection: `refs pin` records what a search found.

## Consequences

- The catalog loads `references/`, and `botica refs check` runs in CI.
- `.agents/snippets/` and `scaffolding/templates/snippets/` are removed once each snippet is a
  reference.
- The skill tells an agent to run `botica refs show <rule>` before it writes code that the rule
  covers.
- The page of a standard or a rule on the site lists its topics and links them.

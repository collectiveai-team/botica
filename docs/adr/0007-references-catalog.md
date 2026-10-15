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

## Not decided

- Whether the site publishes an `llms.txt` generated from the catalog.

## Considered options

- **References inside the folder of a rule.** Rejected: a topic concerns several rules and
  standards, and its references would be repeated.
- **Snippets copied into each repository.** Rejected: a copy drifts and needs adapting anyway.
- **Context7 for the documentation of libraries.** It indexes the documentation of public libraries
  and serves it through MCP; private sources need a paid plan. Left out: a direct link to the page of
  the documentation needs no service.
- **A search of code of botica's own.** Rejected: grep.app, the MCP of Sourcegraph and the Librarian
  of Amp search code on GitHub. Botica keeps the selection: `refs pin` records what a search found.

## Consequences

- The catalog loads `references/`, and `botica refs check` runs in CI.
- `.agents/snippets/` and `scaffolding/templates/snippets/` are removed once each snippet is a
  reference.
- The skill tells an agent to run `botica refs show <rule>` before it writes code that the rule
  covers.
- The page of a standard or a rule on the site lists its topics and links them.

---
title: Write a rule
---

# Write a rule

A rule is a folder `<family>/<slug>/`: a `rule.toml`, a `README.md`, what runs, and `fixtures/`.
`botica rules new` writes the folder, with `TODO` where the rule needs its text, and
`botica rules conform` checks it against its fixtures.

```bash
botica rules new structure/no-foo --kind ast-grep --language python --standard CES-63
                               # the folder of a new rule (ast-grep, judgment or external) in catalog/rules
botica rules new structure/no-blobs --kind extension --runtime python --language python \
  --catalog .botica/rules/acme   # a rule of a catalog of a project: id acme/no-blobs, code ACMESTRUC001
botica rules new structure/dep-hygiene --kind external --language python --tool deptry \
  --docs https://deptry.com/ --config prek.toml   # an external rule and its fragment config/prek.toml
botica rules new structure/no-exec --kind ast-grep --language python --from judge-it
                               # a rule with the text of the judgment rule judge-it, added to its covered-by
botica rules conform catalog/rules   # each rule over its fixtures, with and without each optimisation;
                               # .botica/rules/<name> for a catalog of a project (it has a catalog.toml);
                               # the projects of the fixtures go to .botica/conformance, which ignores itself
botica rules test --judge abide [test-through-interface]
                               # abide judges the fixtures of each judgment rule; compared with expected.json
```

## The kind of a rule

| Kind | What it is | Where |
|---|---|---|
| `ast-grep` | a pattern over the syntax tree of one file, in YAML | any catalog |
| `extension` | a program that answers the protocol of botica ([Extensions](extensions.md)) | a catalog of a project |
| `external` | another tool enforces the rule through its configuration | any catalog |
| `judgment` | a question that a judge answers on the code | any catalog |
| `native` | Rust inside botica | the built-in catalog |

A pattern rule is the first choice. A judgment rule is for what no deterministic check can
express, even with configuration from the repository, and says why in `why-not-deterministic`
([ADR 0005](https://github.com/collectiveai-team/botica/blob/main/docs/adr/0005-deterministic-first-judgment-rules.md)).

## A pattern rule

`botica rules new` writes this `rule.toml`, a preview rule that is off by default with the next
code of its family:

```toml
id = "acme/no-eval"
code = "ACMESTRUC002"
summary = "TODO: what the rule asks, in one line"
status = "preview"
scope = "file"
needs = "syntax"
run-on = ["cli", "save", "type"]
default-enabled = false
default-severity = "warning"

[[implementations]]
kind = "ast-grep"
languages = ["python"]
file = "python.yml"
```

`run-on` says where the rule runs: `cli` in `botica lint`, `save` and `type` in the editor. The
pattern is in `python.yml`; in a catalog of a project its `id` is the full id:

```yaml
id: acme/no-eval
language: python
severity: warning
message: "eval() runs text as code: parse the text with ast.literal_eval or json.loads instead"
rule:
  pattern: eval($$$ARGS)
```

The message says the rule and its fix, and names no other rule. The README has the sections of
its template in their order (`Directive`, `Why`, `Do`, `Don't`, `Suppression` for this kind), and
the catalog does not load a README whose sections do not match its kind.

## Fixtures

`fixtures/valid/` holds files on which the rule finds nothing, and `fixtures/invalid/` files on
which it finds something. `fixtures/expected.json` lists what the invalid files raise: the path
from `invalid/`, the severity, the range and the message of each finding. The output of
`botica lint` in that folder has that form; write it, then read it:

```bash
cd .botica/rules/acme/structure/no-eval/fixtures/invalid
botica lint --preview --select acme/no-eval --output-format json . > ../expected.json
```

`botica rules conform` copies each side of the fixtures into a project of its own under
`.botica/conformance`, runs the rule with and without each optimisation of the engine, and compares
the findings with `expected.json`. It exits with 1 when a rule finds something on a valid fixture,
raises something else on the invalid ones, or has no fixture on a side. A rule that fails its
fixtures is changed, or its fixtures are, with a reason; the expectation is not rewritten to match
the output.

## Promote a rule

A new rule is `status = "preview"` and runs only with `--preview` and when it is selected.
`botica rules eval <id> <path>` runs it on real code to review its false positives. `stable` and
`default-enabled = true` are a separate change.

## External rules

Another tool enforces an external rule (`kind = "external"`) through its configuration. The rule
names the tool, the address of its documentation and, in `config/<path>` of its folder, the
fragment of configuration that goes into `<path>` at the root of the project, in the format of that
file: TOML, YAML or INI (`config/pnpm-workspace.yaml`, `config/.npmrc`,
`config/.cargo/config.toml`). [`external-config`](../rules/botica/bt004-external-config.md) reports
a key of the fragment that the settings do not set; a key set to a weaker value is the decision of
the repository. For ruff, pyrefly, pytest, uv, pnpm, npm, Yarn, Cargo and Bun it reads the settings
where the tool reads them: a fragment written for `pyproject.toml` also holds in `ruff.toml`, under
`extend-select`, or when the repository switches the family off with `ignore`.

An external rule asks nothing of a project that does not use its tool. `applies-when` lists globs,
from the root of the project, of the files that show the tool; without it, a file of one of
`languages` shows it:

```toml
[[implementations]]
kind = "external"
languages = ["javascript", "typescript"]
tool = "pnpm"
docs = "https://pnpm.io/settings"
config = "config/pnpm-workspace.yaml"
applies-when = ["pnpm-lock.yaml"]
```

`*` stays inside a folder and `**/` matches any folders, the root too: `**/Dockerfile` matches
`Dockerfile` and `deploy/Dockerfile`. A language whose files botica does not know, such as
`dockerfile`, needs `applies-when`, or the rule counts as used in every project.

The built-in catalog has no external rule; a catalog of a project declares them.

## Judgment rules

A judgment rule has one implementation of kind `judgment`, with a question, its criteria and
`why-not-deterministic`:

```toml
[[implementations]]
kind = "judgment"
languages = ["python"]
when = "edit"
why-not-deterministic = "Whether a test reaches into the internals of the unit under test needs the code of both."

[implementations.question]
type = "boolean"
instructions = "Does a test read a private attribute of the unit under test?"
criteria = { true = "a test reaches into the internals", false = "the tests call the public interface" }
```

`botica lint` never runs it. `botica rules export --format agents-md` writes the active judgment
rules into `AGENTS.md`, and `--format abide` into `.abide/rubric.json`, the rubric of
[abide](https://www.npmjs.com/package/@coldtea/abide). abide reads `true` as the violation: the
question asks whether the file breaks the rule. `botica lint --judged <file>` reads what abide
reports and applies the configuration and the comments to it.

`botica rules test --judge abide <slug>` calibrates a rule. It lays out the fixtures in
`.botica/judge/<slug>`, a git repository with a rubric that holds the rule alone, runs
`abide audit --json`, and compares each verdict with the band and the range of probability that
`fixtures/expected.json` gives the fixture. abide judges through the API of TypeSafe. With
`TYPESAFE_AI_BASE_URL` it sends its calls to another server with the same API instead, such as the
server of llama.cpp or a model run on its own. `botica rules test --judge abide` runs abide with
the environment as it is.

## A rule of the built-in catalog

A rule of `catalog/rules/` names the standard it implements in `standard`, and a pull request that
adds one touches that rule and one standard: `scripts/check_rule_change.py` fails one that touches
more. The rule `botica-rule-author` of the skills of this repository describes the same steps for
an agent.

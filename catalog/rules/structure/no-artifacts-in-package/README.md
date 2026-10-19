## Directive

Data, trained models and reports live **beside** the code of a project, not in it. Inside the code,
a file of data or of a model (`csv`, `parquet`, `pkl`, `pt`, `h5`, `npy`, `onnx`, `db`, `gz`…) is
an error. In a **Python** package, so is any file that is not code under a `data/`, `resources/`
or `reports/` directory.

The code of a project is its `code-dirs` (see `no-notebooks`).

## Why

Datasets, generated reports and static assets are not importable code. Bundling them into the
package bloats the wheel, leaks large or binary files into installs, and blurs the line between
"the library" and "the workspace around it". It is the layout of `cookiecutter-data-science` and
Kedro, which Collective's repositories follow.

The name of a directory is not enough: `data/` and `reports/` are often code (a `data/` package
with an `__init__.py`). That is why the rule looks at the **kind of file**, and for the three
names only at files that are not code.

## Exceptions

Java and Kotlin resources live in `src/main/resources`, outside `src/main/java`, so they are never
flagged. Files under `tests`, `test`, `__tests__`, `fixtures`, `testdata`, `examples`, `docs`,
`benches`, `snapshots` or a `*_tests` / `*-tests` directory are left alone.

## Do

```text
myproject/
  data/raw/sales.csv
  models/churn.pkl
  reports/q3.html
  src/myproject/
    __init__.py
    features.py
```

## Don't

```text
myproject/
  src/myproject/
    __init__.py
    features.py
    sales.csv
    churn.pkl
```

## Suppression

The finding belongs to a whole file. `extend-ignores` in the configuration of the rule leaves out
the files that match a glob; `**/resources/**` allows package data.

```toml
# botica.toml
[rules.no-artifacts-in-package]
extend-ignores = ["**/resources/**"]
```

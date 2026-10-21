## Directive

The option `layers` lists the layers of the project from the top one down, each a Python module
or several modules separated by `|`. A module of a layer imports nothing of a layer above it, and
the modules separated by `|` in one layer import nothing of each other. `botica lint` reports each
import statement that breaks this, at the statement, and also an import of a module in no layer
that leads, through other modules in no layer, to a module that the importer may not import; the
message shows that chain. The rule runs once for the whole project, on every `botica lint`. An
error by default; without `layers` it reports nothing.

A module is in a layer when it is the module of the layer or a module inside it. The first part of
the name of each module of the layers is a top package. The module of a file is its path from the
folder that holds its top package: the root of the project, else `src/`, else the shallowest folder
that holds it; the files of the package in other folders (`tests/app/…`) are left out. An import
names the longest module of the project its name starts with (`from app.api import router` names
`app.api` when `router` is no module), and a relative import starts from the package of its file.
Every import counts, inside a function or an `if TYPE_CHECKING:` block too.

## Why

When `core` cannot import `api`, the lower layers stay usable and testable without the upper ones,
and the dependency graph stays acyclic. A rule makes the direction a failure of the run instead of
something that a review has to notice.

## Do

```toml
[rules.import-layers]
layers = ["app.entrypoints", "app.api", "app.database | app.impl", "app.core"]
```

```python
# app/api/routes.py
from app.core.models import Order
```

## Don't

```python
# app/core/models.py
from app.api.routes import router
```

```python
# app/database/tables.py
from app.impl.service import price
```

## Options

- `layers` (strings, empty by default): the layers from the top one down, e.g.
  `["app.entrypoints", "app.api", "app.database | app.impl", "app.core"]`.

## Suppression

The rule is of project scope, and only the configuration of botica silences it: the path in
`extend-ignores` of `[rules.import-layers]`, or `ignore = ["import-layers"]`. A comment that names it is `invalid-suppression`.

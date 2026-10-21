## Directive

Layers and packages are named `entrypoints`, `api`, `database`, `impl` and `core`, an interface is a port and its implementation an adapter, and a concept that already has a name gets no synonym.

## Why

With one word for each concept, a reader or an agent finds a unit by its name, and `core` means the same thing in every repository. Two names for one concept are how two modules end up doing the same job.

## Flag

- A package or module named `web`, `rest` or `endpoints_v2` for the inbound HTTP layer, where `api` is the term.
- `persistence`, `dal` or `meta` for the unit that holds the tables, where `database` is the term.
- `common`, `shared` or `utils` for the domain core, where `core` is the term.
- A class called `…Manager`, `…Handler` or `…Processor` that implements a port, where adapter is the term.

## Do not flag

- A name that a framework or a library imposes, such as an exception handler of FastAPI.
- A concept that is new to the vocabulary and is added to it.
- The tests and their fixtures.
- A word inside a string or in data, and a local variable.

## Confirm

Before reporting, name the term of the vocabulary that the unit should take.

## Do

```python
from app.database.orders import OrderTable
from app.core.ports import OrderRepository


class SqlOrderRepository(OrderRepository): ...
```

## Don't

```python
from app.persistence.orders import OrderTable
from app.shared.ports import OrderRepository


class OrderManager(OrderRepository): ...
```

## Suppression

A file that keeps a name on purpose takes `botica: ignore-file[architecture-vocabulary] -- <reason>` in a comment. A folder goes in the `extend-ignores` of the rule.

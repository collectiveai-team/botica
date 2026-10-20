## Directive

Each concept of the domain has one term, the one that `GLOSSARY.md` defines, in the code and in the documentation. `GLOSSARY.md` lists each term with its definition and the words to avoid for it, and the code and the documentation use the term.

## Why

When two words name one concept, a reader searches for one and misses the other, and two modules end up doing the same work under different names. One term per concept lets the code, the documentation and a conversation about the domain use the same words.

## Flag

- Two names for one concept in the same file or change: `customer`, `client` and `account` for the one who places orders.
- A name that the glossary lists under the words to avoid for a term.
- A new concept of the domain with no entry in the glossary.

## Do not flag

- Terms of programming that are not of the domain: array, function, endpoint, request.
- The names of a library or of an external API, which the project does not choose.

## Confirm

Before reporting a synonym, read the glossary entry and check that both words name the same concept.

## Do

```python
def invoice_for(customer: Customer) -> Invoice: ...
```

with `GLOSSARY.md` listing **Customer** and `client, buyer` as words to avoid.

## Don't

```python
def invoice_for(client: Customer) -> Invoice: ...


def bill_buyer(account: Customer) -> Invoice: ...
```

## Suppression

A file takes `botica: ignore-file[ubiquitous-language] -- <reason>` in a comment, such as an adapter that maps the names of an external API. A folder goes in the `extend-ignores` of the rule.

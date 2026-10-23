## Directive

The recipes that a project uses ask for each package of a `[[tools]]` step with one requirement.
The recipes a project uses are the ones that `installed` in `[recipes]` of the configuration names,
the ones of `recipes/` at the root of the project, and the recipes that these extend. botica
resolves them together, as `botica recipe install` resolves the names it gets, and reports each
package, of the same manager and group, that a recipe asks for with another requirement than the
first recipe of the chain that asks for it. `ruff` and `ruff>=0.15` are different requirements;
`Ruff` and `ruff`, or `blockbuster>=1.5,<1.6` and `Blockbuster >= 1.5, < 1.6`, are the same.

```toml
# recipes/team.toml
extends = ["base-python"]  # base-python asks for "ruff"

[[tools]]
manager = "uv"
packages = ["ruff>=0.15"]  # reported here
```

The finding sits on the requirement in the file of the later recipe of the chain, or of the earlier
one when the later one is built in. When both are built in, it is about the file of the
configuration. When `recipes/` does not load, or `installed` names a recipe that does not exist,
botica cannot look for conflicts, and that is a finding about the file at fault. `botica recipe
check` prints a `conflict` line for each conflict between the recipes it checks and the ones the
project uses.

`botica recipe install` writes `installed` when it writes a new `botica.toml`. In a project that
already has a configuration, list the recipes it installed by hand:

```toml
[recipes]
installed = ["base-python", "team"]
```

## Why

`botica recipe install` stops with exit code 2, before it writes anything, when the recipes it gets
ask for one package with two requirements: which one won would depend on the order of `extends`.
Without the rule, a project learns it when it next runs `install`, maybe long after the commit
that changed one of the recipes. The rule reports it in that commit.

## Suppression

The finding is about the recipes of the project, so only the configuration silences it:
`ignore = ["recipe-conflict"]`, or the path in `extend-ignores` of `[rules.recipe-conflict]`. To
fix it, write the same requirement in both recipes, or leave the package out of the recipe that
extends the other.

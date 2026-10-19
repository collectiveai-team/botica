## Directive

The varlock schema of the project (`.env.schema` at its root) declares each environment variable
that a settings class reads, and a settings class reads each variable that the schema declares.
`botica lint` reports a field whose variable the schema lacks, at the name of the field, and a
variable of the schema that no settings class reads, at its key. A variable whose decorators
include `@auditIgnore` is read by another program and is left out. The rule runs once for the
whole project, on every `botica lint`, when the project has the schema; the second kind of finding
also needs a settings class in the project. A warning by default.

The reader of Python finds the `pydantic_settings.BaseSettings` classes and the classes that extend
them in the project. The variable of a field is `env_prefix` followed by the name of the field, or
the plain-string `alias` or `validation_alias` of its `Field(...)`, compared without case unless
the class sets `case_sensitive=True`. A subclass reads the fields of its parents with its own
`env_prefix`.

## Why

The schema is the list of the variables that a deployment sets and validates, and the settings
class is the list of the variables that the code reads. A variable added to one list and not to
the other is a setting that the deployment never validates, or one that it sets for nothing.

## Exceptions

A field holding the settings class of another package, a field whose `validation_alias` is an
`AliasChoices` or an `AliasPath`, and every field of a class that sets `env_nested_delimiter`, an
`alias_generator` or an inner `class Config` have names botica cannot list: they are not reported,
and the variables they may read count as read.

## Do

```python
class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_prefix="APP_")

    database_url: str
```

```env
# @type=url
APP_DATABASE_URL=

# @auditIgnore
CI=false
```

## Don't

```python
class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_prefix="APP_")

    database_url: str
    debug: bool = False
```

```env
APP_DATABASE_URL=
STRIPE_KEY=
```

## Suppression

Mark a variable that another program reads with `# @auditIgnore` above it in the schema. A field is
silenced through the configuration of botica, with its file in `extend-ignores` of
`[rules.settings-env-schema]` or `ignore = ["settings-env-schema"]`. A comment that names the rule is `invalid-suppression`.

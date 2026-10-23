## Directive

Each entry of `updates` in `.github/dependabot.yml` has `cooldown` with `default-days` of 7 or more.
The rule reports an entry with `package-ecosystem` that has no `cooldown`, whose `cooldown` has no
`default-days`, or whose `default-days` is less than 7. `cooldown` may be a block or a flow mapping.
A warning by default.

## Why

Without `cooldown`, Dependabot waits 3 days after a release before it opens a pull request for it
([GitHub changelog](https://github.blog/changelog/2026-07-14-dependabot-version-updates-introduce-default-package-cooldown/)).
The rule asks for 7, the default of the parameter `cooldown-days` of the recipes. The cooldown does
not hold back a security update.

## Do

```yaml
updates:
  - package-ecosystem: uv
    directory: "/"
    schedule:
      interval: weekly
    cooldown:
      default-days: 7
```

## Don't

```yaml
updates:
  - package-ecosystem: uv
    directory: "/"
    schedule:
      interval: weekly
```

## Suppression

One comment silences the entry. It names the rule by its slug and goes on the line where the entry
starts.

```yaml
  - package-ecosystem: docker # botica: ignore[dependabot-cooldown] -- base images are pinned by digest
```

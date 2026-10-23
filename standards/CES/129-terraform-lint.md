---
id: CES-129
title: Terraform is formatted, valid, linted and scanned before it is committed
---

## Directive

Each folder of Terraform or OpenTofu code of a repository passes four checks, as hooks of prek:

- `terraform fmt` leaves it as it is;
- `terraform validate` passes, after `terraform init -backend=false`;
- [`tflint`](https://github.com/terraform-linters/tflint) passes with the `.tflint.hcl` of the root
  of the repository, which turns on the `recommended` preset of the Terraform ruleset;
- [`trivy config`](https://trivy.dev/latest/docs/scanner/misconfiguration/) finds no
  misconfiguration of severity `HIGH` or `CRITICAL`, except the checks that `.trivyignore` lists
  with a reason.

In CI, [`checkov`](https://www.checkov.io/) finds no failed check that `.checkov.baseline` does
not list.

The recipe `iac-terraform` adds the hooks, `.tflint.hcl`, `.trivyignore`, `.checkov.yaml`, an
empty `.checkov.baseline`, the lines of `.gitignore` for the state and the folder `.terraform`, and
a workflow that runs the hooks and checkov in CI.

## Why

`terraform plan` is the first command that reads the code, and it needs credentials, the state and
a network. A reference to a value that does not exist, a variable that nothing reads or an S3
bucket open to the public is found there, or after `apply`. The four checks read the code alone and
take about a second for a folder whose providers are installed.

checkov has more checks than trivy, among them IAM policies with `*` as their resource, and takes
several seconds and a Python environment, so it runs in CI and not on each commit.

## Scope

The hooks of [pre-commit-terraform](https://github.com/antonbabenko/pre-commit-terraform) run each
tool once for each folder with a changed `.tf`, `.tofu` or `.tfvars` file. With
`--hook-config=--tool-version=<version>`, each hook downloads that release of Terraform, tflint or
trivy the first time and keeps it in `~/.cache/pre-commit-terraform`, so the same versions run on
every machine and in CI, and none of the tools has to be installed. The versions are the
parameters `terraform-version`, `tflint-version` and `trivy-version` of the recipe, and
`checkov-version` is the version of checkov. A repository of OpenTofu adds
`--hook-config=--tf-path=opentofu` to the hooks of `fmt` and `validate`.

`terraform validate` needs the providers. The first run in a folder without `.terraform`
downloads them: about 250 MB and 50 s for the AWS provider. `TF_PLUGIN_CACHE_DIR` keeps one copy
for every folder.

trivy uses the checks built into its release (`--skip-check-update`), so the result changes only
when the version of trivy changes, and reads only Terraform: hadolint lints the Dockerfiles
(CES-114).

checkov reports a severity only with an API key of Prisma Cloud, so its threshold is a baseline and
not a severity: the checks that fail when a repository adopts checkov go in `.checkov.baseline`,
and a check that fails on new code fails the workflow.

## Migration (existing repos)

Run `botica recipe install iac-terraform`, then `prek run --all-files`. `terraform_fmt` rewrites
the files it reports. Fix the findings of `terraform_validate` and `tflint` first: they are errors
of the code. For each finding of trivy, change the resource, or ignore it on the resource with
`#trivy:ignore:<id>` and the reason.

Then run checkov once with `--create-baseline` at the root, with the version of the workflow:
`uvx --python 3.12 checkov@<version> --directory . --create-baseline`. It writes the failed checks
of today into `.checkov.baseline`; commit it, and remove an entry when its resource is fixed.

## Suppression

A finding of tflint: `# tflint-ignore: <rule>` on the line above. A finding of trivy:
`#trivy:ignore:<id>` on the line above the resource, with the reason. A finding of checkov:
`#checkov:skip=<check>:<reason>` inside the resource. A check that does not apply to the whole
repository goes in `.tflint.hcl` (a `rule` block with `enabled = false`), in `.trivyignore` or in
the `skip-check` list of `.checkov.yaml`, with a comment that gives the reason. The hooks are not
removed to silence one finding.

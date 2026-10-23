## Directive

`prek.toml` has the hooks `terraform_fmt`, `terraform_validate`, `terraform_tflint` and
`terraform_trivy`, so that each folder of Terraform or OpenTofu code of the repository is
formatted, valid, passes tflint and has no misconfiguration of severity `HIGH` or `CRITICAL` that
trivy finds, before it is committed.

## Why

`terraform plan` is the first command that reads the code, and it needs credentials, the state and
a network. A reference to a value that does not exist, a variable that nothing reads or an S3
bucket without a public access block is found there, or after `apply`. The four hooks read the
code alone, in about two seconds for a repository whose tools and providers are downloaded.

## Exceptions

A repository with no `.tf` file has nothing to check: `external-config` skips the rule unless a
file matches its `applies-when` glob, `**/*.tf`.

## Tool

The hooks of [pre-commit-terraform](https://github.com/antonbabenko/pre-commit-terraform) run
each tool once for each folder with a changed file:

- `terraform_fmt` runs `terraform fmt` and rewrites the files, so a commit with unformatted code
  fails and leaves the files formatted.
- `terraform_validate` runs `terraform validate`, after `terraform init -backend=false` in a
  folder with no `.terraform`. The first run downloads the providers: the AWS provider is about
  250 MB, and its first `init` took 50 s.
- `terraform_tflint` runs `tflint --init` and `tflint` with the `.tflint.hcl` of the root of the
  repository.
- `terraform_trivy` runs `trivy config` with the checks built into its release, for Terraform only.

With `--hook-config=--tool-version=<version>` each hook downloads that release of its tool into
`~/.cache/pre-commit-terraform` the first time, so neither Terraform, tflint nor trivy has to be
installed. The hooks need bash, curl and unzip.

checkov is not a hook: the workflow `terraform.yml` of the recipe runs it in CI. It is not part of
this rule.

## Configuration

The block `# MARK: terraform` that the recipe `iac-terraform` adds to `prek.toml`, with the
default values of its parameters `terraform-version`, `tflint-version` and `trivy-version`
(`botica recipe install iac-terraform --set trivy-version=0.77.0` writes another one):

```toml
[[repos]]
repo = "https://github.com/antonbabenko/pre-commit-terraform"
rev = "v1.109.3"

[[repos.hooks]]
id = "terraform_fmt"
args = ["--hook-config=--tool-version=1.17.0"]

[[repos.hooks]]
id = "terraform_validate"
args = ["--hook-config=--tool-version=1.17.0"]

[[repos.hooks]]
id = "terraform_tflint"
args = [
  "--hook-config=--tool-version=0.65.0",
  "--args=--config=__GIT_WORKING_DIR__/.tflint.hcl",
]

[[repos.hooks]]
id = "terraform_trivy"
args = [
  "--hook-config=--tool-version=0.76.1",
  "--args=--skip-check-update",
  "--args=--misconfig-scanners=terraform",
  "--args=--severity=HIGH,CRITICAL",
  "--args=--ignorefile=__GIT_WORKING_DIR__/.trivyignore",
]
```

tflint and trivy run inside each folder and would read `.tflint.hcl` and `.trivyignore` from
there, so the hooks pass their paths from the root of the repository. A repository of OpenTofu adds `--hook-config=--tf-path=opentofu` to
`terraform_fmt` and `terraform_validate`, with a version of OpenTofu in `--tool-version`.
`external-config` checks the four hook ids, not their arguments.

## Do

```hcl
variable "reports_bucket" {
  type = string
}

resource "aws_s3_bucket" "reports" {
  bucket = var.reports_bucket
}

resource "aws_s3_bucket_public_access_block" "reports" {
  bucket                  = aws_s3_bucket.reports.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}
```

## Don't

```hcl
variable "reports_bucket" {
  type = string
}

variable "owner_team" {
  type = string
}

resource "aws_s3_bucket" "reports" {
    bucket = var.reports_bucket
}
```

`terraform fmt` rewrites the indentation of `bucket`, tflint reports `owner_team`
(`terraform_unused_declarations`), and trivy reports the bucket without a public access block
(AWS-0086, AWS-0087, AWS-0091, AWS-0093).

## Suppression

A finding of tflint: `# tflint-ignore: <rule>` on the line above. A finding of trivy:
`#trivy:ignore:<id>` on the line above the resource, such as `#trivy:ignore:AWS-0031`, with the
reason in the same comment block. A rule of tflint that does not apply to the repository is
turned off with a `rule` block in `.tflint.hcl`, and a check of trivy goes in `.trivyignore` with
its reason. The hooks are not removed to silence one finding.

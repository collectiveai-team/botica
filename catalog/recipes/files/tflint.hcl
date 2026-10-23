# CES-129. The hook `terraform_tflint` passes this file to each folder with `--config`.
# A provider has its own ruleset, such as `tflint-ruleset-aws`; add it as a `plugin` block with a
# `version`, and `tflint --init` downloads it.
# https://github.com/terraform-linters/tflint/blob/master/docs/user-guide/config.md
tflint {
  required_version = ">= 0.65"
}

config {
  call_module_type = "local"
}

plugin "terraform" {
  enabled = true
  preset  = "recommended"
}

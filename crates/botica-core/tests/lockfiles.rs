//! `lockfile-committed`: a manifest has the lockfile of one package manager, next to it or above.

use botica_core::lockfiles::{check, Source};
use botica_core::Severity;

fn reported(paths: &[&str], manifests: &[(&str, &str)]) -> Vec<String> {
    let paths: Vec<String> = paths.iter().map(|path| (*path).to_owned()).collect();
    let sources: Vec<Source<'_>> = manifests
        .iter()
        .map(|(path, text)| Source { path, text })
        .collect();
    check(&paths, &sources, Severity::Error)
        .into_iter()
        .map(|finding| finding.path)
        .collect()
}

const PROJECT: &str = "[project]\nname = \"a\"\n";

#[test]
fn a_lockfile_at_the_root_of_the_workspace_pins_its_members() {
    let paths = ["uv.lock", "pyproject.toml", "packages/a/pyproject.toml"];
    let manifests = [
        ("pyproject.toml", PROJECT),
        ("packages/a/pyproject.toml", PROJECT),
    ];
    assert!(reported(&paths, &manifests).is_empty());
}

#[test]
fn a_manifest_with_no_lockfile_above_it_is_reported() {
    let paths = ["packages/a/pyproject.toml", "packages/b/uv.lock"];
    let manifests = [("packages/a/pyproject.toml", PROJECT)];
    assert_eq!(reported(&paths, &manifests), ["packages/a/pyproject.toml"]);
}

#[test]
fn two_files_of_one_manager_are_not_two_managers() {
    let manifest = "{\"dependencies\": {\"a\": \"1\"}}";
    let paths = ["package.json", "bun.lock", "bun.lockb"];
    assert!(reported(&paths, &[("package.json", manifest)]).is_empty());
}

#[test]
fn a_manifest_that_pins_nothing_needs_no_lockfile() {
    let manifests = [
        ("go.mod", "module a\n\ngo 1.25\n// require nothing\n"),
        ("package.json", "{\"name\": \"a\", \"dependencies\": {}}"),
        ("pyproject.toml", "[tool.ruff]\nline-length = 100\n"),
    ];
    assert!(reported(&["go.mod", "package.json", "pyproject.toml"], &manifests).is_empty());
}

#[test]
fn the_package_manager_named_in_package_json_decides_the_lockfile() {
    let manifest =
        "{\"packageManager\": \"pnpm@10.18.0+sha512.abc\", \"devDependencies\": {\"a\": \"1\"}}";
    assert!(reported(
        &["package.json", "pnpm-lock.yaml"],
        &[("package.json", manifest)]
    )
    .is_empty());
    assert_eq!(
        reported(
            &["package.json", "package-lock.json"],
            &[("package.json", manifest)]
        ),
        ["package.json"]
    );
}

const ROOT: &str = "terraform {\n  required_providers {\n    aws = {\n      source  = \"hashicorp/aws\"\n      version = \"~> 6.0\"\n    }\n  }\n}\n\nprovider \"aws\" {\n  region = var.region\n}\n\nmodule \"network\" {\n  source = \"../../modules/network\"\n}\n";

const CHILD: &str = "terraform {\n  required_providers {\n    aws = {\n      source  = \"hashicorp/aws\"\n      version = \">= 6.0\"\n    }\n  }\n}\n";

#[test]
fn a_root_module_of_terraform_needs_its_lockfile_in_its_own_folder() {
    let manifests = [
        ("envs/prod/main.tf", ROOT),
        ("modules/network/main.tf", CHILD),
    ];
    assert_eq!(
        reported(
            &["envs/prod/main.tf", "modules/network/main.tf"],
            &manifests
        ),
        ["envs/prod/main.tf"]
    );
    assert_eq!(
        reported(
            &[
                ".terraform.lock.hcl",
                "envs/prod/main.tf",
                "modules/network/main.tf"
            ],
            &manifests
        ),
        ["envs/prod/main.tf"]
    );
    assert!(reported(
        &[
            "envs/prod/.terraform.lock.hcl",
            "envs/prod/main.tf",
            "modules/network/main.tf"
        ],
        &manifests
    )
    .is_empty());
}

#[test]
fn each_root_module_of_terraform_is_reported_once() {
    let backend = "terraform {\n  backend \"s3\" {\n    key = \"state\"\n  }\n}\n";
    let versions = "terraform {\n  required_providers {\n    aws = {\n      source = \"hashicorp/aws\"\n    }\n  }\n}\n";
    let manifests = [
        ("envs/dev/backend.tf", backend),
        ("envs/dev/versions.tf", versions),
        ("envs/prod/main.tf", ROOT),
        ("envs/prod/outputs.tf", "output \"id\" {\n  value = 1\n}\n"),
    ];
    let paths = manifests.map(|(path, _)| path);
    assert_eq!(
        reported(&paths, &manifests),
        ["envs/dev/backend.tf", "envs/prod/main.tf"]
    );
}

#[test]
fn a_module_with_no_provider_backend_or_cloud_block_needs_no_lockfile() {
    let manifests = [
        ("main.tf", CHILD),
        (
            "examples/basic/main.tf",
            "module \"this\" { source = \"../..\" }\n",
        ),
    ];
    assert!(reported(&["main.tf", "examples/basic/main.tf"], &manifests).is_empty());
}

#[test]
fn a_root_module_with_no_providers_needs_no_lockfile() {
    let manifests = [(
        "main.tf",
        "terraform {\n  cloud {\n    organization = \"o\"\n  }\n}\n\nresource \"terraform_data\" \"a\" {}\n",
    )];
    assert!(reported(&["main.tf"], &manifests).is_empty());
}

#[test]
fn a_module_that_a_local_source_calls_is_a_child_even_with_a_provider_block() {
    let legacy = "provider \"aws\" {\n  region = \"eu-west-1\"\n}\n";
    let root = "module \"legacy\" {\n  source = \"./legacy/\"\n}\n\nprovider \"aws\" {}\n";
    let manifests = [("main.tf", root), ("legacy/main.tf", legacy)];
    assert!(reported(
        &[".terraform.lock.hcl", "main.tf", "legacy/main.tf"],
        &manifests
    )
    .is_empty());
}

#[test]
fn blocks_in_strings_heredocs_and_comments_do_not_count() {
    let text = "# provider \"aws\" {}\nresource \"aws_iam_policy\" \"p\" {\n  policy = <<EOT\n{\nprovider \"x\" {\n}\nEOT\n  name = \"{\"\n}\n";
    assert!(reported(&["main.tf"], &[("main.tf", text)]).is_empty());
    let tofu = "provider \"aws\" {}\n";
    assert_eq!(
        reported(&["main.tofu"], &[("main.tofu", tofu)]),
        ["main.tofu"]
    );
}

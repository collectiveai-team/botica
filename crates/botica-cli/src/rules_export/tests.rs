use super::*;
use botica_catalog::SourceFile;

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.to_owned(),
        text: text.to_owned(),
    }
}

#[test]
fn a_pipe_in_a_summary_does_not_split_the_cell_of_the_index() {
    let rule = "id = \"log-no-print\"\ncode = \"BTLOG001\"\nsummary = \"a | b\"\n\
                status = \"stable\"\nscope = \"file\"\nneeds = \"syntax\"\n\
                run-on = [\"cli\"]\ndefault-enabled = true\ndefault-severity = \"error\"\n\n\
                [[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\n\
                file = \"python.yml\"\n";
    let catalog = Catalog::from_files(&[
        file(
            "logging/family.toml",
            "code = \"LOG\"\ndescription = \"d\"\n",
        ),
        file("logging/log-no-print/rule.toml", rule),
        file("logging/log-no-print/README.md", "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don\'t\n\ndont\n\n## Suppression\n\ns\n"),
        file("logging/log-no-print/python.yml", "id: log-no-print\n"),
    ])
    .expect("the catalog loads");

    let pages = render(&catalog);

    let index = &pages[0].text;
    assert!(index.contains("<br>a \\| b |  | stable |"), "{index}");
}

#[test]
fn the_row_of_a_rule_has_its_tags_under_its_summary_and_its_standard_in_a_cell() {
    let rule = "id = \"log-no-print\"\ncode = \"BTLOG001\"\nsummary = \"s\"\nstandard = \"CES-9\"\n\
                tags = [\"typing\", \"layout\"]\nstatus = \"stable\"\nscope = \"file\"\nneeds = \"syntax\"\n\
                run-on = [\"cli\"]\ndefault-enabled = true\ndefault-severity = \"error\"\n\n\
                [[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\n\
                file = \"python.yml\"\n";
    let catalog = Catalog::from_files(&[
        file(
            "logging/family.toml",
            "code = \"LOG\"\ndescription = \"d\"\n",
        ),
        file("logging/log-no-print/rule.toml", rule),
        file("logging/log-no-print/README.md", "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don\'t\n\ndont\n\n## Suppression\n\ns\n"),
        file("logging/log-no-print/python.yml", "id: log-no-print\n"),
    ])
    .expect("the catalog loads");

    let pages = render(&catalog);

    let index = &pages[0].text;
    assert!(
        index.contains("<br>s<br><small>typing, layout</small> | CES-9 | stable |"),
        "{index}"
    );
}

#[test]
fn the_rubric_looks_at_the_files_that_extend_exclude_leaves_in_the_walk() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/unit-tests/project-files");
    let _ = fs::remove_dir_all(&root);
    for file in ["src/a.py", "vendor/b.py"] {
        let path = root.join(file);
        fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
        fs::write(path, "x\n").expect("a file");
    }
    let catalog = Catalog::embedded().expect("the catalog");
    let config = botica_config::Config::from_botica_toml("extend-exclude = [\"vendor\"]\n")
        .expect("a valid configuration");
    let resolution = Arc::new(Resolution::new(&config, &catalog).expect("it resolves"));

    assert_eq!(project_files(&root, &resolution), ["src/a.py"]);
}

#[derive(clap::Parser)]
struct Command {
    #[command(flatten)]
    export: ExportArgs,
}

fn args(extra: &[&str]) -> ExportArgs {
    use clap::Parser;
    let base = ["export", "--format", "agents-md", "--output", "AGENTS.md"];
    Command::parse_from(base.iter().chain(extra)).export
}

#[test]
fn the_block_of_agents_md_links_each_rule_under_the_base_url() {
    use crate::agents_md::tests::{resolution_with, Judgment};
    let resolution = resolution_with(&[Judgment::new("testing", "test-one", "BTTEST901")], "");
    let text = "<!-- botica:rules -->\n<!-- /botica:rules -->\n";

    let updated = updated_agents_md(
        &args(&["--base-url", "https://x.org"]),
        Path::new("AGENTS.md"),
        &resolution,
        text,
    )
    .expect("the markers are there")
    .expect("the block changes");

    assert!(
        updated.contains(" → https://x.org/rules/testing/bttest901-test-one/\n"),
        "{updated}"
    );
}

mod common;

use common::{catalog, family, file, issues, rule_toml, NATIVE_ALL};

fn with_readme(dir: &str, readme: &str) -> Vec<botica_catalog::SourceFile> {
    let (slug, code) = if dir == "botica" {
        ("io-error", "BT001")
    } else {
        ("no-utils", "BTSTRUC001")
    };
    vec![
        family("structure", "STRUC"),
        file(
            &format!("{dir}/{slug}/rule.toml"),
            &rule_toml(slug, code, "", NATIVE_ALL),
        ),
        file(&format!("{dir}/{slug}/README.md"), readme),
    ]
}

fn readme(titles: &[&str]) -> String {
    titles
        .iter()
        .map(|title| format!("## {title}\n\nText.\n\n"))
        .collect()
}

#[test]
fn a_readme_with_every_section_of_the_template_in_its_order_loads() {
    let text = readme(&[
        "Directive",
        "Why",
        "Exceptions",
        "Do",
        "Don't",
        "Options",
        "Suppression",
    ]);

    assert!(catalog(vec![with_readme("structure", &text)]).is_ok());
}

#[test]
fn exceptions_and_options_may_be_left_out() {
    let text = readme(&["Directive", "Why", "Do", "Don't", "Suppression"]);

    assert!(catalog(vec![with_readme("structure", &text)]).is_ok());
}

#[test]
fn a_readme_without_a_required_section_is_rejected_with_the_name_of_the_section() {
    let text = readme(&["Directive", "Why", "Don't", "Suppression"]);

    let message = issues(vec![with_readme("structure", &text)]);

    assert!(
        message.contains("structure/no-utils/README.md: missing section `Do`"),
        "{message}"
    );
}

#[test]
fn a_section_that_the_template_does_not_name_is_rejected() {
    let text = readme(&[
        "Directive",
        "Why",
        "Which files",
        "Do",
        "Don't",
        "Suppression",
    ]);

    let message = issues(vec![with_readme("structure", &text)]);

    assert!(
        message.contains(
            "structure/no-utils/README.md: section `Which files` is not in the template: \
             Directive, Why, Exceptions, Do, Don't, Options, Suppression"
        ),
        "{message}"
    );
}

#[test]
fn a_section_out_of_the_order_of_the_template_is_rejected() {
    let text = readme(&["Directive", "Why", "Don't", "Do", "Suppression"]);

    let message = issues(vec![with_readme("structure", &text)]);

    assert!(
        message.contains(
            "structure/no-utils/README.md: section `Do` comes after `Don't`, and the template \
             puts it before"
        ),
        "{message}"
    );
}

#[test]
fn text_before_the_first_section_is_rejected() {
    let text = format!(
        "# no-utils\n\n{}",
        readme(&["Directive", "Why", "Do", "Don't", "Suppression"])
    );

    let message = issues(vec![with_readme("structure", &text)]);

    assert!(
        message.contains(
            "structure/no-utils/README.md: text before `## Directive`, the first section"
        ),
        "{message}"
    );
}

#[test]
fn a_rule_of_botica_itself_has_directive_why_and_suppression_only() {
    let own = readme(&["Directive", "Why", "Suppression"]);
    assert!(catalog(vec![with_readme("botica", &own)]).is_ok());

    let with_do = readme(&["Directive", "Why", "Do", "Suppression"]);
    let message = issues(vec![with_readme("botica", &with_do)]);
    assert!(
        message.contains(
            "botica/io-error/README.md: section `Do` is not in the template: \
             Directive, Why, Suppression"
        ),
        "{message}"
    );
}

#[test]
fn blank_lines_before_the_first_section_are_not_text() {
    let text = format!(
        "\n\n{}",
        readme(&["Directive", "Why", "Do", "Don't", "Suppression"])
    );

    assert!(catalog(vec![with_readme("structure", &text)]).is_ok());
}

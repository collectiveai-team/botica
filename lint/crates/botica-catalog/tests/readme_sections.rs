mod common;

use botica_catalog::Section;
use common::{catalog, family, file, rule_toml, NATIVE_ALL};

fn sections(readme: &str) -> Vec<Section> {
    let loaded = catalog(vec![
        vec![family("logging", "LOG")],
        vec![
            file(
                "logging/log-no-print/rule.toml",
                &rule_toml("log-no-print", "BTLOG001", "", NATIVE_ALL),
            ),
            file("logging/log-no-print/README.md", readme),
        ],
    ])
    .expect("the catalog loads");
    loaded.rules()[0].sections()
}

fn section(title: &str, text: &str) -> Section {
    Section {
        title: title.to_owned(),
        text: text.to_owned(),
    }
}

#[test]
fn a_readme_is_split_at_its_second_level_headings_with_the_text_of_each() {
    assert_eq!(
        sections("## Directive\n\nLibraries log.\n\n## Why\n\nA print is lost.\nTwice.\n"),
        [
            section("Directive", "Libraries log."),
            section("Why", "A print is lost.\nTwice."),
        ]
    );
}

#[test]
fn a_third_level_heading_stays_in_the_text_of_its_section() {
    assert_eq!(
        sections("## Do\n\n### Python\n\nlog it\n"),
        [section("Do", "### Python\n\nlog it")]
    );
}

#[test]
fn a_heading_inside_a_code_block_does_not_start_a_section() {
    assert_eq!(
        sections("## Suppression\n\n```toml\n## botica.toml\nignore = []\n```\n"),
        [section(
            "Suppression",
            "```toml\n## botica.toml\nignore = []\n```"
        )]
    );
}

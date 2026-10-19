mod common;

use botica_catalog::Section;
use common::{catalog, family, file, rule_toml, NATIVE_ALL};

fn sections(why: &str, r#do: &str, suppression: &str) -> Vec<Section> {
    let readme = format!(
        "## Directive\n\nLibraries log.\n\n## Why\n\n{why}\n\n## Do\n\n{do}\n\n## Don't\n\nprint\n\n\
         ## Suppression\n\n{suppression}\n",
        r#do = r#do,
    );
    let loaded = catalog(vec![
        vec![family("logging", "LOG")],
        vec![
            file(
                "logging/log-no-print/rule.toml",
                &rule_toml("log-no-print", "BTLOG001", "", NATIVE_ALL),
            ),
            file("logging/log-no-print/README.md", &readme),
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
        sections("A print is lost.\nTwice.", "log", "a comment"),
        [
            section("Directive", "Libraries log."),
            section("Why", "A print is lost.\nTwice."),
            section("Do", "log"),
            section("Don't", "print"),
            section("Suppression", "a comment"),
        ]
    );
}

#[test]
fn a_third_level_heading_stays_in_the_text_of_its_section() {
    assert_eq!(
        sections("w", "### Python\n\nlog it", "s")[2],
        section("Do", "### Python\n\nlog it")
    );
}

#[test]
fn a_heading_inside_a_code_block_does_not_start_a_section() {
    assert_eq!(
        sections("w", "d", "```toml\n## botica.toml\nignore = []\n```")[4],
        section("Suppression", "```toml\n## botica.toml\nignore = []\n```")
    );
}

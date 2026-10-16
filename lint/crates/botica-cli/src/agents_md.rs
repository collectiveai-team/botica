//! The block of `AGENTS.md` that lists the judgment rules of a project.

use std::collections::BTreeMap;

use botica_catalog::Rule;
use botica_config::Resolution;

use crate::rule_page::{file_name, group_of};

pub const START: &str = "<!-- botica:rules -->";
pub const END: &str = "<!-- /botica:rules -->";

/// The standards site, where each rule has its page.
pub const SITE: &str = "https://collectiveai-team.github.io/botica/";

/// The text between the markers: a `### <family>` per family with an active judgment rule, and
/// under it a line per rule, in the order of their codes (the order of the catalog).
pub fn block(resolution: &Resolution, base_url: &str) -> String {
    let mut families: BTreeMap<&str, Vec<&Rule>> = BTreeMap::new();
    for entry in resolution.entries() {
        if entry.active && entry.rule.is_judgment() {
            families
                .entry(group_of(&entry.rule))
                .or_default()
                .push(&entry.rule);
        }
    }
    let mut text = String::from("\n");
    for (family, rules) in families {
        text.push_str(&format!("### {family}\n\n"));
        for rule in rules {
            text.push_str(&format!(
                "- {} `{}`: {} → {}\n",
                rule.code,
                rule.id,
                first_sentence(rule),
                page_url(base_url, rule)
            ));
        }
        text.push('\n');
    }
    text
}

/// The first sentence of the Directive of a rule, on one line.
fn first_sentence(rule: &Rule) -> String {
    let directive = rule.directive();
    let paragraph = directive.split("\n\n").next().unwrap_or_default();
    match paragraph.find(". ") {
        Some(at) => paragraph[..=at].to_owned(),
        None => paragraph.to_owned(),
    }
}

fn page_url(base_url: &str, rule: &Rule) -> String {
    let page = file_name(rule);
    format!(
        "{}/rules/{}/{}/",
        base_url.trim_end_matches('/'),
        group_of(rule),
        page.trim_end_matches(".md")
    )
}

/// `text` with `block` between the markers, or why the markers cannot be found.
pub fn replace(text: &str, block: &str) -> Result<String, String> {
    let missing = || format!("needs a line `{START}` and, after it, a line `{END}`");
    let start = text.find(START).ok_or_else(missing)? + START.len();
    let end = start + text[start..].find(END).ok_or_else(missing)?;
    Ok(format!("{}{block}{}", &text[..start], &text[end..]))
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::Path;

    use super::*;
    use botica_catalog::{read_files, Catalog, SourceFile};
    use botica_config::Config;

    const README: &str = "## Directive\n\nTests assert behaviour\nthrough the public seam. They \
        never read a private helper.\n\nA second paragraph.\n\n## Why\n\nw\n\n## Flag\n\nf\n\n\
        ## Do not flag\n\nn\n\n## Confirm\n\nc\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n\
        ## Suppression\n\ns\n";

    /// A judgment rule for a catalog of a test: `extra` goes in the root of its `rule.toml`.
    pub(crate) struct Judgment {
        pub folder: &'static str,
        pub slug: &'static str,
        pub code: &'static str,
        pub enabled: bool,
        pub when: &'static str,
        pub languages: &'static str,
        pub extra: &'static str,
    }

    impl Judgment {
        pub(crate) const fn new(
            folder: &'static str,
            slug: &'static str,
            code: &'static str,
        ) -> Self {
            Self {
                folder,
                slug,
                code,
                enabled: true,
                when: "edit",
                languages: "\"python\"",
                extra: "",
            }
        }

        fn rule_toml(&self) -> String {
            format!(
                "id = \"{}\"\ncode = \"{}\"\nsummary = \"s\"\nstatus = \"stable\"\n\
                 default-enabled = {}\ndefault-severity = \"warning\"\n{}\n\n\
                 [[implementations]]\nkind = \"judgment\"\nlanguages = [{}]\n\
                 when = \"{}\"\nwhy-not-deterministic = \"w\"\n\n\
                 [implementations.question]\ntype = \"boolean\"\ninstructions = \"q\"\n",
                self.slug, self.code, self.enabled, self.extra, self.languages, self.when
            )
        }
    }

    /// The catalog of this checkout with `rules`, under `config`.
    pub(crate) fn resolution_with(rules: &[Judgment], config: &str) -> Resolution {
        let catalog_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rules");
        let mut files = read_files(&catalog_dir).expect("the catalog is read");
        for rule in rules {
            for (name, text) in [
                ("rule.toml", rule.rule_toml()),
                ("README.md", README.to_owned()),
            ] {
                files.push(SourceFile {
                    path: format!("{}/{}/{name}", rule.folder, rule.slug),
                    text,
                });
            }
        }
        let catalog = Catalog::from_files(&files).unwrap_or_else(|error| panic!("{error}"));
        let config = Config::from_botica_toml(config).expect("a valid configuration");
        Resolution::new(&config, &catalog).expect("it resolves")
    }

    /// Two active judgment rules in `testing`, given out of the order of their codes, an inactive
    /// one, and an active one in `architecture`.
    fn resolution(config: &str) -> Resolution {
        resolution_with(
            &[
                Judgment::new("testing", "test-two", "BTTEST902"),
                Judgment::new("testing", "test-one", "BTTEST901"),
                Judgment {
                    enabled: false,
                    ..Judgment::new("testing", "test-off", "BTTEST903")
                },
                Judgment::new("architecture", "arch-one", "BTARCH901"),
            ],
            config,
        )
    }

    const SENTENCE: &str = "Tests assert behaviour through the public seam.";

    #[test]
    fn the_block_lists_the_active_judgment_rules_by_family_in_the_order_of_their_codes() {
        let text = block(&resolution(""), "https://example.org/site/");

        assert_eq!(
            text,
            format!(
                "\n### architecture\n\n\
                 - BTARCH901 `arch-one`: {SENTENCE} → https://example.org/site/rules/architecture/btarch901-arch-one/\n\
                 \n### testing\n\n\
                 - BTTEST901 `test-one`: {SENTENCE} → https://example.org/site/rules/testing/bttest901-test-one/\n\
                 - BTTEST902 `test-two`: {SENTENCE} → https://example.org/site/rules/testing/bttest902-test-two/\n\n"
            )
        );
    }

    #[test]
    fn the_configuration_of_the_project_decides_which_rules_are_listed() {
        let text = block(
            &resolution("ignore = [\"test-one\", \"arch-one\"]\n"),
            "https://example.org",
        );

        assert_eq!(
            text,
            format!(
                "\n### testing\n\n\
                 - BTTEST902 `test-two`: {SENTENCE} → https://example.org/rules/testing/bttest902-test-two/\n\n"
            )
        );
    }

    #[test]
    fn with_no_active_judgment_rule_the_block_is_empty() {
        let text = block(
            &resolution("ignore = [\"test-one\", \"test-two\", \"arch-one\"]\n"),
            SITE,
        );

        assert_eq!(text, "\n");
    }

    #[test]
    fn the_block_goes_between_the_markers_and_the_rest_of_the_file_stays() {
        let text = format!("# Notes\n\nbefore\n{START}\nold line\n{END}\nafter\n");

        let replaced = replace(&text, "\nnew\n").expect("the markers are there");

        assert_eq!(
            replaced,
            format!("# Notes\n\nbefore\n{START}\nnew\n{END}\nafter\n")
        );
    }

    #[test]
    fn a_file_without_both_markers_in_order_is_an_error() {
        for text in [
            format!("{START}\nno end\n"),
            format!("no start\n{END}\n"),
            format!("{END}\nreversed\n{START}\n"),
            "nothing\n".to_owned(),
        ] {
            let error = replace(&text, "\n").expect_err("the markers are missing");

            assert_eq!(
                error,
                format!("needs a line `{START}` and, after it, a line `{END}`"),
                "{text}"
            );
        }
    }
}

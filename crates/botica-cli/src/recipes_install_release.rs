//! The release of botica in a file that `botica recipe install` writes. A template names the
//! release before the one that embeds it, since the release moves the versions after the binary
//! is built. Install writes the release of the binary that runs it instead: in the `rev` of the
//! mirror of botica, whose tag the mirror has once the binary is published, and in each
//! `botica@X.Y.Z` (as `uvx` runs it), so the file asks for what this release knows.

use std::borrow::Cow;

const MIRROR: &str = "repo = \"https://github.com/collectiveai-team/botica-pre-commit\"";
const PACKAGE: &str = "botica@";

/// `line` with each `botica@X.Y.Z` that no letter, digit, `-` or `/` precedes set to `version`.
fn pin_package(line: &str, version: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(at) = rest.find(PACKAGE) {
        let before = rest[..at].chars().next_back();
        let after = &rest[at + PACKAGE.len()..];
        let digits = after
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(after.len());
        let standalone = !before.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '/');
        out.push_str(&rest[..at + PACKAGE.len()]);
        if standalone && after[..digits].split('.').count() == 3 && digits > 0 {
            out.push_str(version);
            rest = &after[digits..];
        } else {
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// `text` with the `rev = "v…"` line after each line that names the mirror, and each
/// `botica@X.Y.Z`, set to this release.
pub fn at_this_release(text: &str) -> Cow<'_, str> {
    let version = env!("CARGO_PKG_VERSION");
    let rev = format!("rev = \"v{version}\"");
    let mut out = String::with_capacity(text.len());
    let mut after_mirror = false;
    for line in text.split_inclusive('\n') {
        let body = line.trim_start();
        if after_mirror && body.starts_with("rev = \"v") {
            let indent = &line[..line.len() - body.len()];
            let end = if line.ends_with('\n') { "\n" } else { "" };
            out.push_str(&format!("{indent}{rev}{end}"));
        } else {
            out.push_str(&pin_package(line, version));
        }
        after_mirror = body.trim_end() == MIRROR;
    }
    if out == text {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    #[test]
    fn the_rev_after_the_mirror_is_this_release_and_other_revs_stay() {
        let text = "[[repos]]\nrepo = \"https://github.com/collectiveai-team/botica-pre-commit\"\n\
                    rev = \"v0.0.1\"\n\n[[repos]]\nrepo = \"https://github.com/x/y\"\n\
                    rev = \"v9.9.9\"\n";

        let pinned = at_this_release(text);

        let expected = text.replace("rev = \"v0.0.1\"", &format!("rev = \"v{VERSION}\""));
        assert_eq!(pinned, expected);
        assert!(pinned.contains("rev = \"v9.9.9\""));
    }

    #[test]
    fn each_botica_at_a_version_is_this_release_and_other_packages_stay() {
        let text = "run: uvx botica@0.0.1 rules export --check\n\
                    run: npx @coldtea/abide@0.0.11 check && uvx my-botica@1.2.3 x\n";

        let pinned = at_this_release(text);

        assert_eq!(
            pinned,
            format!(
                "run: uvx botica@{VERSION} rules export --check\n\
                 run: npx @coldtea/abide@0.0.11 check && uvx my-botica@1.2.3 x\n"
            )
        );
    }

    #[test]
    fn a_text_without_the_mirror_or_the_package_is_returned_as_it_is() {
        let text = "repos = []\nbotica@latest\n";

        assert!(matches!(at_this_release(text), Cow::Borrowed(t) if t == text));
    }
}

//! The `rev` of the mirror of botica in a file that `botica recipe install` writes. A template
//! names the release before the one that embeds it, since the release moves the `rev` after the
//! binary is built; install writes the release of the binary that runs it, whose tag the mirror
//! has once the binary is published, so the file asks for the hooks that this release knows.

use std::borrow::Cow;

const MIRROR: &str = "repo = \"https://github.com/collectiveai-team/botica-pre-commit\"";

/// `text` with the `rev = "v…"` line after each line that names the mirror set to this release.
pub fn at_this_release(text: &str) -> Cow<'_, str> {
    let rev = format!("rev = \"v{}\"", env!("CARGO_PKG_VERSION"));
    let mut out = String::with_capacity(text.len());
    let mut after_mirror = false;
    let mut changed = false;
    for line in text.split_inclusive('\n') {
        let body = line.trim_start();
        if after_mirror && body.starts_with("rev = \"v") {
            let indent = &line[..line.len() - body.len()];
            let end = if line.ends_with('\n') { "\n" } else { "" };
            let new = format!("{indent}{rev}{end}");
            changed |= new != line;
            out.push_str(&new);
        } else {
            out.push_str(line);
        }
        after_mirror = body.trim_end() == MIRROR;
    }
    if changed {
        Cow::Owned(out)
    } else {
        Cow::Borrowed(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rev_after_the_mirror_is_this_release_and_other_revs_stay() {
        let text = "[[repos]]\nrepo = \"https://github.com/collectiveai-team/botica-pre-commit\"\n\
                    rev = \"v0.0.1\"\n\n[[repos]]\nrepo = \"https://github.com/x/y\"\n\
                    rev = \"v9.9.9\"\n";

        let pinned = at_this_release(text);

        let expected = text.replace(
            "rev = \"v0.0.1\"",
            &format!("rev = \"v{}\"", env!("CARGO_PKG_VERSION")),
        );
        assert_eq!(pinned, expected);
        assert!(pinned.contains("rev = \"v9.9.9\""));
    }

    #[test]
    fn a_text_without_the_mirror_is_returned_as_it_is() {
        let text = "repos = []\n";

        assert!(matches!(at_this_release(text), Cow::Borrowed(t) if t == text));
    }
}

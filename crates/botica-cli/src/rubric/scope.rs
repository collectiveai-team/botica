//! The `scope` of a rule in the rubric: positive globs only, since abide joins the globs of a
//! scope by "or" and a negated one widens the match.

use std::collections::BTreeSet;

/// The globs that cover `inside` and no other file of `files`, both from the root of the project.
/// `files` are the files of the languages of the rule; `extensions` are those of the languages,
/// none when the rule applies to every language. A directory whose files are all inside is one
/// glob per extension; any other directory lists its files that are inside one by one and looks
/// at its directories.
pub fn globs(files: &[String], inside: &BTreeSet<String>, extensions: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    cover("", files, inside, extensions, &mut found);
    found
}

fn cover(
    dir: &str,
    files: &[String],
    inside: &BTreeSet<String>,
    extensions: &[&str],
    found: &mut Vec<String>,
) {
    let under: Vec<&String> = files.iter().filter(|file| file.starts_with(dir)).collect();
    if !under.iter().any(|file| inside.contains(*file)) {
        return;
    }
    if under.iter().all(|file| inside.contains(*file)) {
        found.extend(directory_globs(dir, extensions));
        return;
    }
    let mut children = BTreeSet::new();
    for file in under {
        match file[dir.len()..].split_once('/') {
            Some((child, _)) => {
                children.insert(format!("{dir}{child}/"));
            }
            None if inside.contains(file) => found.push(file.clone()),
            None => {}
        }
    }
    for child in children {
        cover(&child, files, inside, extensions, found);
    }
}

fn directory_globs(dir: &str, extensions: &[&str]) -> Vec<String> {
    if extensions.is_empty() {
        return vec![format!("{dir}**")];
    }
    extensions
        .iter()
        .map(|extension| format!("{dir}**/*.{extension}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    fn scope(files: &[&str], inside: &[&str], extensions: &[&str]) -> Vec<String> {
        let inside: BTreeSet<String> = inside.iter().map(|item| (*item).to_owned()).collect();
        globs(&strings(files), &inside, extensions)
    }

    const FILES: [&str; 6] = [
        "app.py",
        "src/a.py",
        "src/pkg/b.py",
        "src/pkg/c.py",
        "tests/test_a.py",
        "tests/unit/test_b.py",
    ];

    #[test]
    fn a_project_whose_files_are_all_inside_is_one_glob_per_extension() {
        assert_eq!(
            scope(&FILES, &FILES, &["py", "pyi"]),
            ["**/*.py", "**/*.pyi"]
        );
    }

    #[test]
    fn a_directory_with_a_file_outside_lists_its_files_and_covers_its_directories_whole() {
        let inside = [
            "src/a.py",
            "src/pkg/b.py",
            "src/pkg/c.py",
            "tests/unit/test_b.py",
        ];

        assert_eq!(
            scope(&FILES, &inside, &["py"]),
            ["src/**/*.py", "tests/unit/**/*.py"]
        );
    }

    #[test]
    fn a_file_outside_in_a_directory_leaves_its_siblings_one_by_one() {
        let inside = [
            "app.py",
            "src/a.py",
            "src/pkg/c.py",
            "tests/test_a.py",
            "tests/unit/test_b.py",
        ];

        assert_eq!(
            scope(&FILES, &inside, &["py"]),
            ["app.py", "src/a.py", "src/pkg/c.py", "tests/**/*.py"]
        );
    }

    #[test]
    fn nothing_inside_is_no_glob() {
        assert!(scope(&FILES, &[], &["py"]).is_empty());
    }

    #[test]
    fn a_rule_of_every_language_covers_a_directory_with_every_file() {
        let inside = ["src/a.py", "src/pkg/b.py", "src/pkg/c.py"];

        assert_eq!(scope(&FILES, &inside, &[]), ["src/**"]);
    }

    #[test]
    fn a_directory_is_not_a_prefix_of_another_with_a_longer_name() {
        let files = ["src/a.py", "srcx/b.py"];

        assert_eq!(scope(&files, &["src/a.py"], &["py"]), ["src/**/*.py"]);
    }
}

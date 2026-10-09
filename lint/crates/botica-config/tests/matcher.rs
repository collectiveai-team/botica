use botica_config::PathMatcher;

fn matcher(patterns: &[&str]) -> PathMatcher {
    let owned: Vec<String> = patterns.iter().map(|p| (*p).to_owned()).collect();
    PathMatcher::new(&owned).unwrap_or_else(|error| panic!("{error:?}"))
}

#[test]
fn a_pattern_without_a_slash_matches_the_name_in_any_directory() {
    let m = matcher(&["*.lock", "package-lock.json", "*.min.js"]);
    for hit in [
        "Cargo.lock",
        "a/b/uv.lock",
        "package-lock.json",
        "web/package-lock.json",
        "dist/app.min.js",
    ] {
        assert!(m.is_match(hit), "{hit}");
    }
    for miss in ["lock.txt", "package-lock.json.bak", "app.js"] {
        assert!(!m.is_match(miss), "{miss}");
    }
}

#[test]
fn patterns_with_a_directory_are_anchored_like_gitignore() {
    let m = matcher(&["vendor/**", "tests/**"]);
    assert!(m.is_match("vendor/x/y.py"));
    assert!(m.is_match("tests/a.py"));
    assert!(!m.is_match("src/vendor/y.py"));
    assert!(!m.is_match("src/tests/a.py"));
}

#[test]
fn double_star_in_the_middle_matches_any_depth() {
    let m = matcher(&["**/fixtures/**/*.json"]);
    assert!(m.is_match("a/fixtures/b/c.json"));
    assert!(m.is_match("fixtures/c.json"));
    assert!(!m.is_match("a/fixtures/b/c.py"));
}

#[test]
fn a_directory_name_matches_what_is_inside_it() {
    let m = matcher(&["docs/"]);
    assert!(m.is_match("docs/a.md"));
    assert!(m.is_match("x/docs/a/b.md"));
}

#[test]
fn negation_does_not_turn_a_miss_into_a_hit() {
    let m = matcher(&["*.md", "!keep.md"]);
    assert!(m.is_match("a.md"));
    assert!(!m.is_match("keep.md"));
}

#[test]
fn an_empty_matcher_matches_nothing() {
    let m = matcher(&[]);
    assert!(m.is_empty());
    assert!(!m.is_match("anything"));
}

#[test]
fn invalid_patterns_are_reported_with_their_position() {
    let owned = vec![
        "ok/**".to_owned(),
        "[literal".to_owned(),
        "{a".to_owned(),
        "{b".to_owned(),
    ];
    let errors = PathMatcher::new(&owned)
        .expect_err("two patterns are invalid; an unclosed bracket is a literal, as in gitignore");
    let positions: Vec<usize> = errors.iter().map(|(index, _)| *index).collect();
    assert_eq!(positions, [2, 3]);
    assert!(PathMatcher::check("fine/*.rs").is_ok());
    assert!(PathMatcher::check("[x").is_ok());
    assert!(PathMatcher::check("{x").is_err());
}

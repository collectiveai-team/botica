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
fn a_walk_for_matches_enters_only_the_directories_an_anchored_pattern_can_reach() {
    let m = matcher(&["generated/api/*.py", "/build/out"]);
    for dir in [
        "",
        "generated",
        "generated/api",
        "generated/api/v2",
        "build",
        "build/out/x",
    ] {
        assert!(m.may_match_below(dir), "{dir}");
    }
    for dir in ["src", "generated/other", "other/generated", "builds"] {
        assert!(!m.may_match_below(dir), "{dir}");
    }
}

#[test]
fn a_pattern_that_can_match_at_any_depth_reaches_every_directory() {
    for pattern in ["*.gen.py", "docs/", "**/fixtures/x.json", "*/deep/x.py"] {
        let m = matcher(&[pattern]);
        for dir in ["", "src", "a/b/c"] {
            assert!(m.may_match_below(dir), "{pattern} in {dir}");
        }
    }
}

#[test]
fn a_matcher_with_no_pattern_that_selects_reaches_nothing() {
    for patterns in [&[][..], &["!keep/"], &["# a comment"]] {
        let m = matcher(patterns);
        assert!(!m.may_match_below(""), "{patterns:?}");
        assert!(!m.may_match_below("generated"), "{patterns:?}");
    }
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

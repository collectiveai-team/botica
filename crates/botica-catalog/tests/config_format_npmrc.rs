//! `.npmrc` is read as npm reads it: a comment may follow a value that is not quoted. In the INI
//! files of pytest the text after a value is part of the value.

use botica_catalog::Format;

#[test]
fn npm_cuts_a_value_that_is_not_quoted_at_its_first_comment() {
    let table = Format::of(".npmrc")
        .expect("a format")
        .parse("a=3 ; why\nb=\"x # y\"\nc=1#2\nd=a\\;b\n")
        .expect("parses");

    assert_eq!(table["a"].as_str(), Some("3"));
    assert_eq!(table["b"].as_str(), Some("x # y"));
    assert_eq!(table["c"].as_str(), Some("1"));
    assert_eq!(table["d"].as_str(), Some("a;b"));
}

#[test]
fn pytest_keeps_the_text_after_a_value() {
    let table = Format::of("pytest.ini")
        .expect("a format")
        .parse("[pytest]\naddopts = -q # c\n")
        .expect("parses");

    assert_eq!(table["pytest"]["addopts"].as_str(), Some("-q # c"));
}

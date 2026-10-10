use botica_astgrep::AstGrepSet;
use botica_core::suppress::Policy;
use botica_core::{Diagnostic, Document, Engine, NativeRule, Severity};

/// Reports every `FLAG` it finds, on the line where it starts.
struct Flag;

impl NativeRule for Flag {
    fn id(&self) -> &'static str {
        "flag-rule"
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let text = doc
            .text()
            .expect("the engine gives text to rules that need it");
        for (start, _) in text.match_indices("FLAG") {
            let (line, column) = doc.position(start);
            let (end_line, end_column) = doc.position(start + 4);
            out.push(Diagnostic {
                path: doc.path.to_owned(),
                rule: self.id().to_owned(),
                severity: Severity::Error,
                start,
                end: start + 4,
                line,
                column,
                end_line,
                end_column,
                file_level: false,
                message: "flagged".to_owned(),
            });
        }
    }
}

/// Reports once per file, with no line.
struct WholeFile;

impl NativeRule for WholeFile {
    fn id(&self) -> &'static str {
        "whole-file"
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let text = doc
            .text()
            .expect("the engine gives text to rules that need it");
        if text.contains("BIG") {
            out.push(Diagnostic {
                path: doc.path.to_owned(),
                rule: self.id().to_owned(),
                severity: Severity::Warning,
                start: 0,
                end: 0,
                line: 1,
                column: 1,
                end_line: 1,
                end_column: 1,
                file_level: true,
                message: "too big".to_owned(),
            });
        }
    }
}

/// Reports from `BEGIN` to the next `END`, which may be several lines apart.
struct Multi;

impl NativeRule for Multi {
    fn id(&self) -> &'static str {
        "multi-rule"
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let text = doc
            .text()
            .expect("the engine gives text to rules that need it");
        for (start, _) in text.match_indices("BEGIN") {
            let Some(length) = text[start..].find("END") else {
                continue;
            };
            let (line, column) = doc.position(start);
            let (end_line, end_column) = doc.position(start + length + 3);
            out.push(Diagnostic {
                path: doc.path.to_owned(),
                rule: self.id().to_owned(),
                severity: Severity::Error,
                start,
                end: start + length + 3,
                line,
                column,
                end_line,
                end_column,
                file_level: false,
                message: "spans lines".to_owned(),
            });
        }
    }
}

fn policy() -> Policy {
    Policy::new()
        .with_rule("flag-rule", &["BTFLG001", "old-flag"])
        .with_rule("whole-file", &["BTFLG002"])
        .with_rule("other-rule", &["BTFLG003"])
        .with_rule("multi-rule", &["BTFLG004"])
        .report_unused(Severity::Warning)
        .report_invalid(Severity::Warning)
}

fn engine(policy: Policy) -> Engine {
    Engine::new(
        vec![Box::new(Flag), Box::new(WholeFile), Box::new(Multi)],
        AstGrepSet::default(),
    )
    .with_suppressions(policy)
}

/// `rule@line` of every diagnostic, in order.
fn found(path: &str, text: &str) -> Vec<String> {
    summarize(&engine(policy()).lint_source(path, text))
}

fn summarize(found: &[Diagnostic]) -> Vec<String> {
    found
        .iter()
        .map(|d| format!("{}@{}", d.rule, d.line))
        .collect()
}

#[test]
fn without_a_comment_nothing_is_silenced() {
    assert_eq!(found("a.py", "x = FLAG\n"), ["flag-rule@1"]);
}

#[test]
fn a_botica_comment_on_the_same_line_silences_the_rule_by_slug() {
    assert!(found("a.py", "x = FLAG  # botica: ignore[flag-rule]\n").is_empty());
}

#[test]
fn the_code_and_an_alias_name_the_same_rule() {
    assert!(found("a.py", "x = FLAG  # botica: ignore[BTFLG001]\n").is_empty());
    assert!(found("a.py", "x = FLAG  # botica: ignore[old-flag]\n").is_empty());
}

#[test]
fn a_comment_on_another_line_silences_nothing() {
    let text = "# botica: ignore[flag-rule]\nx = FLAG\ny = 1  # botica: ignore[flag-rule]\n";
    let rules = found("a.py", text);
    assert!(rules.contains(&"flag-rule@2".to_owned()), "{rules:?}");
}

#[test]
fn only_the_named_rules_are_silenced() {
    let text = "x = FLAG  # botica: ignore[other-rule]\n";
    let rules = found("a.py", text);
    assert!(rules.contains(&"flag-rule@1".to_owned()), "{rules:?}");
}

#[test]
fn several_rules_can_be_named_with_commas_or_spaces() {
    let text = "x = FLAG BIG  # botica: ignore[other-rule, flag-rule]\n\
                y = FLAG  # botica: ignore[flag-rule other-rule]\n";
    let rules = found("a.py", text);
    assert!(
        !rules.iter().any(|r| r.starts_with("flag-rule")),
        "{rules:?}"
    );
}

#[test]
fn a_reason_after_two_dashes_is_ignored() {
    let text = "x = FLAG  # botica: ignore[flag-rule] -- legacy, see #12 and ignore[other]\n";
    assert!(found("a.py", text).is_empty());
    let empty = "x = FLAG  # botica: ignore[flag-rule] --\n";
    assert!(found("a.py", empty).is_empty());
}

#[test]
fn ignore_file_silences_the_rule_on_every_line() {
    let text = "# botica: ignore-file[flag-rule]\nx = FLAG\ny = FLAG\n";
    assert!(found("a.py", text).is_empty());
}

#[test]
fn ignore_file_works_from_any_line() {
    let text = "x = FLAG\ny = FLAG\n# botica: ignore-file[flag-rule]\n";
    assert!(found("a.py", text).is_empty());
}

#[test]
fn a_file_level_finding_is_silenced_only_by_ignore_file() {
    let line = "x = BIG  # botica: ignore[whole-file]\n";
    let rules = found("a.py", line);
    assert!(rules.contains(&"whole-file@1".to_owned()), "{rules:?}");
    let file = "x = BIG\n# botica: ignore-file[whole-file]\n";
    assert!(found("a.py", file).is_empty());
}

// --- noqa -----------------------------------------------------------------------------------

#[test]
fn noqa_with_a_botica_code_silences_the_rule() {
    assert!(found("a.py", "x = FLAG  # noqa: BTFLG001\n").is_empty());
    assert!(found("a.py", "x = FLAG  # NOQA: BTFLG001\n").is_empty());
}

#[test]
fn noqa_takes_only_codes_so_a_slug_does_nothing() {
    let rules = found("a.py", "x = FLAG  # noqa: flag-rule\n");
    assert!(rules.contains(&"flag-rule@1".to_owned()), "{rules:?}");
}

#[test]
fn a_bare_noqa_silences_nothing() {
    let rules = found("a.py", "x = FLAG  # noqa\n");
    assert_eq!(rules, ["flag-rule@1"], "and it is not botica's to report");
}

#[test]
fn codes_of_other_tools_in_the_same_noqa_are_skipped_without_a_warning() {
    let text = "x = FLAG  # noqa: E501, BTFLG001, F401\n";
    assert!(found("a.py", text).is_empty());
    let text = "x = 1  # noqa: E501, F401\n";
    assert!(found("a.py", text).is_empty());
}

#[test]
fn noqa_ends_its_codes_at_the_first_word_and_at_the_reason() {
    let text = "x = FLAG  # noqa: E501 legacy BTFLG001\n";
    let rules = found("a.py", text);
    assert!(rules.contains(&"flag-rule@1".to_owned()), "{rules:?}");
    let text = "x = FLAG  # noqa: BTFLG001 -- see BTFLG002 and E501\n";
    assert!(found("a.py", text).is_empty());
}

#[test]
fn a_directive_written_inside_a_reason_is_not_read() {
    let text = "x = FLAG  # noqa: BTFLG001 -- like noqa: BTFLG003\n";
    assert!(found("a.py", text).is_empty());
    let text = "x = FLAG  # botica: ignore[flag-rule] -- not botica: ignore[other-rule]\n";
    assert!(found("a.py", text).is_empty());
}

#[test]
fn noqa_exists_only_in_python() {
    let rules = found("a.sh", "x=FLAG  # noqa: BTFLG001\n");
    assert_eq!(rules, ["flag-rule@1"]);
    assert!(found("a.pyi", "x = FLAG  # noqa: BTFLG001\n").is_empty());
}

#[test]
fn a_noqa_after_another_comment_marker_still_counts() {
    assert!(found("a.py", "x = FLAG  # type: ignore # noqa: BTFLG001\n").is_empty());
}

// --- comments, not strings --------------------------------------------------------------------

#[test]
fn text_inside_a_string_is_not_a_comment() {
    let text = "s = 'FLAG # botica: ignore[flag-rule]'\n";
    assert_eq!(found("a.py", text), ["flag-rule@1"]);
    let text = "s = \"FLAG # noqa: BTFLG001\"\n";
    assert_eq!(found("a.py", text), ["flag-rule@1"]);
}

#[test]
fn text_inside_a_triple_quoted_string_is_not_a_comment() {
    let text = "s = \"\"\"\n# botica: ignore-file[flag-rule]\n\"\"\"\nx = FLAG\n";
    assert_eq!(found("a.py", text), ["flag-rule@4"]);
}

#[test]
fn an_escaped_quote_does_not_end_the_string() {
    let text = "s = 'it\\'s FLAG # botica: ignore[flag-rule]'\n";
    assert_eq!(found("a.py", text), ["flag-rule@1"]);
}

#[test]
fn a_comment_after_a_string_counts() {
    assert!(found("a.py", "s = 'a#b' + FLAG  # botica: ignore[flag-rule]\n").is_empty());
}

#[test]
fn prose_that_only_mentions_botica_is_left_alone() {
    let text = "x = FLAG  # see botica: the linter\n";
    assert_eq!(found("a.py", text), ["flag-rule@1"]);
    let text = "x = FLAG  # abotica: ignore[flag-rule]\n";
    assert_eq!(found("a.py", text), ["flag-rule@1"]);
}

// --- multi-line strings -------------------------------------------------------------------------

#[test]
fn a_finding_inside_a_multi_line_string_is_silenced_on_the_last_line_of_the_string() {
    let text = "s = \"\"\"\nFLAG\nend\"\"\"  # botica: ignore[flag-rule]\n";
    assert!(found("a.py", text).is_empty());
}

#[test]
fn a_comment_on_the_first_line_of_a_multi_line_string_does_not_silence_it() {
    let text = "s = \"\"\"  # botica: ignore[flag-rule]\nFLAG\nend\"\"\"\n";
    let rules = found("a.py", text);
    assert!(rules.contains(&"flag-rule@2".to_owned()), "{rules:?}");
}

#[test]
fn a_finding_after_a_multi_line_string_counts_on_its_own_line() {
    let text = "s = \"\"\"\nabc\n\"\"\"; y = FLAG  # botica: ignore[flag-rule]\n";
    assert!(found("a.py", text).is_empty());
}

// --- other languages ----------------------------------------------------------------------------

#[test]
fn javascript_comments_and_template_literals() {
    assert!(found("a.ts", "const x = FLAG; // botica: ignore[flag-rule]\n").is_empty());
    assert!(found("a.js", "const x = FLAG; /* botica: ignore[flag-rule] */\n").is_empty());
    let text = "const s = `${a} FLAG // botica: ignore[flag-rule]`;\n";
    assert_eq!(found("a.js", text), ["flag-rule@1"]);
    let text = "const s = `a ${ `b ${c}` } d`; const x = FLAG; // botica: ignore[flag-rule]\n";
    assert!(found("a.js", text).is_empty());
}

#[test]
fn a_javascript_regex_literal_may_hold_slashes_and_quotes() {
    let text = "const r = /['\"]\\/\\/ botica: ignore[flag-rule]/; const x = FLAG;\n";
    assert_eq!(found("a.js", text), ["flag-rule@1"]);
    let text = "const q = a / b; const x = FLAG; // botica: ignore[flag-rule]\n";
    assert!(found("a.js", text).is_empty());
}

#[test]
fn a_finding_inside_a_multi_line_template_is_silenced_on_its_last_line() {
    let text = "const s = `\nFLAG\n`; // botica: ignore[flag-rule]\n";
    assert!(found("a.js", text).is_empty());
}

#[test]
fn hash_comment_languages() {
    assert!(found("run.sh", "echo FLAG  # botica: ignore[flag-rule]\n").is_empty());
    assert!(found("ci.yaml", "key: FLAG  # botica: ignore[flag-rule]\n").is_empty());
    assert!(found("Dockerfile", "RUN FLAG  # botica: ignore[flag-rule]\n").is_empty());
    assert!(found(
        "pyproject.toml",
        "k = 'FLAG'  # botica: ignore[flag-rule]\n"
    )
    .is_empty());
}

#[test]
fn a_hash_without_a_blank_before_it_is_not_a_comment_in_shell() {
    let text = "echo ${#FLAG}x#botica: ignore[flag-rule]\n";
    assert_eq!(found("run.sh", text), ["flag-rule@1"]);
}

#[test]
fn markdown_and_html_comments() {
    assert!(found("a.md", "FLAG <!-- botica: ignore[flag-rule] -->\n").is_empty());
    assert!(found("a.html", "<p>FLAG</p> <!-- botica: ignore[flag-rule] -->\n").is_empty());
    assert!(found("a.md", "<!-- botica: ignore-file[flag-rule] -->\nFLAG\n").is_empty());
}

#[test]
fn languages_without_a_comment_scanner_are_not_read() {
    assert_eq!(
        found("a.json", "{\"a\": \"FLAG # botica: ignore[flag-rule]\"}\n"),
        ["flag-rule@1"]
    );
    assert_eq!(
        found("notes.txt", "FLAG # botica: ignore[flag-rule]\n"),
        ["flag-rule@1"]
    );
}

// --- unused-suppression ---------------------------------------------------------------------------

fn messages_of(rule: &str, path: &str, text: &str) -> Vec<String> {
    engine(policy())
        .lint_source(path, text)
        .into_iter()
        .filter(|d| d.rule == rule)
        .map(|d| format!("{}:{}:{}", d.line, d.column, d.message))
        .collect()
}

#[test]
fn a_suppression_that_silences_nothing_is_reported_on_its_directive() {
    let text = "x = 1  # botica: ignore[flag-rule]\n";
    let found = engine(policy()).lint_source("a.py", text);
    assert_eq!(summarize(&found), ["unused-suppression@1"]);
    let message = &found[0].message;
    assert!(message.contains("unused: `flag-rule`"), "{message}");
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(found[0].column, 10, "starts at `botica:`");
}

#[test]
fn only_the_unused_ids_of_a_directive_are_listed() {
    let text = "x = FLAG  # botica: ignore[flag-rule, other-rule]\n";
    let found = messages_of("unused-suppression", "a.py", text);
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("`other-rule`") && !found[0].contains("`flag-rule`"));
}

#[test]
fn a_known_rule_that_is_not_active_in_the_file_is_non_enabled() {
    let text = "x = 1  # botica: ignore[other-rule]\n";
    let found = messages_of("unused-suppression", "a.py", text);
    assert!(found[0].contains("non-enabled: `other-rule`"), "{found:?}");
}

#[test]
fn a_line_suppression_on_a_file_level_finding_suggests_ignore_file() {
    let text = "x = BIG  # botica: ignore[whole-file]\n";
    let found = messages_of("unused-suppression", "a.py", text);
    assert!(found[0].contains("ignore-file"), "{found:?}");
}

#[test]
fn unused_noqa_codes_are_reported_too() {
    let text = "x = 1  # noqa: E501, BTFLG001\n";
    let found = messages_of("unused-suppression", "a.py", text);
    assert!(
        found[0].contains("`BTFLG001`") && !found[0].contains("E501"),
        "{found:?}"
    );
}

#[test]
fn unused_suppression_can_silence_itself_in_the_same_comment() {
    let text = "x = 1  # botica: ignore[flag-rule, unused-suppression]\n";
    assert!(found("a.py", text).is_empty());
    let text = "x = 1  # noqa: BTFLG001, BT002\n";
    assert!(found("a.py", text).is_empty());
}

#[test]
fn unused_suppression_can_be_silenced_for_the_whole_file() {
    let text = "# botica: ignore-file[unused-suppression]\nx = 1  # botica: ignore[flag-rule]\n";
    assert!(found("a.py", text).is_empty());
}

#[test]
fn naming_unused_suppression_is_never_reported_as_unused() {
    assert!(found("a.py", "x = 1  # botica: ignore[unused-suppression]\n").is_empty());
    assert!(found("a.py", "x = 1  # botica: ignore[invalid-suppression]\n").is_empty());
}

#[test]
fn nothing_is_reported_when_the_rule_is_not_active() {
    let quiet = Policy::new().with_rule("flag-rule", &[]);
    let found = engine(quiet).lint_source("a.py", "x = 1  # botica: ignore[flag-rule]\n");
    assert!(found.is_empty(), "{found:?}");
}

// --- invalid-suppression --------------------------------------------------------------------------

#[test]
fn an_unknown_id_is_reported_on_that_id() {
    let text = "x = FLAG  # botica: ignore[flag-rule, no-such-rule]\n";
    let found = engine(policy()).lint_source("a.py", text);
    assert_eq!(summarize(&found), ["invalid-suppression@1"]);
    assert!(found[0].message.contains("`no-such-rule`"));
    assert_eq!(found[0].severity, Severity::Warning);
    assert_eq!(found[0].column, 39, "on the id, not on the comment");
}

#[test]
fn an_unknown_id_in_noqa_is_reported() {
    let text = "x = 1  # noqa: BTNOPE001\n";
    assert_eq!(
        summarize(&engine(policy()).lint_source("a.py", text)),
        ["invalid-suppression@1"]
    );
}

#[test]
fn a_family_or_a_prefix_is_not_an_id() {
    let text = "x = 1  # botica: ignore[BTFLG]\ny = 1  # noqa: BT001\n";
    let found = messages_of("invalid-suppression", "a.py", text);
    assert_eq!(found.len(), 2, "{found:?}");
}

#[test]
fn an_external_id_is_accepted_without_a_warning() {
    let policy = policy().with_external(|id| id.starts_with("acme-legacy/"));
    let text = "x = 1  # botica: ignore[acme-legacy/no-utils]\n";
    let found = engine(policy).lint_source("a.py", text);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn malformed_directives_are_reported() {
    for (text, expected) in [
        ("x = 1  # botica: ignore\n", "name the rules"),
        ("x = 1  # botica: ignore-file\n", "name the rules"),
        ("x = 1  # botica: ignore[]\n", "empty"),
        ("x = 1  # botica: ignore[ , ]\n", "empty"),
        ("x = 1  # botica: ignore[flag-rule\n", "never closed"),
        ("x = 1  # botica: ignroe[flag-rule]\n", "`ignroe`"),
        ("x = 1  # botica: disable[flag-rule]\n", "`disable`"),
    ] {
        let found = messages_of("invalid-suppression", "a.py", text);
        assert_eq!(found.len(), 1, "{text:?}: {found:?}");
        assert!(found[0].contains(expected), "{text:?}: {found:?}");
    }
}

#[test]
fn invalid_suppression_can_silence_itself() {
    let text = "x = 1  # botica: ignore[typo, invalid-suppression]\n";
    assert!(found("a.py", text).is_empty());
}

// --- compat-ast-grep-ignore --------------------------------------------------------------------------

#[test]
fn ast_grep_ignore_is_ignored_unless_compat_is_on() {
    let text = "x = FLAG  # ast-grep-ignore: flag-rule\n";
    assert_eq!(found("a.py", text), ["flag-rule@1"]);
}

#[test]
fn compat_honours_ast_grep_ignore_on_the_same_line_and_on_the_previous_one() {
    let on = || policy().with_compat_ast_grep_ignore(true);
    let same = "x = FLAG  # ast-grep-ignore: flag-rule\n";
    assert!(engine(on()).lint_source("a.py", same).is_empty());
    let before = "# ast-grep-ignore: other, flag-rule\nx = FLAG\n";
    assert!(engine(on()).lint_source("a.py", before).is_empty());
    let far = "# ast-grep-ignore: flag-rule\n\nx = FLAG\n";
    assert_eq!(
        summarize(&engine(on()).lint_source("a.py", far)),
        ["flag-rule@3"]
    );
    let bare = "x = FLAG  # ast-grep-ignore\n";
    assert_eq!(
        summarize(&engine(on()).lint_source("a.py", bare)),
        ["flag-rule@1"]
    );
}

#[test]
fn compat_follows_ast_grep_about_which_lines_count() {
    let on = || policy().with_compat_ast_grep_ignore(true);
    let lint = |text: &str| summarize(&engine(on()).lint_source("a.py", text));
    // a comment after code on the previous line covers only that line
    assert_eq!(
        lint("x = 1  # ast-grep-ignore: flag-rule\ny = FLAG\n"),
        ["flag-rule@2"]
    );
    // a finding that spans lines is covered from its first line and from its last line, not the middle
    assert!(lint("BEGIN  # ast-grep-ignore: multi-rule\n  END\n").is_empty());
    assert!(lint("BEGIN\n  END  # ast-grep-ignore: multi-rule\n").is_empty());
    assert_eq!(
        lint("BEGIN\n  # ast-grep-ignore: multi-rule\n  END\n"),
        ["multi-rule@1"]
    );
    assert!(lint("  # ast-grep-ignore: multi-rule\nBEGIN\n  END\n").is_empty());
}

#[test]
fn compat_directives_are_never_reported_as_unused_or_invalid() {
    let text = "x = 1  # ast-grep-ignore: flag-rule, whatever\n";
    let found = engine(policy().with_compat_ast_grep_ignore(true)).lint_source("a.py", text);
    assert!(found.is_empty(), "{found:?}");
}

// --- robustness ------------------------------------------------------------------------------------

/// xorshift: a fixed seed keeps the run reproducible without a dependency.
fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn no_input_makes_the_parser_panic() {
    const PIECES: [&str; 26] = [
        "# ",
        "botica:",
        " ignore",
        "-file",
        "[",
        "]",
        ", ",
        "flag-rule",
        "BTFLG001",
        "noqa",
        ": ",
        " -- ",
        "\n",
        "\"\"\"",
        "'",
        "\"",
        "`",
        "${",
        "}",
        "/*",
        "*/",
        "//",
        "<!--",
        "-->",
        "é",
        "\u{1F980}",
    ];
    const PATHS: [&str; 7] = [
        "a.py",
        "a.ts",
        "a.js",
        "a.sh",
        "a.md",
        "a.toml",
        "Dockerfile",
    ];
    let engine = engine(policy().with_compat_ast_grep_ignore(true));
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    for round in 0..4000 {
        let mut text = String::new();
        for _ in 0..(next(&mut state) % 14) {
            text.push_str(PIECES[(next(&mut state) % PIECES.len() as u64) as usize]);
            if next(&mut state).is_multiple_of(5) {
                text.push_str("FLAG ");
            }
        }
        let path = PATHS[round % PATHS.len()];
        let found = engine.lint_source(path, &text);
        for d in &found {
            assert!(
                d.start <= d.end && d.end <= text.len(),
                "{path} {text:?}: {d:?}"
            );
            assert!(text.is_char_boundary(d.start), "{path} {text:?}: {d:?}");
            assert!(d.line >= 1 && d.column >= 1);
        }
    }
}

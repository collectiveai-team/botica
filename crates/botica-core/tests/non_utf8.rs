use botica_astgrep::AstGrepSet;
use botica_core::{Diagnostic, Document, Engine, FileSizeGuard, NativeRule, Needs, Severity};

fn latin1(lines: usize) -> Vec<u8> {
    b"caf\xE9\n".repeat(lines)
}

#[test]
fn lines_are_counted_on_the_bytes_of_a_file_that_is_not_utf8() {
    let bytes = latin1(701);

    let doc = Document::from_bytes("notes.txt", &bytes);

    assert!(doc.text().is_none());
    assert_eq!(doc.line_count(), 701);
}

#[test]
fn the_last_line_counts_without_its_newline() {
    assert_eq!(Document::from_bytes("a.txt", b"a\nb").line_count(), 2);
    assert_eq!(Document::from_bytes("a.txt", b"a\nb\n").line_count(), 2);
    assert_eq!(Document::from_bytes("a.txt", b"").line_count(), 0);
}

#[test]
fn a_lone_carriage_return_is_not_a_line_break() {
    assert_eq!(Document::from_bytes("a.txt", b"a\rb\rc").line_count(), 1);
}

#[test]
fn bytes_that_are_utf8_give_their_text() {
    let doc = Document::from_bytes("a.txt", "caf\u{e9}\n".as_bytes());

    assert_eq!(doc.text(), Some("caf\u{e9}\n"));
    assert_eq!(doc.position(5), (1, 5), "a column counts characters");
}

#[test]
fn in_a_file_that_is_not_utf8_a_column_counts_bytes() {
    let doc = Document::from_bytes("a.txt", b"caf\xE9 = 1\n\xE9ab\n");

    assert_eq!(doc.position(4), (1, 5), "the space after the stray byte");
    assert_eq!(
        doc.position(10),
        (2, 2),
        "the `a` after the stray byte of line 2"
    );
}

/// Reports once per file, whatever the rule says about what it needs.
struct Sees {
    id: &'static str,
    needs: Needs,
}

impl NativeRule for Sees {
    fn id(&self) -> &'static str {
        self.id
    }

    fn needs(&self) -> Needs {
        self.needs
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        out.push(Diagnostic::file_level(
            doc.path,
            self.id,
            Severity::Warning,
            "seen".to_owned(),
        ));
    }
}

fn engine_of_each_need() -> Engine {
    let rules: Vec<Box<dyn NativeRule>> = vec![
        Box::new(Sees {
            id: "needs-text",
            needs: Needs::Text,
        }),
        Box::new(Sees {
            id: "needs-bytes",
            needs: Needs::Bytes,
        }),
        Box::new(Sees {
            id: "needs-path",
            needs: Needs::Path,
        }),
    ];
    Engine::new(rules, AstGrepSet::default())
}

fn rules_of(found: &[Diagnostic]) -> Vec<&str> {
    found.iter().map(|d| d.rule.as_str()).collect()
}

#[test]
fn a_file_that_is_not_utf8_is_not_given_to_the_rules_that_need_its_text() {
    let found = engine_of_each_need().lint_bytes("notes.txt", &latin1(1));

    assert_eq!(rules_of(&found), ["needs-bytes", "needs-path"]);
}

#[test]
fn a_utf8_file_is_given_to_every_rule() {
    let found = engine_of_each_need().lint_bytes("notes.txt", "caf\u{e9}\n".as_bytes());

    assert_eq!(
        rules_of(&found),
        ["needs-bytes", "needs-path", "needs-text"]
    );
}

#[test]
fn file_size_guard_counts_the_lines_of_a_latin1_file() {
    let rules: Vec<Box<dyn NativeRule>> = vec![Box::new(FileSizeGuard::default())];
    let engine = Engine::new(rules, AstGrepSet::default());

    let found = engine.lint_bytes("notes.txt", &latin1(701));

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rule, "file-size-guard");
    assert_eq!(found[0].severity, Severity::Error);
    assert!(
        found[0].message.contains("701 lines"),
        "{}",
        found[0].message
    );
    assert!(engine.lint_bytes("notes.txt", &latin1(399)).is_empty());
}

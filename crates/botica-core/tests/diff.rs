use botica_core::diff::{Changes, Mismatch};
use botica_core::{Diagnostic, Severity};

const CHANGED: &str = "\
diff --git a/src/a.py b/src/a.py
index 1111111..2222222 100644
--- a/src/a.py
+++ b/src/a.py
@@ -1,4 +1,5 @@
 import os
-x = 1
+x = 2
+y = 3
 z = 4
 w = 5
@@ -10,2 +11,2 @@ def f():
-    return 1
+    return 2
     pass
diff --git a/new.py b/new.py
new file mode 100644
index 0000000..3333333
--- /dev/null
+++ b/new.py
@@ -0,0 +1,2 @@
+a = 1
+b = 2
diff --git a/gone.py b/gone.py
deleted file mode 100644
index 4444444..0000000
--- a/gone.py
+++ /dev/null
@@ -1 +0,0 @@
-q = 1
";

fn finding(path: &str, line: usize, end_line: usize) -> Diagnostic {
    Diagnostic {
        path: path.to_owned(),
        rule: "r".to_owned(),
        severity: Severity::Error,
        start: 0,
        end: 0,
        line,
        column: 1,
        end_line,
        end_column: 1,
        file_level: false,
        measure: None,
        message: String::new(),
    }
}

#[test]
fn a_diff_lists_the_files_it_adds_or_changes_and_not_the_deleted_ones() {
    let changes = Changes::parse(CHANGED).expect("a diff");

    assert_eq!(changes.paths().collect::<Vec<_>>(), ["new.py", "src/a.py"]);
    assert!(changes.creates("new.py"));
    assert!(!changes.creates("src/a.py"));
    assert!(!changes.contains("gone.py"));
}

#[test]
fn a_finding_stays_when_its_range_touches_an_added_line() {
    let changes = Changes::parse(CHANGED).expect("a diff");

    assert!(changes.keeps(&finding("src/a.py", 2, 2)));
    assert!(changes.keeps(&finding("src/a.py", 3, 3)));
    assert!(changes.keeps(&finding("src/a.py", 1, 4)));
    assert!(changes.keeps(&finding("src/a.py", 11, 0)));
    assert!(!changes.keeps(&finding("src/a.py", 1, 1)));
    assert!(!changes.keeps(&finding("src/a.py", 4, 10)));
    assert!(!changes.keeps(&finding("other.py", 2, 2)));
}

#[test]
fn a_finding_about_a_whole_file_stays_when_the_file_changed_and_one_with_no_path_always() {
    let changes = Changes::parse(CHANGED).expect("a diff");
    let whole = |path: &str| Diagnostic::file_level(path, "r", Severity::Error, String::new());

    assert!(changes.keeps(&whole("src/a.py")));
    assert!(!changes.keeps(&whole("other.py")));
    assert!(changes.keeps(&whole("")));
}

#[test]
fn the_base_of_a_file_is_its_text_with_the_hunks_applied_in_reverse() {
    let changes = Changes::parse(CHANGED).expect("a diff");
    let after =
        "import os\nx = 2\ny = 3\nz = 4\nw = 5\n6\n7\n8\n9\ndef f():\n    return 2\n    pass\n";

    let base = changes.base("src/a.py", after).expect("the hunks apply");

    assert_eq!(
        base.as_deref(),
        Some("import os\nx = 1\nz = 4\nw = 5\n6\n7\n8\n9\ndef f():\n    return 1\n    pass\n")
    );
    assert_eq!(changes.base("new.py", "a = 1\nb = 2\n"), Ok(None));
    assert_eq!(changes.base("other.py", "x\n"), Ok(None));
}

#[test]
fn hunks_with_no_context_rebuild_removed_lines_and_a_missing_last_newline() {
    let patch = "\
--- a/c.toml
+++ b/c.toml
@@ -2,2 +1,0 @@
-b = 1
-c = 2
@@ -5 +3 @@
-e = 1
\\ No newline at end of file
+e = 2
";
    let changes = Changes::parse(patch).expect("a diff");

    let base = changes.base("c.toml", "a = 0\nd = 0\ne = 2\n");

    assert_eq!(
        base,
        Ok(Some("a = 0\nb = 1\nc = 2\nd = 0\ne = 1".to_owned()))
    );
    assert!(changes.keeps(&finding("c.toml", 3, 3)));
    assert!(!changes.keeps(&finding("c.toml", 1, 2)));
}

#[test]
fn a_diff_that_does_not_describe_the_file_is_a_mismatch() {
    let changes = Changes::parse(CHANGED).expect("a diff");

    let base = changes.base("src/a.py", "import os\nx = 9\ny = 3\nz = 4\nw = 5\n");

    assert_eq!(
        base,
        Err(Mismatch {
            path: "src/a.py".to_owned(),
            line: 2,
        })
    );
}

#[test]
fn lines_ending_in_a_carriage_return_keep_it() {
    let patch = "--- a/w.txt\n+++ b/w.txt\n@@ -1 +1 @@\n-a\r\n+b\r\n";
    let changes = Changes::parse(patch).expect("a diff");

    assert_eq!(changes.base("w.txt", "b\r\n"), Ok(Some("a\r\n".to_owned())));
}

#[test]
fn a_renamed_file_and_a_quoted_path_take_the_new_name() {
    let patch = "\
diff --git a/old.py b/new name.py
similarity index 100%
rename from old.py
rename to new name.py
diff --git \"a/caf\\303\\251.py\" \"b/tab\\there.py\"
--- \"a/tab\\there.py\"
+++ \"b/tab\\there.py\"
@@ -1 +1 @@
-a
+b
";
    let changes = Changes::parse(patch).expect("a diff");

    assert_eq!(
        changes.paths().collect::<Vec<_>>(),
        ["new name.py", "tab\there.py"]
    );
}

#[test]
fn a_file_added_by_hand_has_every_line_added() {
    let mut changes = Changes::default();
    changes.add_created("u.py", 3);

    assert!(changes.creates("u.py"));
    assert!(changes.keeps(&finding("u.py", 3, 3)));
    assert!(!changes.keeps(&finding("u.py", 4, 4)));
}

#[test]
fn a_header_that_is_not_one_of_a_hunk_is_malformed() {
    let error = Changes::parse("--- a/x\n+++ b/x\n@@ -a +1 @@\n").expect_err("malformed");

    assert_eq!(error.line, 3);
}

use super::js::{js_chunks, js_members};
use super::python::{python_chunks, python_members};
use super::{merge, pick};

fn starts(source: &str) -> Vec<&str> {
    python_chunks(source.as_bytes())
        .chunks
        .iter()
        .map(|c| source[c.start..c.end].lines().next().unwrap_or(""))
        .collect()
}

#[test]
fn splits_at_column_zero_statements() {
    let source = "import os\n\ndef f():\n    return 1\n\nx = 2\n";
    assert_eq!(starts(source), ["import os", "def f():", "x = 2"]);
}

#[test]
fn strings_brackets_and_continuations_do_not_split() {
    let source = "s = \"\"\"\nnot = a statement\n\"\"\"\ny = (\n1)\nz = 1 + \\\n2\nw = 'a#b'\n";
    assert_eq!(
        starts(source),
        ["s = \"\"\"", "y = (", "z = 1 + \\", "w = 'a#b'"]
    );
}

#[test]
fn decorators_and_clause_keywords_continue_the_statement() {
    let source = "@dec\n@other\ndef f():\n    pass\ntry:\n    x\nexcept E:\n    y\nfinally:\n    z\n# note\nif a:\n    b\nelse:\n    c\n";
    assert_eq!(starts(source), ["@dec", "try:", "if a:"]);
}

fn js_starts(source: &str) -> Vec<&str> {
    js_chunks(source.as_bytes())
        .chunks
        .iter()
        .map(|c| source[c.start..c.end].lines().next().unwrap_or(""))
        .collect()
}

#[test]
fn js_splits_before_statement_keywords_only() {
    let source = "import a from 'a'\nconst x =\nfoo()\nfunction f() {\nconst y = 1\n}\nfoo()\nexport default f\n";
    assert_eq!(
        js_starts(source),
        [
            "import a from 'a'",
            "const x =",
            "function f() {",
            "foo()",
            "export default f"
        ]
    );
}

#[test]
fn js_strings_templates_regexes_and_decorators_do_not_split() {
    let source = concat!(
        "const t = `a ${ {b: `\nconst inner = 1` } } c\nexport x`\n",
        "const r = /[/]const/g\n",
        "// const in a comment\n/* \nconst in a block */\n",
        "@Component({\n})\nexport class C {}\n",
        "let d = a / b\nlet e = 1\n",
    );
    assert_eq!(
        js_starts(source),
        [
            "const t = `a ${ {b: `",
            "const r = /[/]const/g",
            "@Component({",
            "let d = a / b",
            "let e = 1"
        ]
    );
}

#[test]
fn likely_syntax_errors_are_flagged() {
    let python = |s: &str| python_chunks(s.as_bytes()).suspect;
    assert!(!python("class A:\n    def f(self):\n        if x:  # c\n            pass\n    y = 1\nz = (\n  1)\n"));
    assert!(python("x = 1\n    y = 2\n")); // unexpected indent
    assert!(python("def f():\nreturn 1\n")); // expected an indented block
    assert!(python("if a:\n        b\n    c\n")); // dedent to no enclosing level
    assert!(python("f(a))\n")); // unmatched ')'
    assert!(python("x = (1,\n")); // never closed
    assert!(python("s = 'abc\n")); // unterminated string
    assert!(python("s = \"\"\"abc\n")); // unterminated triple quote

    let js = |s: &str| js_chunks(s.as_bytes()).suspect;
    assert!(!js(
        "const a = `x ${b} y`\nfunction f() { return /}/ }\nconst p = <p>Don't</p>\n"
    ));
    assert!(js("f(a))\n"));
    assert!(js("function f() {\n"));
    assert!(js("const t = `abc\n"));
    assert!(js("/* open\n"));
}

fn narrowed(source: &str, needle: &str) -> String {
    let text = source.as_bytes();
    let positions: Vec<usize> = source.match_indices(needle).map(|(i, _)| i).collect();
    let top = pick(&python_chunks(text).chunks, &positions);
    python_members(text, &top, &positions)
        .iter()
        .map(|c| &source[c.start..c.end])
        .collect()
}

#[test]
fn classes_and_functions_narrow_to_header_and_members_with_candidates() {
    let source = "import os\n\n@dec(\n    1,\n)\nclass A(\n    Base,\n):\n    '''doc'''\n\n    def f(self):\n        return 1\n\n    @prop\n    def g(self):\n        print(2)\n\n    x = 3\n\n\ndef h():\n    a = 1\n    print(3)\n";
    assert_eq!(
            narrowed(source, "print"),
            "@dec(\n    1,\n)\nclass A(\n    Base,\n):\n    @prop\n    def g(self):\n        print(2)\n\n    \
             def h():\n    print(3)\n"
                .replace("    def h", "def h")
        );
}

#[test]
fn a_candidate_in_the_header_keeps_the_whole_class() {
    let source =
        "class A(Base):\n    def f(self):\n        return 1\n    def g(self):\n        return 2\n";
    assert_eq!(narrowed(source, "Base"), source);
}

fn js_narrowed(source: &str, needle: &str) -> String {
    let text = source.as_bytes();
    let positions: Vec<usize> = source.match_indices(needle).map(|(i, _)| i).collect();
    let top = pick(&js_chunks(text).chunks, &positions);
    js_members(text, &top, &positions)
        .iter()
        .map(|c| &source[c.start..c.end])
        .collect()
}

#[test]
fn js_classes_and_functions_narrow_to_header_members_and_closing_brace() {
    let source = concat!(
        "import a from 'a'\n",
        "@Dec()\nexport class A extends B<{ x: number }> {\n",
        "  f() {\n    return 1\n  }\n\n",
        "  @Input()\n  g = console.log(2);\n\n",
        "  h() {\n    console.log(3)\n  }\n",
        "}\n",
        "function k() {\n  const a = 1;\n  if (a) {\n  }\n  else {\n    console.log(4);\n  }\n}\n",
    );
    assert_eq!(
        js_narrowed(source, "console"),
        concat!(
            "@Dec()\nexport class A extends B<{ x: number }> {\n",
            "  @Input()\n  g = console.log(2);\n\n",
            "  h() {\n    console.log(3)\n  }\n",
            "}\n",
            "function k() {\n  if (a) {\n  }\n  else {\n    console.log(4);\n  }\n}\n",
        )
    );
}

#[test]
fn js_keeps_whole_statements_it_cannot_narrow() {
    // A candidate in the header, an object literal, and members without semicolons.
    let header = "class A extends Base {\n  f() {}\n  g() {}\n}\n";
    assert_eq!(js_narrowed(header, "Base"), header);
    let object = "export const o = {\n  a: f(1),\n  b: 2,\n}\n";
    assert_eq!(js_narrowed(object, "f(1)"), object);
    let loose = "class A {\n  x = 1\n  y = f()\n}\n";
    assert_eq!(js_narrowed(loose, "f()"), loose);
}

#[test]
fn js_declarations_end_at_their_body_and_expressions_narrow_too() {
    let source = concat!(
        "class A {\n  f() {}\n  g() { log() }\n}\nexports.A = A;\n",
        "module.exports = cls => class B extends cls {\n  h() {}\n  i() { log() }\n};\n",
        "const k = () => {\n  a();\n  log();\n}\n",
    );
    assert_eq!(
        js_narrowed(source, "log"),
        concat!(
            "class A {\n  g() { log() }\n}\n",
            "module.exports = cls => class B extends cls {\n  i() { log() }\n};\n",
            "const k = () => {\n  log();\n}\n",
        )
    );
}

#[test]
fn picked_chunks_merge_when_adjacent() {
    let chunks = python_chunks(b"a = 1\nb = 2\nc = 3\nd = 4\n").chunks;
    let picked = merge(pick(&chunks, &[2, 7, 20, 21]));
    assert_eq!(
        picked
            .iter()
            .map(|c| (c.start, c.end, c.start_row, c.end_row))
            .collect::<Vec<_>>(),
        [(0, 12, 0, 2), (18, 24, 3, 4)]
    );
}

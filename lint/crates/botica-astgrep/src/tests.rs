use crate::{compiled, AstGrepRule, AstGrepSet, Tuning};
use ast_grep_language::SupportLang;
use std::path::Path;

#[test]
fn base_languages_are_compiled() {
    for language in [
        SupportLang::Python,
        SupportLang::TypeScript,
        SupportLang::Go,
    ] {
        assert!(compiled(language));
    }
}

#[test]
fn region_parsing_finds_the_same_hits_as_a_whole_file_parse() {
    let yaml = "id: p\nlanguage: python\nrule:\n  pattern: print($$$A)\n  not:\n    inside:\n      stopBy: end\n      pattern: 'if __name__ == \"__main__\": $$$B'\n";
    let source = "import os\n\nX = 1\n\ndef f():\n    print(1)\n\nclass C:\n    def g(self):\n        '''\nprint(2)\n'''\n        return 2\n\nY = [\nprint(3)]\n\nif __name__ == \"__main__\":\n    print(4)\n";
    let filler: String = (0..40)
        .map(|i| format!("def h{i}(a, b):\n    return a + b * {i}\n\n"))
        .collect();
    let source = format!("{filler}{source}");
    let source = source.as_str();
    let whole = AstGrepSet::with_tuning(
        AstGrepRule::from_yaml(yaml).unwrap(),
        Tuning {
            regions: false,
            ..Tuning::default()
        },
    );
    let regions = AstGrepSet::new(AstGrepRule::from_yaml(yaml).unwrap());
    let path = Path::new("m.py");
    let expected = whole.check(path, source);
    assert_eq!(expected.len(), 2);
    assert_eq!(regions.check(path, source), expected);
    assert_eq!(regions.stats().region_files, 1);
    assert!(regions.stats().parsed_bytes < source.len());
}

#[test]
fn an_inside_that_reads_its_ancestor_keeps_the_whole_class() {
    // Narrowed to `g`, the class would lose `x = 1`, the `inside` would fail and the `not`
    // would report the print.
    let yaml = "id: p\nlanguage: python\nrule:\n  pattern: print($A)\n  not:\n    inside:\n      stopBy: end\n      pattern: |\n        class $C:\n            x = 1\n            $$$B\n";
    let filler: String = (0..40)
        .map(|i| format!("def h{i}(a, b):\n    return a + b * {i}\n\n"))
        .collect();
    let source = format!(
            "{filler}class C:\n    x = 1\n\n    def f(self):\n        return 1\n\n    def g(self):\n        print(2)\n"
        );
    let whole = AstGrepSet::with_tuning(
        AstGrepRule::from_yaml(yaml).unwrap(),
        Tuning {
            regions: false,
            ..Tuning::default()
        },
    );
    let regions = AstGrepSet::new(AstGrepRule::from_yaml(yaml).unwrap());
    let path = Path::new("m.py");
    assert!(whole.check(path, &source).is_empty());
    assert!(regions.check(path, &source).is_empty());
    assert_eq!(regions.stats().region_files, 1);
    assert!(AstGrepRule::from_yaml(yaml).unwrap()[0].inspects_descendants);
    // An ancestor that is never narrowed (a top-level `if`) does not stop the narrowing.
    let main = "id: p\nlanguage: python\nrule:\n  pattern: print($A)\n  not:\n    inside:\n      stopBy: end\n      pattern: |\n        if __name__ == \"__main__\":\n            $$$B\n";
    assert!(!AstGrepRule::from_yaml(main).unwrap()[0].inspects_descendants);
}

#[test]
fn skipping_subtrees_finds_the_same_hits_as_matching_every_node() {
    let yaml = "id: p\nlanguage: python\nrule:\n  pattern: print($A)\n  not:\n    inside:\n      stopBy: end\n      pattern: 'if __name__ == \"__main__\": $$$B'\n---\nid: q\nlanguage: python\nrule:\n  kind: call\n  has:\n    field: function\n    regex: ^log$\n";
    let source = "def f(a):\n    return [x for x in a if print(x)]\n\nclass C:\n    def g(self):\n        log(print(2), print(3, 4))\n\nY = {'k': print(5)}\n\nif __name__ == \"__main__\":\n    print(6)\n";
    let every = AstGrepSet::with_tuning(
        AstGrepRule::from_yaml(yaml).unwrap(),
        Tuning {
            regions: false,
            skip_subtrees: false,
            ..Tuning::default()
        },
    );
    let skipping = AstGrepSet::with_tuning(
        AstGrepRule::from_yaml(yaml).unwrap(),
        Tuning {
            regions: false,
            ..Tuning::default()
        },
    );
    let path = Path::new("m.py");
    let expected = every.check(path, source);
    assert_eq!(expected.len(), 4);
    assert_eq!(skipping.check(path, source), expected);
    assert_eq!(skipping.stats().anchored_files, 1);
    assert_eq!(every.stats().anchored_files, 0);
}

#[test]
fn covers_every_extension_ast_grep_maps_to_a_rule_language() {
    let set = AstGrepSet::new(
        AstGrepRule::from_yaml("id: j\nlanguage: javascript\nrule:\n  pattern: debugger\n")
            .unwrap(),
    );
    for linted in ["a.js", "a.mjs", "a.cjs", "a.jsx"] {
        assert!(set.covers(Path::new(linted)), "{linted}");
    }
    for other in ["a.py", "a.ts", "a.json"] {
        assert!(!set.covers(Path::new(other)), "{other}");
    }
}

#[test]
fn malformed_yaml_is_an_error_not_a_hang() {
    let yaml = "id: x\nlanguage: python\nrule:\n  pattern: if a: b: c\n";
    assert!(AstGrepRule::from_yaml(yaml).is_err());
}

#[test]
fn rules_that_look_at_siblings_are_not_local() {
    let local =
        AstGrepRule::from_yaml("id: a\nlanguage: python\nrule:\n  pattern: print($A)\n").unwrap();
    assert!(local[0].local);
    let sibling = AstGrepRule::from_yaml(
            "id: b\nlanguage: python\nrule:\n  pattern: print($A)\n  follows:\n    pattern: import os\n",
        )
        .unwrap();
    assert!(!sibling[0].local);
}

#[cfg(not(feature = "lang-kotlin"))]
#[test]
fn rules_for_missing_grammars_fail_cleanly_and_their_files_are_skipped() {
    let error = AstGrepRule::from_yaml("id: k\nlanguage: kotlin\nrule:\n  pattern: println($A)\n")
        .err()
        .expect("kotlin is not compiled in");
    assert!(error
        .to_string()
        .contains("not compiled into this botica build"));
    let rules = AstGrepRule::from_yaml("id: p\nlanguage: python\nrule:\n  pattern: print($A)\n")
        .expect("python is base");
    let set = AstGrepSet::new(rules);
    assert!(set
        .check(Path::new("Main.kt"), "fun main() { println(1) }")
        .is_empty());
    assert_eq!(set.check(Path::new("m.py"), "print(1)\n").len(), 1);
}

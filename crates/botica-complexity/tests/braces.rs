//! TypeScript, Go and Rust: the examples of the paper of SonarSource, written in each language,
//! and the names and ranges of the functions.

use ast_grep_language::SupportLang;
use botica_complexity::{measure, Function};

fn values(language: SupportLang, source: &str) -> Vec<(String, u32)> {
    measure(language, source)
        .into_iter()
        .map(|function| (function.name, function.complexity))
        .collect()
}

fn value(language: SupportLang, source: &str) -> u32 {
    let found = values(language, source);
    assert_eq!(found.len(), 1, "{found:?}");
    found[0].1
}

#[test]
fn sum_of_primes_is_seven() {
    let typescript = "function sumOfPrimes(max: number): number {
  let total = 0;
  out: for (let i = 1; i <= max; ++i) {
    for (let j = 2; j < i; ++j) {
      if (i % j === 0) {
        continue out;
      }
    }
    total += i;
  }
  return total;
}
";
    let go = "package p

func sumOfPrimes(max int) int {
	total := 0
out:
	for i := 1; i <= max; i++ {
		for j := 2; j < i; j++ {
			if i%j == 0 {
				continue out
			}
		}
		total += i
	}
	return total
}
";
    let rust = "fn sum_of_primes(max: u32) -> u32 {
    let mut total = 0;
    'out: for i in 1..=max {
        for j in 2..i {
            if i % j == 0 {
                continue 'out;
            }
        }
        total += i;
    }
    total
}
";
    assert_eq!(value(SupportLang::TypeScript, typescript), 7);
    assert_eq!(value(SupportLang::Go, go), 7);
    assert_eq!(value(SupportLang::Rust, rust), 7);
}

#[test]
fn a_switch_and_its_cases_add_one() {
    let typescript = "function words(n: number): string {
  switch (n) {
    case 1: return 'one';
    case 2: return 'a couple';
    default: return 'lots';
  }
}
";
    let go = "package p

func words(n int) string {
	switch n {
	case 1:
		return \"one\"
	case 2:
		return \"a couple\"
	default:
		return \"lots\"
	}
}
";
    let rust = "fn words(n: u32) -> &'static str {
    match n {
        1 => \"one\",
        2 => \"a couple\",
        _ => \"lots\",
    }
}
";
    assert_eq!(value(SupportLang::TypeScript, typescript), 1);
    assert_eq!(value(SupportLang::Go, go), 1);
    assert_eq!(value(SupportLang::Rust, rust), 1);
}

#[test]
fn a_try_does_not_nest_and_a_catch_does() {
    let typescript = "function f() {
  try {
    if (a) {
      for (let i = 0; i < 10; i++) {
        while (b) { g(); }
      }
    }
  } catch (e) {
    if (b) { g(); }
  } finally {
    g();
  }
}
";
    assert_eq!(value(SupportLang::TypeScript, typescript), 9);
}

#[test]
fn a_nested_function_adds_its_nesting_to_the_one_that_holds_it() {
    let typescript = "function f() {\n  const r = () => {\n    if (a) { g(); }\n  };\n}\n";
    let go = "package p\n\nfunc f() {\n\tr := func() {\n\t\tif a {\n\t\t}\n\t}\n\tr()\n}\n";
    let rust = "fn f() {\n    let r = || {\n        if a {}\n    };\n    fn inner() {\n        if b {}\n    }\n}\n";
    assert_eq!(value(SupportLang::TypeScript, typescript), 2);
    assert_eq!(value(SupportLang::Go, go), 2);
    assert_eq!(value(SupportLang::Rust, rust), 4);
}

#[test]
fn each_sequence_of_one_boolean_operator_adds_one() {
    let typescript = "function f() {\n  if (a && b && c || d || e && f) { g(); }\n  return a && !(b && c) && (d ?? e);\n}\n";
    let go = "package p\n\nfunc f() bool {\n\tif a && b && c || d || e && f {\n\t}\n\treturn a && !(b && c)\n}\n";
    let rust = "fn f() -> bool {\n    if a && b && c || d || e && f {}\n    a && !(b && c)\n}\n";
    assert_eq!(value(SupportLang::TypeScript, typescript), 4 + 3);
    assert_eq!(value(SupportLang::Go, go), 4 + 2);
    assert_eq!(value(SupportLang::Rust, rust), 4 + 2);
}

#[test]
fn an_else_and_an_else_if_add_one_each_with_no_nesting() {
    let typescript =
        "function f() {\n  for (const x of xs) {\n    if (a) { g(); } else if (b) { g(); } else { g(); }\n  }\n}\n";
    let go = "package p\n\nfunc f() {\n\tfor _, x := range xs {\n\t\tif a {\n\t\t} else if b {\n\t\t} else {\n\t\t}\n\t}\n}\n";
    let rust = "fn f() {\n    for x in xs {\n        if a {} else if let Some(y) = b {} else {}\n    }\n}\n";
    assert_eq!(value(SupportLang::TypeScript, typescript), 1 + 2 + 1 + 1);
    assert_eq!(value(SupportLang::Go, go), 1 + 2 + 1 + 1);
    assert_eq!(value(SupportLang::Rust, rust), 1 + 2 + 1 + 1);
}

#[test]
fn a_ternary_adds_one_plus_its_nesting() {
    assert_eq!(
        value(
            SupportLang::TypeScript,
            "function f() {\n  return a ? (b ? 1 : 2) : 3;\n}\n"
        ),
        1 + 2
    );
}

#[test]
fn a_select_and_a_type_switch_add_one_plus_their_nesting() {
    let go = "package p\n\nfunc f() {\n\tfor {\n\t\tselect {\n\t\tcase <-c:\n\t\t\tswitch v.(type) {\n\t\t\tcase int:\n\t\t\t}\n\t\t}\n\t}\n}\n";
    assert_eq!(value(SupportLang::Go, go), 1 + 2 + 3);
}

#[test]
fn a_goto_and_a_labelled_break_add_one() {
    let go = "package p\n\nfunc f() {\nout:\n\tfor {\n\t\tbreak out\n\t}\n\tgoto out\n}\n";
    assert_eq!(value(SupportLang::Go, go), 1 + 1 + 1);
    let rust = "fn f() {\n    'out: loop {\n        break 'out;\n    }\n}\n";
    assert_eq!(value(SupportLang::Rust, rust), 1 + 1);
}

#[test]
fn a_direct_recursive_call_adds_one() {
    let typescript = "function fact(n: number): number {\n  return fact(n - 1);\n}\nclass C {\n  m(n: number) {\n    return this.m(n);\n  }\n}\n";
    let go = "package p\n\nfunc fact(n int) int {\n\treturn fact(n - 1)\n}\n\nfunc (s *S) M(n int) int {\n\treturn s.M(n)\n}\n";
    let rust = "fn fact(n: u32) -> u32 {\n    fact(n - 1)\n}\nimpl S {\n    fn m(&self) {\n        self.m();\n    }\n    fn n() {\n        Self::n();\n    }\n}\n";
    assert_eq!(
        values(SupportLang::TypeScript, typescript),
        [("fact".to_owned(), 1), ("C::m".to_owned(), 1)]
    );
    assert_eq!(
        values(SupportLang::Go, go),
        [("fact".to_owned(), 1), ("S::M".to_owned(), 1)]
    );
    assert_eq!(
        values(SupportLang::Rust, rust),
        [
            ("fact".to_owned(), 1),
            ("S::m".to_owned(), 1),
            ("S::n".to_owned(), 1)
        ]
    );
}

#[test]
fn the_question_mark_of_rust_adds_nothing() {
    assert_eq!(
        value(
            SupportLang::Rust,
            "fn f() -> Option<u32> {\n    let x = g()?;\n    Some(x)\n}\n"
        ),
        0
    );
}

fn names(language: SupportLang, source: &str) -> Vec<String> {
    measure(language, source)
        .into_iter()
        .map(|function| function.name)
        .collect()
}

#[test]
fn the_functions_of_a_typescript_module_are_its_functions_constants_and_methods() {
    let source = "export function a() {}
export const b = () => 1;
const c = function () {};
let notAFunction = 1;
export default function () {}
class K {
  m() {}
  handler = () => {};
  field = 1;
}
export abstract class L {
  n() {}
}
describe('x', () => {});
";
    assert_eq!(
        names(SupportLang::TypeScript, source),
        ["a", "b", "c", "default", "K::m", "K::handler", "L::n"]
    );
    assert_eq!(
        names(SupportLang::Tsx, "export const C = () => <div />;\n"),
        ["C"]
    );
}

#[test]
fn the_functions_of_a_rust_file_are_named_after_their_modules_impls_and_traits() {
    let source = "fn a() {}
impl<T> Store<T> {
    pub fn get(&self) {}
}
impl Display for &Item {
    fn fmt(&self) {}
}
trait Shape {
    fn area(&self) -> f64;
    fn twice(&self) -> f64 { 2.0 }
}
#[cfg(test)]
mod tests {
    #[test]
    fn works() {}
}
";
    assert_eq!(
        names(SupportLang::Rust, source),
        [
            "a",
            "Store::get",
            "Item::fmt",
            "Shape::twice",
            "tests::works"
        ]
    );
}

#[test]
fn a_finding_starts_at_the_function_and_ends_with_its_body() {
    let source = "#[test]\nfn works() {\n    if a {}\n}\n";
    let found: Vec<Function> = measure(SupportLang::Rust, source);
    assert_eq!(&source[found[0].start..found[0].start + 8], "fn works");
    assert_eq!(found[0].end, source.len() - 1);
}

#[test]
fn a_source_that_does_not_parse_has_no_functions() {
    assert!(measure(SupportLang::Rust, "fn f( {\n").is_empty());
    assert!(measure(SupportLang::Go, "package p\nfunc f( {\n").is_empty());
    assert!(measure(SupportLang::TypeScript, "function f( {\n").is_empty());
}

#[test]
fn a_language_that_is_not_measured_has_no_functions() {
    assert!(measure(SupportLang::Java, "class A { void f() { if (a) {} } }").is_empty());
}

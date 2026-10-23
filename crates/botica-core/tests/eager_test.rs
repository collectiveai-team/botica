//! `eager-test`: a test that calls more distinct functions of the code under test than the limit.

use botica_core::eager_test::EagerTest;
use botica_core::{Document, NativeRule};

fn reported(path: &str, text: &str, max: usize) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    EagerTest { max }.check(&Document::new(path, text), &mut out);
    out.into_iter()
        .map(|finding| (finding.line, finding.message))
        .collect()
}

const PYTHON: &str = "def test_flow():\n    a()\n    b()\n    a()\n    c()\n    assert d() == 1\n";

#[test]
fn a_name_counts_once_and_the_limit_is_inclusive() {
    assert!(reported("tests/test_x.py", PYTHON, 4).is_empty());
    let found = reported("tests/test_x.py", PYTHON, 3);
    assert_eq!(found.len(), 1);
    assert!(
        found[0].1.starts_with("`test_flow` calls 4 functions of the code under test (`a`, `b`, `c`, `d`; the limit is 3)"),
        "{}",
        found[0].1
    );
}

#[test]
fn assertions_builtins_and_classes_are_not_the_code_under_test() {
    let text = "class TestCart:\n    def test_add(self):\n        cart = Cart()\n        cart.add(1)\n\
                \x20       self.assertEqual(len(cart.items()), 1)\n        assert sorted([2, 1]) == [1, 2]\n";
    assert!(reported("tests/test_cart.py", text, 1).len() == 1);
    assert!(reported("tests/test_cart.py", text, 2).is_empty());
}

#[test]
fn a_factory_of_the_setup_counts_like_any_other_function() {
    let text = "def test_total():\n    user = make_user()\n    order = create_order(user)\n\
                \x20   cart = build_cart(order)\n    assert cart.total() == 1\n";
    let found = reported("tests/test_cart.py", text, 3);
    assert_eq!(found.len(), 1);
    assert!(
        found[0]
            .1
            .contains("(`build_cart`, `create_order`, `make_user`, `total`;"),
        "{}",
        found[0].1
    );
}

#[test]
fn a_python_function_that_is_not_a_test_is_not_counted() {
    let text = "def helper():\n    a()\n    b()\n";
    assert!(reported("tests/test_x.py", text, 1).is_empty());
}

#[test]
fn the_chain_of_expect_is_left_out_and_its_argument_counted() {
    let text = "test(\"t\", () => {\n  const c = make();\n  expect(c.total()).toBe(1);\n});\n";
    let found = reported("web/c.test.ts", text, 1);
    assert_eq!(found.len(), 1);
    assert!(found[0].1.contains("(`make`, `total`;"), "{}", found[0].1);
}

#[test]
fn a_go_test_leaves_out_its_testing_parameter() {
    let text = "package a\n\nfunc TestA(t *testing.T) {\n\tx := New()\n\tif x.Total() != 1 {\n\t\tt.Fatal(\"x\")\n\t}\n}\n";
    let found = reported("a_test.go", text, 1);
    assert!(found[0].1.contains("(`New`, `Total`;"), "{}", found[0].1);
}

#[test]
fn a_rust_test_is_a_function_with_a_test_attribute() {
    let text = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {\n        let c = Cart::new();\n        c.add(1);\n        c.pay();\n        assert_eq!(c.total(), 0);\n    }\n\n    fn helper() {\n        a();\n        b();\n    }\n}\n";
    let found = reported("src/cart.rs", text, 1);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, 4);
    assert!(found[0].1.contains("(`add`, `pay`;"), "{}", found[0].1);
}
